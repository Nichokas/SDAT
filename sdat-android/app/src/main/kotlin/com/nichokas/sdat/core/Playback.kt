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
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
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
import uniffi.tonearm_ffi.homeMixes
import uniffi.tonearm_ffi.stream

/** The app's single player: the iOS Player's queues, the current track from the core's stream URL with the next one
 * preloaded behind it, scrobbling, and the queue saved across restarts. */
// ForwardingSimpleBasePlayer, which the session player is built on, is still marked unstable.
@OptIn(UnstableApi::class)
class Playback(context: Context, private val prefs: Preferences, secrets: Secrets) : TrackPlayer {
    private val scope = MainScope()
    private val player = ExoPlayer.Builder(context)
        .setAudioAttributes(
            AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(),
            true,
        )
        .setHandleAudioBecomingNoisy(true)
        .build()
    internal val liveEngine: LiveEngine = LiveEngine(object : LivePlaybackPort {
        override val currentId get() = track?.id
        override val engineId get() = player.currentMediaItem?.mediaId
        override val requested get() = player.playWhenReady
        override val advancing get() = player.isPlaying
        override val logicalPlaying get() = playing
        override val loading get() = this@Playback.loading
        override val fetching get() = this@Playback.fetching
        override val hasError get() = error != null
        override val suppressed get() = player.playbackSuppressionReason != Player.PLAYBACK_SUPPRESSION_REASON_NONE
        override val idle get() = player.playbackState == Player.STATE_IDLE
        override val ended get() = player.playbackState == Player.STATE_ENDED
        override val ready get() = player.playbackState == Player.STATE_READY
        override val failed get() = player.playerError != null
        override val positionMs get() = player.currentPosition
        override val logicalPositionMs get() = (position * 1000).toLong()
        override val durationMs get() = player.duration
        override val bufferedMs get() = player.bufferedPosition

        override fun playEngine() = player.play()
        override fun pauseEngine() = player.pause()
        override fun seekEngine(ms: Long) = player.seekTo(ms)
        override fun setLogicalPosition(ms: Long) { position = ms / 1000.0 }
        override fun setLogicalPlaying(playing: Boolean) { this@Playback.playing = playing }
        override fun requestPause() = this@Playback.pause()
        override fun seek(ms: Long) = this@Playback.seek(ms / 1000.0)
        override fun toggle() = this@Playback.toggle()
        override fun load(track: Track, atMs: Long, playing: Boolean) = loadLiveTrack(track, atMs, playing)
        override fun followingChanged() = syncPreload()
    }, scope, SystemClock::uptimeMillis)
    private var request: Job? = null
    private val pendingWait = AdvanceWait()
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
    /** The base queue a Handoff resume restored, until what follows its track is appended. Any new list replaces it;
     * a restart or a repeat keeps it. */
    private var resumedBase: List<Track>? = null
    /** Bumped by every play or pause command, so a late handoff result knows the user acted. */
    var controls = 0
        private set

    val track get() = queue.current
    override val current get() = track
    var playing by mutableStateOf(false)
        private set
    override val isPlaying get() = playing
    val userQueue get() = queue.userQueue
    val upNext get() = queue.upNext
    val played get() = queue.played
    val shuffle get() = queue.shuffle
    val repeat get() = queue.repeat
    val source get() = queue.source
    /** Keeps the previous track's value until the new stream's real quality is known. */
    private var shownQuality by mutableStateOf<Quality?>(null)
    val badgeQuality get() = shownQuality ?: prefs.streamingQuality
    val streamingQuality get() = prefs.streamingQuality
    private var fetching by mutableStateOf(false)
    private var buffering by mutableStateOf(false)
    private val busy get() = fetching || pendingWait.active
    val loading: Boolean get() = busy || buffering || liveEngine.aligning
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
    private var refetched: Pair<String, Long>? = null

    /** What the MediaSession drives: the player, with next/previous going through this queue (ExoPlayer holds only
     * the current track and the preloaded one). */
    inner class SessionPlayer : ForwardingSimpleBasePlayer(player) {
        /** For changes ExoPlayer never sees: the restored queue landing, a seek or a new track before its stream. */
        fun refresh() = invalidateState()

        override fun getState(): State {
            val state = super.getState()
            val t = track
            val commands = sessionCommands(state.availableCommands.toIntSet(), hasTrack = t != null, emptyTimeline = state.timeline.isEmpty)
                .toPlayerCommands()
            if (!state.timeline.isEmpty || t == null) return state.buildUpon().setAvailableCommands(commands).build()
            // Never handed to ExoPlayer, which throws building a source for an item without a URI.
            val item = MediaItemData.Builder(t.id)
                .setMediaItem(MediaItem.Builder().setMediaId(t.id).setMediaMetadata(metadata(t)).build())
                .setDurationUs(if (duration > 0) (duration * 1_000_000).toLong() else C.TIME_UNSET)
                .setIsSeekable(true)
                .build()
            return state.buildUpon()
                .setPlaylist(listOf(item))
                .setCurrentMediaItemIndex(0)
                .setContentPositionMs((position * 1000).toLong())
                .setAvailableCommands(commands)
                .build()
        }

        private fun Player.Commands.toIntSet(): Set<Int> = buildSet(size()) {
            for (i in 0 until size()) add(get(i))
        }

        private fun Set<Int>.toPlayerCommands(): Player.Commands =
            Player.Commands.Builder().apply { forEach(::add) }.build()

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
            if (!Live.isGuest) {
                // Notification and headset play/pause count as user commands too.
                controls++
                return super.handleSetPlayWhenReady(playWhenReady)
            }
            if (playWhenReady) Live.userPlay() else Live.userPause()
            return Futures.immediateVoidFuture()
        }

        // The session prepares a stopped player when play is pressed. Stopped means the stream failed, a controller
        // stopped it or nothing was loaded yet (a restored queue); the item left in the player may still be the
        // previous track's stream, so the current track starts again where it was.
        override fun handlePrepare(): ListenableFuture<*> {
            if (!busy) track?.let { start(it, play = player.playWhenReady, resumeAt = position) }
            return Futures.immediateVoidFuture()
        }

        // A stream or refill still in flight would prepare() and play after the stop.
        override fun handleStop(): ListenableFuture<*> {
            request?.cancel()
            pendingWait.cancel()
            fetching = false
            // A guest's start may still be aligning; left set, it reads as loading and the follower never reloads.
            liveEngine.cancelLive()
            player.playWhenReady = false
            return super.handleStop()
        }
    }

    val sessionPlayer = SessionPlayer()

    private val restore = scope.launch {
        val saved = saver.load()
        restoring = false
        // Anything played meanwhile wins, and is saved now that persisting is on. A Discover queue among it still
        // waits for its source.
        if (track != null) {
            persist()
            pendingMixes?.let(::resumeInfinite)
            pendingMixes = null
            return@launch
        }
        if (saved == null) return@launch
        val (state, queued) = saved
        queue.restore(queued, state)
        awaitingMixes = queued?.endless == true
        duration = state.current.durationSecs.toDouble()
        position = maxOf(0.0, minOf(state.position, if (duration > 0) duration - 1 else state.position))
        pendingMixes?.let(::resumeInfinite)
        pendingMixes = null
        sessionPlayer.refresh()
    }

    init {
        player.addListener(object : Player.Listener {
            // The requested state, so the icon matches what a tap does even while audio focus suppresses output.
            override fun onEvents(player: Player, events: Player.Events) {
                liveEngine.syncPlaying()
                tick()
            }

            override fun onPlayWhenReadyChanged(playWhenReady: Boolean, reason: Int) {
                liveEngine.playRequestChanged(playWhenReady, reason in systemPauses)
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
                if (state == Player.STATE_READY) liveEngine.clearLoadPosition()
                // A Listen Together guest waits for the host's next track instead of advancing.
                if (state == Player.STATE_ENDED) if (liveEngine.liveFollowing) player.pause() else advance(auto = true)
            }

            override fun onPlayerError(e: PlaybackException) {
                val t = track ?: return fail(e.errorCodeName)
                // Counts deep sleep, which uptime skips: the expiry follows a pause of hours.
                val now = SystemClock.elapsedRealtime()
                // A broken download spends no budget: a new play picks the same file again, and only its online stream
                // can help.
                if (!offline) {
                    if (!refetchDue(refetched, t.id, now)) return fail(e.errorCodeName)
                    refetched = t.id to now
                }
                // A guest still aligning has the player parked paused, so the new item aligns again like a fresh start.
                restart(t, position, online = true, play = player.playWhenReady || liveEngine.aligning)
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

    /** Reloads the current stream at the same position and keeps downloaded playback going. */
    fun setStreamingQuality(q: Quality) {
        if (q == prefs.streamingQuality) return
        prefs.streamingQuality = q
        val t = track ?: return
        if (offline && Downloads.localStream(t.id) != null) {
            if (preloaded?.source?.offline != true) {
                dropPreload()
                syncPreload()
            }
            return
        }
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

    private var mixesJob: Job? = null

    /** Asks for the home mixes while a Discover queue restored or taken over waits for its source, since Discover
     * may have none loaded: without it, playback ends with the tracks it has. */
    private fun fetchMixes() {
        mixesJob?.cancel()
        mixesJob = scope.launch {
            var pause = 2_000L
            while (awaitingMixes && infinite == null) {
                val mixes = try { homeMixes() } catch (e: CancellationException) { throw e } catch (e: Exception) { emptyList() }
                if (mixes.isNotEmpty()) return@launch resumeInfinite(mixes)
                delay(pause)
                pause = minOf(pause * 2, 60_000L)
            }
        }
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
        if (track == null || busy) return
        controls++
        val to = clampSeek(secs, duration)
        player.seekTo((to * 1000).toLong())
        position = to
        persist()
    }

    private fun advance(auto: Boolean, play: Boolean = true) {
        when (
            advanceAction(
                auto = auto,
                repeat = queue.repeat,
                hasNext = queue.upNext.isNotEmpty() || queue.userQueue.isNotEmpty(),
                hasInfinite = infinite != null,
                awaitingMixes = awaitingMixes,
                awaitingUpNext = awaitingUpNext,
            )
        ) {
            Advance.Stop -> return stopAtEnd()
            Advance.Queue -> {
                val t = queue.advance(manual = !auto) ?: return stopAtEnd()
                val p = preloaded
                if (t === preloading && p != null) {
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
                if (auto && t === track && player.currentMediaItem?.mediaId == t.id) {
                    // Repeat-one, or a one-track queue on repeat-all: the ended item plays again without a new fetch.
                    pendingWait.cancel()
                    player.seekToDefaultPosition()
                    player.playWhenReady = play
                    position = 0.0
                    entry++
                    listen = newListen(t)
                    persist()
                    started(t)
                    return
                }
                start(t, play)
                return
            }
            Advance.Wait -> {}
        }
        // Restored or taken over, and its source is not here yet: asked for now.
        if (awaitingMixes && infinite == null && mixesJob?.isActive != true) fetchMixes()
        val from = track
        val entryBefore = entry
        // Like a skip to a queued track: the old one goes quiet, and a play or pause tapped while the refill loads
        // decides how the next one starts.
        if (!auto) {
            request?.cancel()
            player.stop()
            player.playWhenReady = true
            fetching = false
        }
        pendingWait.launchOrJoin(scope, auto) {
            // A handoff's own tracks come first, when they are still on their way, and a Discover queue waits for
            // its source. A track to play ends the wait, and so does another start. The tracks have a limit, for an
            // answer of the core that never comes; the source is asked for until it comes.
            val limit = SystemClock.elapsedRealtime() + HANDOFF_WAIT_MS
            // Handed over, or added with Play Next meanwhile.
            val queued = { queue.upNext.isNotEmpty() || queue.userQueue.isNotEmpty() }
            while (track === from && entry == entryBefore && !queued()) {
                val tracks = awaitingUpNext && SystemClock.elapsedRealtime() < limit
                val source = awaitingMixes && infinite == null
                if (!tracks && !source) break
                delay(100)
            }
            // A track picked during the wait owns playback now: its queue is not this one's to refill.
            if (track !== from || entry != entryBefore) return@launchOrJoin
            // Once the handoff wait ends and resumedBase is cleared, Repeat.All wraps the settled queue normally instead of stopping or pausing.
            if (!queued() && awaitingUpNext) resumedBase = null
            if (!queued()) infinite?.let { fetchMore(it) }
            // A track picked during the fetch owns playback now; a source replaced meanwhile had nothing more.
            if (track !== from || entry != entryBefore) return@launchOrJoin
            val finalAuto = pendingWait.auto
            // A Replay tapped during the fetch wins; that track's next end advances again.
            if (finalAuto && player.playbackState != Player.STATE_ENDED) {
                pendingWait.clear(coroutineContext.job)
                syncPreload()
                return@launchOrJoin
            }
            pendingWait.clear(coroutineContext.job)
            when (postWaitAction(auto = finalAuto, hasNext = queued(), repeat = queue.repeat)) {
                PostWait.Advance -> advance(finalAuto, play = player.playWhenReady)
                PostWait.StopAtEnd -> stopAtEnd()
                // The skip found nothing; the player may hold a stale item, so the track is loaded again, paused.
                PostWait.RestartPaused -> from?.let { start(it, play = false) }
            }
        }
    }

    /** End of a finite queue: rewound and paused, and saved that way. Playing it again counts as a new listen. */
    private fun stopAtEnd() {
        pendingWait.cancel()
        player.pause()
        player.seekToDefaultPosition()
        position = 0.0
        listen = null
        persist()
    }

    private fun refillIfNeeded() {
        val source = infinite ?: return
        // A handoff's own tracks come first: a refill before them would take their place.
        if (awaitingUpNext) return
        if (queue.needsRefill) scope.launch { fetchMore(source) }
    }

    /** What follows a handoff's resumed track may still come: its queue is the one playing, as the resume left it. */
    private val awaitingUpNext get() = resumedBase.let { it != null && queue.original === it }

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
        if (busy || player.currentMediaItem?.mediaId != track?.id) return
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
        pendingWait.cancel()
        liveEngine.cancelLive()
        dropPreload()
        // Following a playing host: start only with a healthy buffer and on time, so nothing needs correcting after.
        val align = liveEngine.shouldAlign(play)
        position = resumeAt
        liveEngine.loadingFrom((resumeAt * 1000).toLong())
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
        liveEngine.prepareAlignment(align)
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
                // Cleared by a pause tapped while the stream loaded.
                liveEngine.startAlignment()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                fetching = false
                fail(friendlyMessage(e))
            }
        }
    }

    private fun fail(message: String) {
        liveEngine.cancelLive()
        liveEngine.clearLoadPosition()
        error = message
        player.playWhenReady = false
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
        pendingWait.cancel()
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
        val waiting = awaitingUpNext && queue.upNext.isEmpty() && queue.userQueue.isEmpty()
        val next = queue.preloadable?.takeIf { prefs.preloadNext && !liveEngine.liveFollowing && !busy && !waiting && player.currentMediaItem?.mediaId == track?.id }
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

    /** The queue is Discover's endless shuffle, also before its source is attached. */
    val endless get() = infinite != null || awaitingMixes

    private fun persist() {
        sessionPlayer.refresh()
        if (restoring) return
        saver.save(queue, endless, position)
        onChange?.invoke()
    }

    /** Called with every save: track, queue, repeat, play, pause and seek changes, and every few seconds while
     * playing. */
    var onChange: (() -> Unit)? = null

    /** Another device's playback (Handoff), loaded like the queue saved at launch and played from [atMs].
     * [discover]: Discover's endless queue, which goes on with this device's own shuffle. */
    fun resume(current: Track, atMs: Long, repeat: Repeat, discover: Boolean) {
        // An arrival that plays here is newer than a Continue here still on its way.
        controls++
        val (saved, state) = resumed(current, maxOf(atMs, 0) / 1000.0, repeat, discover)
        queue.restore(saved, state)
        replaceInfinite(null)
        resumedBase = queue.original
        awaitingMixes = discover
        resumeInfinite(Discover.mixes)
        if (discover) fetchMixes()
        start(current, resumeAt = state.position)
    }

    /** The rest of a resumed queue (Handoff), after its track [after], while that resume's queue still is the one
     * playing. No [tracks]: nothing follows it, and a Discover queue goes on from its own source. */
    fun appendUpNext(after: String, tracks: List<Track>) {
        val base = resumedBase
        if (base == null || queue.original !== base || !upcomingApplies(after, track?.id, base.map { it.id })) return
        resumedBase = null
        if (tracks.isNotEmpty()) {
            queue.extend(tracks)
            changed()
        } else {
            syncPreload()
        }
        refillIfNeeded()
    }

    /** After a failure or a controller's stop there is nothing prepared, so this starts the track again where it was;
     * a restored queue starts this way too. */
    fun toggle() {
        val t = track ?: return
        controls++
        if (liveEngine.dropLiveStart()) return
        liveEngine.clearSystemPause()
        val unprepared = error != null || !busy && player.playbackState == Player.STATE_IDLE
        when (tapAction(unprepared, player.playbackState == Player.STATE_ENDED, player.playWhenReady)) {
            Tap.Retry -> start(t, resumeAt = position)
            Tap.Replay -> {
                // The ended item plays again with no fetch; a refill still loading gives way to it.
                pendingWait.cancel()
                entry++
                listen = newListen(t)
                player.seekToDefaultPosition()
                player.play()
                syncPreload()
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
        pendingWait.cancel()
        resumedBase = null
        // Mixes still on their way are this account's, not the next one's.
        mixesJob?.cancel()
        liveEngine.cancelLive()
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

    val livePublishedPositionMs get() = liveEngine.livePublishedPositionMs

    fun pause() {
        controls++
        if (!liveEngine.dropLiveStart()) player.pause()
    }

    private fun loadLiveTrack(track: Track, atMs: Long, playing: Boolean) {
        queue.play(listOf(track), 0, "Listen Together")
        replaceInfinite(null)
        start(track, playing, resumeAt = maxOf(atMs, 0) / 1000.0)
    }
}

private val systemPauses = setOf(Player.PLAY_WHEN_READY_CHANGE_REASON_AUDIO_BECOMING_NOISY, Player.PLAY_WHEN_READY_CHANGE_REASON_AUDIO_FOCUS_LOSS)

/** How long a queue that ran dry waits for the tracks a handoff still owes it. */
private const val HANDOFF_WAIT_MS = 20_000L

/** Where a track plays from; [offline] for its downloaded copy. */
private class Source(val uri: String, val quality: Quality, val offline: Boolean)

private class Preloaded(val item: MediaItem, val source: Source)

private val queueCommands = setOf(
    Player.COMMAND_SEEK_TO_NEXT,
    Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
    Player.COMMAND_SEEK_TO_PREVIOUS,
    Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
)

private val unguardedSessionCommands = setOf(
    Player.COMMAND_SET_REPEAT_MODE,
    Player.COMMAND_SET_SHUFFLE_MODE,
    Player.COMMAND_SET_SPEED_AND_PITCH,
    Player.COMMAND_SET_MEDIA_ITEM,
    Player.COMMAND_CHANGE_MEDIA_ITEMS,
    Player.COMMAND_RELEASE,
)

/** What a controller may do with no track: read state and change the volume, which does not touch the queue. */
private val idleSessionCommands = setOf(
    Player.COMMAND_GET_CURRENT_MEDIA_ITEM,
    Player.COMMAND_GET_TIMELINE,
    Player.COMMAND_GET_METADATA,
    Player.COMMAND_GET_AUDIO_ATTRIBUTES,
    Player.COMMAND_GET_VOLUME,
    Player.COMMAND_GET_DEVICE_VOLUME,
    Player.COMMAND_GET_TEXT,
    Player.COMMAND_GET_TRACKS,
    Player.COMMAND_SET_VOLUME,
    Player.COMMAND_SET_DEVICE_VOLUME_WITH_FLAGS,
    Player.COMMAND_ADJUST_DEVICE_VOLUME_WITH_FLAGS,
)

internal fun sessionCommands(base: Set<Int>, hasTrack: Boolean, emptyTimeline: Boolean): Set<Int> {
    if (!hasTrack) return base intersect idleSessionCommands
    val allowed = base - unguardedSessionCommands
    return if (emptyTimeline) allowed + queueCommands + Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM else allowed + queueCommands
}

internal enum class Advance { Queue, Wait, Stop }

internal fun advanceAction(
    auto: Boolean,
    repeat: Repeat,
    hasNext: Boolean,
    hasInfinite: Boolean,
    awaitingMixes: Boolean,
    awaitingUpNext: Boolean,
): Advance = when {
    auto && repeat == Repeat.One -> Advance.Queue
    hasNext -> Advance.Queue
    awaitingUpNext -> Advance.Wait
    repeat == Repeat.All -> Advance.Queue
    hasInfinite || awaitingMixes -> Advance.Wait
    else -> Advance.Stop
}

internal class AdvanceWait {
    private var job: Job? = null
    var active by mutableStateOf(false)
        private set
    var auto = false
        private set

    /** Reuses an active wait (upgrading auto to manual on a skip) or starts a new one. */
    fun launchOrJoin(scope: CoroutineScope, auto: Boolean, block: suspend CoroutineScope.() -> Unit) {
        if (job?.isActive == true) {
            this.auto = this.auto && auto
            return
        }
        this.auto = auto
        val launched = scope.launch(start = CoroutineStart.LAZY) {
            try {
                block()
            } finally {
                clear(coroutineContext.job)
            }
        }
        job = launched
        active = true
        launched.invokeOnCompletion { clear(launched) }
        launched.start()
    }

    fun clear(current: Job) {
        if (job === current) {
            job = null
            active = false
        }
    }

    fun cancel() {
        val current = job
        job = null
        active = false
        current?.cancel()
    }
}

internal enum class PostWait { Advance, StopAtEnd, RestartPaused }

/** Once the handoff wait ends and resumedBase is cleared, Repeat.All wraps the settled queue normally instead of stopping or pausing. */
internal fun postWaitAction(auto: Boolean, hasNext: Boolean, repeat: Repeat): PostWait = when {
    hasNext || repeat == Repeat.All || (auto && repeat == Repeat.One) -> PostWait.Advance
    auto -> PostWait.StopAtEnd
    else -> PostWait.RestartPaused
}

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

/** A dead URL fails again within seconds, an expired one hours later. */
internal fun refetchDue(last: Pair<String, Long>?, trackId: String, now: Long) =
    last == null || last.first != trackId || now - last.second >= RETRY_WINDOW_MS

internal const val RETRY_WINDOW_MS = 60_000L
