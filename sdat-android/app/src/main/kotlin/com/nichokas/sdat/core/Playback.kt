package com.nichokas.sdat.core

import android.content.Context
import android.net.Uri
import android.os.SystemClock
import androidx.annotation.OptIn
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.ForwardingSimpleBasePlayer
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.MimeTypes
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.nichokas.sdat.ui.TrackPlayer
import kotlin.coroutines.cancellation.CancellationException
import kotlin.math.abs
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Mix
import uniffi.tonearm_ffi.Quality
import uniffi.tonearm_ffi.Shuffle
import uniffi.tonearm_ffi.Stream
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.stream

/** The app's single player: the iOS Player's queues, the current track from the core's stream URL with the next one
 * preloaded behind it, scrobbling, and the queue saved across restarts. */
// ForwardingSimpleBasePlayer, which the session player is built on, is still marked unstable.
@OptIn(UnstableApi::class)
class Playback(context: Context, private val prefs: Preferences, secrets: Secrets) : TrackPlayer, LiveFollower {
    private val scope = MainScope()
    private val player = ExoPlayer.Builder(context)
        .setAudioAttributes(
            AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(),
            true,
        )
        .setHandleAudioBecomingNoisy(true)
        .build()
    private var request: Job? = null
    private val queue = PlayQueue<Track> { it.id }
    private var infinite: Shuffle? = null
    private var refilling: Deferred<Boolean>? = null
    private val scrobbler = Scrobbler(prefs, secrets)
    private var listen: Listen? = null
    /** The track whose stream is being fetched or already sits behind the current item; [preloaded] once it does. */
    private var preloading: Track? = null
    private var preloaded: Preloaded? = null
    private var preload: Job? = null
    private val saver = QueueSaver(context.filesDir, scope)
    private var restoring = true
    /** A Discover queue without its endless source, restored or played before the home mixes loaded, until
     * [resumeInfinite] gives it one. */
    private var awaitingMixes = false
    /** Home mixes that arrived while the saved queue was still loading. */
    private var pendingMixes: List<Mix>? = null
    /** Bumped whenever a queue entry starts, even the same track again: a refill that waited can tell a newer play
     * took over, and the player sheet drops a drag meant for the previous entry. */
    var entry by mutableIntStateOf(0)
        private set

    val track get() = queue.current
    override val current get() = track
    var playing by mutableStateOf(false)
        private set
    override val isPlaying get() = playing
    override val currentId get() = track?.id
    val userQueue get() = queue.userQueue
    val upNext get() = queue.upNext
    val shuffle get() = queue.shuffle
    val repeat get() = queue.repeat
    val source get() = queue.source
    /** Keeps the previous track's value until the new stream's real quality is known. */
    private var shownQuality by mutableStateOf<Quality?>(null)
    val badgeQuality get() = shownQuality ?: prefs.streamingQuality
    val streamingQuality get() = prefs.streamingQuality
    private var fetching by mutableStateOf(false)
    private var buffering by mutableStateOf(false)
    /** A Listen Together start waits for a buffer and for the host to reach the parked point. */
    private var aligning by mutableStateOf(false)
    override val loading get() = fetching || buffering || aligning
    var error by mutableStateOf<String?>(null)
        private set
    /** The current track plays its downloaded copy. */
    var offline by mutableStateOf(false)
        private set
    var position by mutableStateOf(0.0)
        private set
    var duration by mutableStateOf(0.0)
        private set
    val progress get() = progress(position, duration)
    private var ticker: Job? = null

    /** What the MediaSession drives: the player, with next/previous going through this queue (ExoPlayer holds only
     * the current track and the preloaded one). */
    val sessionPlayer: Player = object : ForwardingSimpleBasePlayer(player) {
        override fun getState(): State {
            val state = super.getState()
            if (state.playlist.isEmpty()) return state
            return state.buildUpon().setAvailableCommands(state.availableCommands.buildUpon().addAll(*queueCommands).build()).build()
        }

        override fun handleSeek(mediaItemIndex: Int, positionMs: Long, seekCommand: Int): ListenableFuture<*> {
            when (seekAction(seekCommand, mediaItemIndex, player.currentMediaItemIndex)) {
                Seek.Next -> Live.userNext()
                Seek.Previous -> if (!Live.guestRefuses()) previous()
                // Through [seek], so a seek while paused is kept and saved.
                Seek.InCurrent -> {
                    val secs = if (positionMs == C.TIME_UNSET) 0.0 else positionMs / 1000.0
                    if (!Live.userSeek(secs)) seek(secs)
                }
                Seek.Ignore -> {}
            }
            return Futures.immediateVoidFuture()
        }

        override fun handleSetPlayWhenReady(playWhenReady: Boolean): ListenableFuture<*> {
            if (!Live.isGuest) return super.handleSetPlayWhenReady(playWhenReady)
            if (playWhenReady) Live.userPlay() else Live.userPause()
            return Futures.immediateVoidFuture()
        }

        // The session prepares a stopped player when play is pressed. Stopped means the stream failed or a controller
        // stopped it; the item left in the player may still be the previous track's stream, so the current track
        // starts again where it was.
        override fun handlePrepare(): ListenableFuture<*> {
            if (!fetching) track?.let { start(it, play = player.playWhenReady, resumeAt = position) }
            return Futures.immediateVoidFuture()
        }

        // A stream still being fetched would prepare() and play after the stop; a pending refill starts its track
        // paused.
        override fun handleStop(): ListenableFuture<*> {
            request?.cancel()
            fetching = false
            // A guest's start may still be aligning; left set, it reads as loading and the follower never reloads.
            cancelLive()
            player.playWhenReady = false
            return super.handleStop()
        }
    }

    private val restore = scope.launch {
        val saved = saver.load()
        restoring = false
        // Anything played meanwhile wins, and is saved now that persisting is on.
        if (track != null) return@launch persist()
        if (saved == null) return@launch
        val (state, queued) = saved
        queue.restore(queued, state)
        awaitingMixes = queued?.endless == true
        duration = state.current.durationSecs.toDouble()
        position = maxOf(0.0, minOf(state.position, if (duration > 0) duration - 1 else state.position))
        pendingMixes?.let(::resumeInfinite)
        pendingMixes = null
    }

    init {
        player.addListener(object : Player.Listener {
            // The requested state, so the icon matches what a tap does even while audio focus suppresses output.
            override fun onEvents(player: Player, events: Player.Events) {
                syncPlaying()
                tick()
            }

            override fun onPlayWhenReadyChanged(playWhenReady: Boolean, reason: Int) {
                if (playWhenReady) systemPaused = false else if (reason in systemPauses) systemPaused = true
            }

            override fun onIsPlayingChanged(isPlaying: Boolean) {
                ticker?.cancel()
                // iOS's periodic time observer runs every 0.25 s.
                if (isPlaying) ticker = scope.launch { while (true) { tick(); delay(250) } }
                persist()
            }

            override fun onMediaItemTransition(item: MediaItem?, reason: Int) {
                val next = preloading
                val p = preloaded
                if (reason == Player.MEDIA_ITEM_TRANSITION_REASON_AUTO && next != null && p != null && item == p.item) adopt(next, p.source)
            }

            override fun onPlaybackStateChanged(state: Int) {
                buffering = state == Player.STATE_BUFFERING
                if (state == Player.STATE_READY) loadResumeAtMs = null
                // A Listen Together guest waits for the host's next track instead of advancing.
                if (state == Player.STATE_ENDED) if (liveFollowing) player.pause() else advance(auto = true)
            }

            override fun onPlayerError(e: PlaybackException) {
                val t = track
                // A downloaded copy that won't play falls back to the online stream, from where it stopped. A guest
                // still aligning has the player parked paused, so the online item aligns again like a fresh start.
                if (offline && t != null) return restart(t, player.currentPosition / 1000.0, online = true, play = player.playWhenReady || aligning)
                cancelLive()
                loadResumeAtMs = null
                error = e.errorCodeName
            }
        })
    }

    override fun play(tracks: List<Track>, startAt: Int, from: String?) {
        val t = queue.play(tracks, startAt, from) ?: return
        replaceInfinite(null)
        start(t)
    }

    override fun playInOrder(tracks: List<Track>, from: String?) {
        val t = queue.playInOrder(tracks, from) ?: return
        replaceInfinite(null)
        start(t)
    }

    /** Shuffle stays on for later lists, like iOS. */
    override fun playShuffled(tracks: List<Track>, from: String?) {
        val t = queue.playShuffled(tracks, from) ?: return
        replaceInfinite(null)
        start(t)
    }

    /** Tapping a Discover cover: that song, the rest of the visible pool, then the endless Discover shuffle. */
    fun playDiscover(t: Track, pool: List<Track>, mixes: List<Mix>) {
        queue.discover(t, pool.filter { it.streamable })
        replaceInfinite(mixes.takeIf { it.isNotEmpty() }?.let(::Shuffle))
        awaitingMixes = mixes.isEmpty()
        start(t)
    }

    /** Discover's shuffle: the endless queue alone. The current track plays on until its first batch arrives. */
    fun playInfinite(mixes: List<Mix>) {
        val source = Shuffle(mixes)
        replaceInfinite(source)
        // Held as the pending refill, so a refill for the old queue waits for it and a later replace frees the
        // source only after it returns.
        val job = scope.async {
            val batch = try {
                source.nextBatch(10u).filter { it.streamable }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                // A track still playing is left alone: an error would make the next tap a retry.
                if (infinite === source) friendlyMessage(e).let { if (track == null) error = it else Chrome.flash(it) }
                emptyList()
            }
            if (infinite !== source) return@async false
            val first = queue.startInfinite(batch)
            if (first == null) {
                // Nothing to play: the current queue goes on, and ends, without Discover's source. This job is
                // finishing, so the replace must not cancel it under whoever awaits it.
                if (refilling === coroutineContext.job) refilling = null
                replaceInfinite(null)
                return@async false
            }
            start(first)
            true
        }
        refilling = job
        job.invokeOnCompletion { if (refilling === job) refilling = null }
    }

    /** Inserts at [offset] in the user queue (Listen Together's Play Next run). */
    fun insertUpNext(track: Track, offset: Int) {
        if (this.track == null) return play(listOf(track))
        queue.insertUser(track, offset)
        changed()
    }

    override fun playNext(tracks: List<Track>) {
        if (tracks.isEmpty()) return
        // With nothing playing the list starts from its first track, not from its end.
        if (track == null) return play(tracks)
        queue.playNext(tracks)
        changed()
    }

    override fun addToQueue(tracks: List<Track>) {
        if (tracks.isEmpty()) return
        if (track == null) return play(tracks)
        queue.addToQueue(tracks)
        changed()
    }

    fun jumpToUpNext(offset: Int) {
        queue.jumpToUpNext(offset)?.let { start(it) }
    }

    fun jumpToUser(offset: Int) {
        queue.jumpToUser(offset)?.let { start(it) }
    }

    fun removeUpNext(offset: Int) {
        queue.removeUpNext(offset)
        changed()
    }

    fun removeUser(offset: Int) {
        queue.removeUser(offset)
        changed()
    }

    /** An endless queue keeps the source's order. */
    fun setShuffle(on: Boolean) {
        queue.setShuffle(on, reorder = !endless)
        changed()
    }

    fun cycleRepeat() {
        queue.cycleRepeat()
        changed()
    }

    /** Settings and the player's quality badge: the playing track switches at once and keeps its place (iOS
     * reloadQuality), and stays paused if it was. */
    fun setStreamingQuality(q: Quality) {
        if (q == prefs.streamingQuality) return
        prefs.streamingQuality = q
        val t = track ?: return
        restart(t, position)
    }

    /** The same play goes on from [at] with another stream, so it keeps what it counted towards its scrobble. */
    private fun restart(t: Track, at: Double, online: Boolean = false, play: Boolean = player.playWhenReady) {
        val heard = listen
        start(t, play = play, resumeAt = at, online = online)
        heard?.let { listen = it }
    }

    /** Settings: the preload follows the switch at once. */
    fun setPreloadNext(on: Boolean) {
        prefs.preloadNext = on
        syncPreload()
    }

    /** Gives a Discover queue its endless source once the home mixes load. */
    fun resumeInfinite(mixes: List<Mix>) {
        if (restoring) {
            pendingMixes = mixes
            return
        }
        if (!awaitingMixes || infinite != null || mixes.isEmpty()) return
        replaceInfinite(Shuffle(mixes))
        refillIfNeeded()
    }

    fun next() {
        if (track != null) advance(auto = false)
    }

    fun previous() {
        if (track == null) return
        val prev = if (position > 3) null else queue.previous()
        if (prev == null) seek(0.0) else start(prev)
    }

    /** Ignored while the stream URL loads: the new item starts from its beginning. */
    fun seek(secs: Double) {
        if (track == null || fetching) return
        val to = clampSeek(secs, duration)
        player.seekTo((to * 1000).toLong())
        position = to
        persist()
    }

    private fun advance(auto: Boolean, play: Boolean = true) {
        val t = queue.advance(manual = !auto)
        val p = preloaded
        if (t != null && t === preloading && p != null) {
            // A Next to the preloaded track plays the item already behind the current one.
            preloading = null
            preloaded = null
            player.seekTo(1, 0)
            player.removeMediaItem(0)
            player.playWhenReady = play
            if (player.playbackState == Player.STATE_IDLE) player.prepare()
            error = null
            began(t, p.source)
            return
        }
        if (auto && t != null && t === track && player.currentMediaItem?.mediaId == t.id) {
            // Repeat-one, or a one-track queue on repeat-all: the ended item plays again without a new fetch.
            player.seekToDefaultPosition()
            player.playWhenReady = play
            position = 0.0
            entry++
            listen = newListen(t)
            persist()
            started(t)
            return
        }
        if (t != null) {
            start(t, play)
            return
        }
        // Endless queue ran dry before the refill landed: fetch, then go on.
        val source = infinite ?: return stopAtEnd()
        val from = track
        val entryBefore = entry
        fetching = true
        // Like a skip to a queued track: the old one goes quiet, and a play or pause tapped while the refill loads
        // decides how the next one starts.
        if (!auto) {
            request?.cancel()
            player.stop()
            player.playWhenReady = true
        }
        scope.launch {
            val more = fetchMore(source)
            // A track picked during the fetch owns playback now; a source replaced meanwhile had nothing more.
            if (track !== from || entry != entryBefore) return@launch
            fetching = false
            // A Replay tapped during the fetch wins; that track's next end advances again.
            if (auto && player.playbackState != Player.STATE_ENDED) return@launch
            when {
                // A Play Next added during the fetch plays even when the source had nothing more.
                more || queue.userQueue.isNotEmpty() -> advance(auto, play = player.playWhenReady)
                auto -> stopAtEnd()
                // The skip found nothing; the player may hold a stale item, so the track is loaded again, paused.
                else -> from?.let { start(it, play = false) }
            }
        }
    }

    /** End of a finite queue: rewound and paused, and saved that way. Playing it again counts as a new listen. */
    private fun stopAtEnd() {
        player.pause()
        player.seekToDefaultPosition()
        position = 0.0
        listen = null
        persist()
    }

    private fun refillIfNeeded() {
        val source = infinite ?: return
        if (queue.needsRefill) scope.launch { fetchMore(source) }
    }

    /** Appends a new endless batch, skipping tracks already queued; one fetch at a time. */
    private suspend fun fetchMore(source: Shuffle): Boolean {
        if (infinite !== source) return false
        refilling?.let { return awaitRefill(it) }
        val job = scope.async {
            // The fresh Shuffle starts on the pages the Discover pool came from, so batches can be all
            // already-queued tracks for a while; only an empty batch means the source is exhausted.
            do {
                val batch = try { source.nextBatch(10u) } catch (e: CancellationException) { throw e } catch (e: Exception) { return@async false }
                if (infinite !== source || batch.isEmpty()) return@async false
            } while (!queue.append(batch.filter { it.streamable }))
            changed()
            true
        }
        refilling = job
        try {
            return awaitRefill(job)
        } finally {
            if (refilling === job) refilling = null
        }
    }

    /** A refill cancelled by a new source found nothing more for this caller. */
    private suspend fun awaitRefill(refill: Deferred<Boolean>) = try {
        refill.await()
    } catch (e: CancellationException) {
        currentCoroutineContext().ensureActive()
        false
    }

    private fun replaceInfinite(source: Shuffle?) {
        val old = infinite
        infinite = source
        awaitingMixes = false
        val pending = refilling
        pending?.cancel()
        refilling = null
        // A cancelled fetch may still be inside the core; free the source once it returns.
        old?.let { scope.launch { pending?.join(); it.close() } }
    }

    private fun tick() {
        // The player keeps the previous track's item until the new stream URL arrives, or if fetching it failed.
        if (fetching || player.currentMediaItem?.mediaId != track?.id) return
        val from = position
        position = player.currentPosition / 1000.0
        if (player.duration != C.TIME_UNSET && player.duration > 0) duration = player.duration / 1000.0
        listen?.let { if (it.heard(from, position, player.isPlaying)) scrobbler.scrobble(it) }
        if (player.isPlaying && saver.due) persist()
    }

    /** Plays the downloaded copy when there is one, unless [online]. */
    private fun start(t: Track, play: Boolean = true, resumeAt: Double = 0.0, online: Boolean = false) {
        entry++
        request?.cancel()
        cancelLive()
        liveSettledAt = null
        dropPreload()
        // Following a playing host: start only with a healthy buffer and on time, so nothing needs correcting after.
        val align = liveFollowing && play && liveTarget != null
        position = resumeAt
        loadResumeAtMs = (resumeAt * 1000).toLong()
        duration = t.durationSecs.toDouble()
        fetching = true
        listen = newListen(t)
        // Stopped, not cleared: an empty player drops the media notification and with it the foreground service,
        // which can't be started again while the app is in the background.
        player.stop()
        player.currentMediaItem?.let { player.replaceMediaItem(player.currentMediaItemIndex, it.buildUpon().setMediaMetadata(metadata(t)).build()) }
        error = null
        offline = false
        // Requested before the fetch so a pause tapped while the URL loads still holds after prepare().
        player.playWhenReady = play && !align
        aligning = align
        persist()
        request = scope.launch {
            try {
                val src = source(t, online)
                shownQuality = src.quality
                offline = src.offline
                player.setMediaItem(mediaItem(t, src), (resumeAt * 1000).toLong())
                player.prepare()
                fetching = false
                started(t)
                if (align) {
                    liveJob = scope.launch {
                        liveAlign()
                        aligning = false
                        liveSettledAt = uptime()
                    }
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                error = e.message ?: e.javaClass.simpleName
                fetching = false
                aligning = false
                loadResumeAtMs = null
            }
        }
    }

    /** A preloaded track the player moved on to by itself: it becomes the current one without a new fetch. */
    private fun adopt(t: Track, src: Source) {
        preloading = null
        preloaded = null
        player.removeMediaItem(0)
        val next = queue.advance(manual = false)
        // Preloading follows every queue change, so this is the track it fetched.
        if (next?.id != t.id) return next?.let { start(it) } ?: stopAtEnd()
        began(t, src)
    }

    /** The preloaded item became the current one. */
    private fun began(t: Track, src: Source) {
        entry++
        position = 0.0
        duration = t.durationSecs.toDouble()
        shownQuality = src.quality
        offline = src.offline
        listen = newListen(t)
        persist()
        started(t)
    }

    private fun newListen(t: Track) = Listen(t, System.currentTimeMillis() / 1000)

    /** iOS afterStart. */
    private fun started(t: Track) {
        scrobbler.nowPlaying(t)
        refillIfNeeded()
        syncPreload()
    }

    /** After a queue edit. */
    private fun changed() {
        persist()
        syncPreload()
    }

    /** Keeps the next track's stream queued behind the playing one, so a natural end flows straight into it (iOS
     * preloadNext). Idempotent: called after anything that may change what plays next. */
    private fun syncPreload() {
        // A guest plays only what the host announces, so an item behind the current one would advance on its own.
        val next = queue.preloadable?.takeIf { prefs.preloadNext && !liveFollowing && !fetching && player.currentMediaItem?.mediaId == track?.id }
        if (next != null && next.id == preloading?.id) return
        dropPreload()
        if (next == null) return
        preloading = next
        preload = scope.launch {
            val src = try {
                source(next)
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                // The next sync tries this track again.
                if (isActive) preloading = null
                return@launch
            }
            // Dropped while the stream call returned: the player no longer expects this item.
            if (!isActive) return@launch
            val item = mediaItem(next, src)
            player.addMediaItem(item)
            preloaded = Preloaded(item, src)
        }
    }

    private suspend fun source(t: Track, online: Boolean = false): Source {
        val local = if (online) null else Downloads.localStream(t.id)
        if (local != null) return Source(Uri.fromFile(local.playlist).toString(), local.quality, offline = true)
        val s = stream(t.id, prefs.streamingQuality)
        return Source(s.url, s.actualQuality, offline = false)
    }

    private fun dropPreload() {
        preload?.cancel()
        preload = null
        preloading = null
        preloaded = null
        if (player.mediaItemCount > 1) player.removeMediaItems(1, player.mediaItemCount)
    }

    private val endless get() = infinite != null || awaitingMixes

    private fun persist() {
        if (!restoring) saver.save(queue, endless, position)
    }

    /** After a failure or a controller's stop there is nothing prepared, so this starts the track again where it was;
     * a restored queue starts this way too. */
    fun toggle() {
        val t = track ?: return
        systemPaused = false
        val unprepared = error != null || !fetching && player.playbackState == Player.STATE_IDLE
        when (tapAction(unprepared, player.playbackState == Player.STATE_ENDED, player.playWhenReady)) {
            Tap.Retry -> start(t, resumeAt = position)
            Tap.Replay -> {
                // The ended item plays again with no fetch; a refill still loading gives way to it.
                fetching = false
                entry++
                listen = newListen(t)
                player.seekToDefaultPosition()
                player.play()
            }
            Tap.Pause -> player.pause()
            Tap.Resume -> {
                if (listen == null) listen = newListen(t)
                player.play()
            }
        }
    }

    /** Signing out: nothing is restored afterwards. */
    fun stop() {
        restore.cancel()
        restoring = false
        request?.cancel()
        cancelLive()
        dropPreload()
        player.pause()
        player.stop()
        player.clearMediaItems()
        position = 0.0
        duration = 0.0
        queue.clear()
        replaceInfinite(null)
        error = null
        offline = false
        fetching = false
        listen = null
        persist()
    }

    // Listen Together. The host just plays normally and publishes [livePublishedPositionMs]; a guest follows.

    /** Following a host: the host decides what plays next, so a track's end waits for it instead of advancing. */
    var liveFollowing = false
        set(on) {
            field = on
            syncPreload()
        }
    /** Where the host is for the track loaded here; set by [Live]. */
    var liveTarget: (() -> HostAt?)? = null
    override var liveSeeking = false
        private set
    override var liveSettledAt: Long? = null
        private set
    override var systemPaused = false
        private set
    private var liveJob: Job? = null
    /** [liveJob] is a resume that has not started playing yet ([playing] already shows it). */
    private var liveResumePending = false
    /** Set while [start] loads a new item: the engine still reports the previous item's position meanwhile. */
    private var loadResumeAtMs: Long? = null

    /** Joining a session is a deliberate choice to listen: forget an older system pause. */
    fun clearSystemPause() { systemPaused = false }

    /** The position a host publishes: a track still loading is published at its start, not at the old item's time. */
    val livePublishedPositionMs get() = loadResumeAtMs ?: player.currentPosition
    val livePublishedPlaying get() = playing && loadResumeAtMs == null

    override val livePositionMs get() = player.currentPosition
    override val liveHasItem get() = player.playbackState != Player.STATE_ENDED && (fetching || player.playbackState != Player.STATE_IDLE)
    override val liveAdvancing get() = player.isPlaying

    override fun livePlay(track: Track, atMs: Long, playing: Boolean) {
        queue.play(listOf(track), 0, "Listen Together")
        replaceInfinite(null)
        start(track, playing, resumeAt = maxOf(atMs, 0) / 1000.0)
    }

    override fun pause() = player.pause()

    override fun seekTo(ms: Long) = seek(ms / 1000.0)

    private fun syncPlaying() {
        playing = player.playWhenReady && player.playbackState != Player.STATE_ENDED || liveResumePending
    }

    private fun cancelLive() {
        liveJob?.cancel()
        liveJob = null
        liveSeeking = false
        liveResumePending = false
        aligning = false
    }

    /** The session ended: a resume or start still waiting for the host never happens, while a drift seek just lands. */
    fun liveEnded() {
        liveFollowing = false
        liveTarget = null
        val waiting = liveResumePending || aligning
        cancelLive()
        if (waiting) player.pause()
        syncPlaying()
    }

    /**
     * Large drift correction: a plain seek to where the host is, while playing. The destination is re-read when the
     * seek starts, and a slow seek that still lands behind is followed by one more aiming ahead by the time the first
     * took (capped), so it does not leave an error that triggers another seek. The speed is never changed.
     */
    override fun liveSeek(ms: Long) {
        if (liveSeeking || track == null) return
        liveSeeking = true
        liveJob = scope.launch {
            var lead = 0L
            for (attempt in 0 until 2) {
                val dest = maxOf((liveTarget?.invoke()?.ms ?: ms) + lead, 0)
                val started = uptime()
                player.seekTo(dest)
                position = dest / 1000.0
                awaitPlaying()
                val host = liveTarget?.invoke()
                if (host == null || !host.playing || host.ms - player.currentPosition <= LATE_TOLERANCE_MS) break
                lead = minOf(uptime() - started, MAX_SEEK_LEAD_MS)
            }
            liveSeeking = false
            liveSettledAt = uptime()
        }
    }

    /** Waits until audio advances again after a seek, bounded so a stall cannot hold corrections forever. */
    private suspend fun awaitPlaying() {
        val deadline = uptime() + BUFFER_TIMEOUT_MS
        delay(20)
        while (!player.isPlaying && player.playWhenReady && uptime() < deadline) delay(20)
    }

    /**
     * The host resumed after a pause: start in step with it. Too far from it (more than the buffer kept while paused
     * covers) it joins like a fresh start instead: playing from the old spot would be the wrong part of the song.
     */
    override fun liveResume() {
        if (playing) return
        val host = liveTarget?.invoke()
        val unprepared = error != null || player.playbackState == Player.STATE_IDLE || player.playbackState == Player.STATE_ENDED
        if (host == null || !host.playing || unprepared) {
            toggle()
            return
        }
        val far = abs(player.currentPosition - host.ms) > RESUME_WINDOW_MS
        systemPaused = false
        liveResumePending = true
        liveSeeking = true
        syncPlaying()
        liveJob = scope.launch {
            if (far) liveAlign() else startInSync()
            liveSeeking = false
            liveResumePending = false
            liveSettledAt = uptime()
            syncPlaying()
        }
    }

    /**
     * Joins the host's playback without audible corrections: parks the item paused a little ahead of the host, waits
     * for a buffer there, then plays when the host reaches that point. Slower to join, but nothing needs correcting
     * right after. Leaves the player paused at the host position when the host pauses meanwhile.
     */
    private suspend fun liveAlign(): Boolean {
        repeat(ALIGN_ATTEMPTS) attempt@{
            player.pause()
            val t = liveTarget?.invoke() ?: return false
            if (!t.playing) {
                player.seekTo(t.ms)
                return false
            }
            val duration = player.duration
            val startAt = (t.ms + JOIN_LEAD_MS).let { if (duration > 0) minOf(it, maxOf(duration - 1_000, t.ms)) else it }
            player.seekTo(startAt)
            awaitBuffer(startAt)
            // The host stopped advancing (buffering) or jumped back: park again near its new position.
            val deadline = uptime() + JOIN_LEAD_MS + BUFFER_TIMEOUT_MS
            while (true) {
                val now = liveTarget?.invoke() ?: return false
                if (!now.playing || now.ms >= startAt) break
                if (uptime() > deadline || startAt - now.ms > JOIN_LEAD_MS + LATE_TOLERANCE_MS) return@attempt
                delay(minOf(50, startAt - now.ms))
            }
            val now = liveTarget?.invoke() ?: return false
            if (!now.playing) {
                player.seekTo(now.ms)
                return false
            }
            if (now.ms - startAt > LATE_TOLERANCE_MS) return@attempt
            if (systemPaused) return false
            player.play()
            return true
        }
        // Very slow network: start where the host is and let the player buffer.
        val t = liveTarget?.invoke() ?: return false
        player.seekTo(t.ms)
        if (!t.playing || systemPaused) return false
        player.play()
        return true
    }

    /** Plays once the host reaches the paused position; a host already past it is followed at once (within tolerance). */
    private suspend fun startInSync() {
        val host = liveTarget?.invoke() ?: return
        val startAt = uptime() + player.currentPosition - host.ms
        while (true) {
            val now = liveTarget?.invoke()
            if (now == null || !now.playing || systemPaused) return
            val left = startAt - uptime()
            if (left <= 0) break
            delay(minOf(50, left))
        }
        player.play()
    }

    /** Waits until [MIN_BUFFER_MS] is loaded after [fromMs] (or up to the end of the track), bounded. */
    private suspend fun awaitBuffer(fromMs: Long) {
        val deadline = uptime() + BUFFER_TIMEOUT_MS
        while (uptime() < deadline && player.playerError == null) {
            val duration = player.duration
            val need = if (duration > 0) minOf(MIN_BUFFER_MS, maxOf(duration - fromMs - 500, 0)) else MIN_BUFFER_MS
            if (player.playbackState == Player.STATE_READY && player.bufferedPosition - fromMs >= need) return
            delay(100)
        }
    }
}


/** Pauses the system makes (headphones unplugged, audio focus lost); Listen Together must not undo them on its own. */
private val systemPauses = setOf(Player.PLAY_WHEN_READY_CHANGE_REASON_AUDIO_BECOMING_NOISY, Player.PLAY_WHEN_READY_CHANGE_REASON_AUDIO_FOCUS_LOSS)

private fun uptime() = SystemClock.uptimeMillis()

/** How far ahead of the host a joining guest parks while it buffers. */
private const val JOIN_LEAD_MS = 8_000L
/** Buffer required ahead of the start point before playing. */
private const val MIN_BUFFER_MS = 6_000L
private const val BUFFER_TIMEOUT_MS = 15_000L
/** Starting this much later than planned is fine (the core tolerates a few seconds); later than that, park again. */
private const val LATE_TOLERANCE_MS = 1_000L
private const val ALIGN_ATTEMPTS = 3
/** Upper bound for aiming ahead of a slow drift seek. */
private const val MAX_SEEK_LEAD_MS = 3_000L
/** Largest guest-host gap a resume is synchronized over. */
private const val RESUME_WINDOW_MS = 3_000L

/** Where a track plays from; [offline] for its downloaded copy. */
private class Source(val uri: String, val quality: Quality, val offline: Boolean)

private class Preloaded(val item: MediaItem, val source: Source)

private val queueCommands = intArrayOf(
    Player.COMMAND_SEEK_TO_NEXT,
    Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
    Player.COMMAND_SEEK_TO_PREVIOUS,
    Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
)

// The core only hands out HLS manifests, and a download is indexed by a local one.
private fun mediaItem(t: Track, src: Source) =
    MediaItem.Builder().setMediaId(t.id).setUri(src.uri).setMimeType(MimeTypes.APPLICATION_M3U8).setMediaMetadata(metadata(t)).build()

private fun metadata(t: Track) = MediaMetadata.Builder()
    .setTitle(t.displayTitle)
    .setArtist(t.artistLine)
    .setAlbumTitle(t.album?.title)
    .setArtworkUri(Downloads.coverFile(t.album?.coverUrlLarge)?.let(Uri::fromFile) ?: t.album?.coverUrlLarge?.let(Uri::parse))
    .build()

internal fun clampSeek(secs: Double, duration: Double) = if (duration > 0) secs.coerceIn(0.0, duration) else maxOf(secs, 0.0)

/** The tier the stream's formats deliver; [Stream.quality] only when it lists none. */
val Stream.actualQuality get() = when {
    "FLAC_HIRES" in formats -> Quality.HI_FI
    "FLAC" in formats -> Quality.CD
    formats.isEmpty() -> quality
    else -> Quality.AAC
}

val Quality.badgeText get() = when (this) {
    Quality.HI_FI -> "Hi-Res Lossless"
    Quality.CD -> "CD Quality"
    Quality.AAC -> "AAC"
}

internal fun progress(position: Double, duration: Double) = if (duration > 0) minOf(position / duration, 1.0) else 0.0

/** Like iOS: past 3 s, or on the first track, previous goes back to its start; otherwise to the track before. */
internal enum class Seek { Next, Previous, InCurrent, Ignore }

/** A controller's seek: the player holds only the current item and the preloaded one behind it, so moving to another
 * item goes through the queue. */
internal fun seekAction(command: Int, index: Int, current: Int) = when (command) {
    Player.COMMAND_SEEK_TO_NEXT, Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM -> Seek.Next
    Player.COMMAND_SEEK_TO_PREVIOUS, Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> Seek.Previous
    else -> when (index) {
        current -> Seek.InCurrent
        current + 1 -> Seek.Next
        else -> Seek.Ignore
    }
}

internal enum class Tap { Retry, Replay, Pause, Resume }

/** Pauses on `playWhenReady`, not `isPlaying`, so a tap while buffering still pauses. */
internal fun tapAction(unprepared: Boolean, ended: Boolean, playWhenReady: Boolean) = when {
    unprepared -> Tap.Retry
    ended -> Tap.Replay
    playWhenReady -> Tap.Pause
    else -> Tap.Resume
}
