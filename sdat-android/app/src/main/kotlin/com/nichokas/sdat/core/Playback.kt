package com.nichokas.sdat.core

import android.content.Context
import android.net.Uri
import android.os.SystemClock
import androidx.annotation.OptIn
import androidx.compose.runtime.getValue
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
import java.util.concurrent.Executors
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
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
class Playback(context: Context, private val prefs: Preferences, secrets: Secrets) : TrackPlayer {
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
    private val store = PlaybackStore(context.filesDir)
    private val io = Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    /** The queue file on disk; a state is saved only against it. Only touched on [io]. */
    private var diskQueueId: Int? = null
    private var restoring = true
    /** A restored Discover queue, until [resumeInfinite] gives it its endless source back. */
    private var restoredEndless = false
    /** Home mixes that arrived while the saved queue was still loading. */
    private var pendingMixes: List<Mix>? = null
    private var queueId = 0
    /** The lists the queue file was cut from. */
    private var savedLists: List<Any?>? = null
    /** The base-queue indices the queue file holds. */
    private var savedRange = IntRange.EMPTY
    private var lastSave = 0L

    val track get() = queue.current
    override val current get() = track
    var playing by mutableStateOf(false)
        private set
    override val isPlaying get() = playing
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
    val loading get() = fetching || buffering
    var error by mutableStateOf<String?>(null)
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
                Seek.Next -> next()
                Seek.Previous -> previous()
                // Through [seek], so a seek while paused is kept and saved.
                Seek.InCurrent -> seek(if (positionMs == C.TIME_UNSET) 0.0 else positionMs / 1000.0)
                Seek.Ignore -> {}
            }
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
            player.playWhenReady = false
            return super.handleStop()
        }
    }

    private val restore = scope.launch {
        val saved = withContext(io) { store.load() }
        restoring = false
        // Anything played meanwhile wins, and is saved now that persisting is on.
        if (track != null) return@launch persist()
        if (saved == null) return@launch
        val (state, queued) = saved
        queue.restore(queued, state)
        restoredEndless = queued?.endless == true
        // The queue file is rewritten on the next persist: the restored index no longer counts from its offset.
        queueId = state.queueId
        duration = state.current.durationSecs.toDouble()
        position = maxOf(0.0, minOf(state.position, if (duration > 0) duration - 1 else state.position))
        pendingMixes?.let(::resumeInfinite)
        pendingMixes = null
    }

    init {
        player.addListener(object : Player.Listener {
            // The requested state, so the icon matches what a tap does even while audio focus suppresses output.
            override fun onEvents(player: Player, events: Player.Events) {
                playing = player.playWhenReady && player.playbackState != Player.STATE_ENDED
                tick()
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
                if (reason == Player.MEDIA_ITEM_TRANSITION_REASON_AUTO && next != null && p != null && item == p.item) adopt(next, p.quality)
            }

            override fun onPlaybackStateChanged(state: Int) {
                buffering = state == Player.STATE_BUFFERING
                if (state == Player.STATE_ENDED) advance(auto = true)
            }

            override fun onPlayerError(e: PlaybackException) {
                error = e.errorCodeName
            }
        })
    }

    override fun play(tracks: List<Track>, startAt: Int, from: String?) {
        val t = queue.play(tracks, startAt, from) ?: return
        replaceInfinite(null)
        start(t)
    }

    /** Shuffle buttons; shuffle stays on for later lists, like iOS. */
    fun playShuffled(tracks: List<Track>, from: String? = null) {
        val t = queue.playShuffled(tracks, from) ?: return
        replaceInfinite(null)
        start(t)
    }

    /** Tapping a Discover cover: that song, the rest of the visible pool, then the endless Discover shuffle. */
    fun playDiscover(t: Track, pool: List<Track>, mixes: List<Mix>) {
        queue.discover(t, pool.filter { it.streamable })
        replaceInfinite(Shuffle(mixes))
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

    override fun playNext(track: Track) {
        if (this.track == null) return play(listOf(track))
        queue.playNext(track)
        changed()
    }

    override fun addToQueue(track: Track) {
        if (this.track == null) return play(listOf(track))
        queue.addToQueue(track)
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
        // The same play goes on, so it keeps what it counted towards its scrobble.
        val heard = listen
        start(t, play = player.playWhenReady, resumeAt = position)
        heard?.let { listen = it }
    }

    /** Settings: the preload follows the switch at once. */
    fun setPreloadNext(on: Boolean) {
        prefs.preloadNext = on
        syncPreload()
    }

    /** Gives a restored Discover queue its endless source back once the home mixes load. */
    fun resumeInfinite(mixes: List<Mix>) {
        if (restoring) {
            pendingMixes = mixes
            return
        }
        if (!restoredEndless || infinite != null || mixes.isEmpty()) return
        replaceInfinite(Shuffle(mixes))
        refillIfNeeded()
    }

    fun next() {
        if (track != null) advance(auto = false)
    }

    fun previous() {
        val i = queue.index ?: return
        if (restartsOnPrevious(position, i)) seek(0.0) else queue.select(i - 1)?.let { start(it) }
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
            began(t, p.quality)
            return
        }
        if (auto && t != null && t === track && player.currentMediaItem?.mediaId == t.id) {
            // Repeat-one, or a one-track queue on repeat-all: the ended item plays again without a new fetch.
            player.seekToDefaultPosition()
            player.playWhenReady = play
            position = 0.0
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
            if (track !== from) return@launch
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
        restoredEndless = false
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
        if (player.isPlaying && SystemClock.elapsedRealtime() - lastSave > 5000) persist()
    }

    private fun start(t: Track, play: Boolean = true, resumeAt: Double = 0.0) {
        request?.cancel()
        dropPreload()
        position = resumeAt
        duration = t.durationSecs.toDouble()
        fetching = true
        listen = newListen(t)
        // Stopped, not cleared: an empty player drops the media notification and with it the foreground service,
        // which can't be started again while the app is in the background.
        player.stop()
        player.currentMediaItem?.let { player.replaceMediaItem(player.currentMediaItemIndex, it.buildUpon().setMediaMetadata(metadata(t)).build()) }
        error = null
        // Requested before the fetch so a pause tapped while the URL loads still holds after prepare().
        player.playWhenReady = play
        persist()
        request = scope.launch {
            try {
                val s = stream(t.id, prefs.streamingQuality)
                shownQuality = s.actualQuality
                player.setMediaItem(mediaItem(t, s), (resumeAt * 1000).toLong())
                player.prepare()
                fetching = false
                started(t)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                error = e.message ?: e.javaClass.simpleName
                fetching = false
            }
        }
    }

    /** A preloaded track the player moved on to by itself: it becomes the current one without a new fetch. */
    private fun adopt(t: Track, quality: Quality) {
        preloading = null
        preloaded = null
        player.removeMediaItem(0)
        val next = queue.advance(manual = false)
        // Preloading follows every queue change, so this is the track it fetched.
        if (next?.id != t.id) return next?.let { start(it) } ?: stopAtEnd()
        began(t, quality)
    }

    /** The preloaded item became the current one. */
    private fun began(t: Track, quality: Quality) {
        position = 0.0
        duration = t.durationSecs.toDouble()
        shownQuality = quality
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
        val next = queue.preloadable?.takeIf { prefs.preloadNext && !fetching && player.currentMediaItem?.mediaId == track?.id }
        if (next != null && next.id == preloading?.id) return
        dropPreload()
        if (next == null) return
        preloading = next
        preload = scope.launch {
            val s = try {
                stream(next.id, prefs.streamingQuality)
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                // The next sync tries this track again.
                if (isActive) preloading = null
                return@launch
            }
            // Dropped while the stream call returned: the player no longer expects this item.
            if (!isActive) return@launch
            val item = mediaItem(next, s)
            player.addMediaItem(item)
            preloaded = Preloaded(item, s.actualQuality)
        }
    }

    private fun dropPreload() {
        preload?.cancel()
        preload = null
        preloading = null
        preloaded = null
        if (player.mediaItemCount > 1) player.removeMediaItems(1, player.mediaItemCount)
    }

    private val endless get() = infinite != null || restoredEndless

    private fun queueLists() = listOf(queue.tracks, queue.original, queue.userQueue, queue.source, endless)

    /** iOS persist: the small state on every call, the queue only when it changed. */
    private fun persist() {
        if (restoring) return
        lastSave = SystemClock.elapsedRealtime()
        val current = queue.current
        if (current == null) {
            savedLists = null
            scope.launch(io) { store.clear() }
            return
        }
        val lists = queueLists()
        if (queueFileStale(savedLists, lists, savedRange, queue.index)) {
            savedLists = lists
            val saved = queue.saved(++queueId, endless)
            savedRange = saved.offset until saved.offset + saved.order.size
            scope.launch(io) {
                if (store.saveQueue(saved)) {
                    diskQueueId = saved.id
                } else {
                    // The next persist tries again; until then states stay with the queue file still on disk.
                    scope.launch { if (savedLists === lists) savedLists = null }
                }
            }
        }
        val state = SavedState(queueId, current, queue.index, position, queue.repeat, queue.shuffle)
        scope.launch(io) { if (state.queueId == diskQueueId) store.saveState(state) }
    }

    /** After a failure or a controller's stop there is nothing prepared, so this starts the track again where it was;
     * a restored queue starts this way too. */
    fun toggle() {
        val t = track ?: return
        val unprepared = error != null || !fetching && player.playbackState == Player.STATE_IDLE
        when (tapAction(unprepared, player.playbackState == Player.STATE_ENDED, player.playWhenReady)) {
            Tap.Retry -> start(t, resumeAt = position)
            Tap.Replay -> {
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
        dropPreload()
        player.pause()
        player.stop()
        player.clearMediaItems()
        position = 0.0
        duration = 0.0
        queue.clear()
        replaceInfinite(null)
        error = null
        fetching = false
        listen = null
        persist()
    }
}

private class Preloaded(val item: MediaItem, val quality: Quality)

private val queueCommands = intArrayOf(
    Player.COMMAND_SEEK_TO_NEXT,
    Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
    Player.COMMAND_SEEK_TO_PREVIOUS,
    Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
)

// The core only hands out HLS manifests.
private fun mediaItem(t: Track, s: Stream) =
    MediaItem.Builder().setMediaId(t.id).setUri(s.url).setMimeType(MimeTypes.APPLICATION_M3U8).setMediaMetadata(metadata(t)).build()

private fun metadata(t: Track) = MediaMetadata.Builder()
    .setTitle(t.displayTitle)
    .setArtist(t.artistLine)
    .setAlbumTitle(t.album?.title)
    .setArtworkUri(t.album?.coverUrlLarge?.let(Uri::parse))
    .build()

/** The queue file must be rewritten when a list it was cut from was replaced (PlayQueue replaces a list whenever it
 * changes) or the playing index left the window it holds. */
internal fun queueFileStale(saved: List<Any?>?, now: List<Any?>, savedRange: IntRange, index: Int?) =
    saved == null || saved.zip(now).any { (a, b) -> a !== b } || index != null && index !in savedRange

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
internal fun restartsOnPrevious(position: Double, index: Int) = position > 3 || index == 0

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
