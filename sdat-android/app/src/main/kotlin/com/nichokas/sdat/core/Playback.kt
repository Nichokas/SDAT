package com.nichokas.sdat.core

import android.content.Context
import android.net.Uri
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
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Mix
import uniffi.tonearm_ffi.Quality
import uniffi.tonearm_ffi.Shuffle
import uniffi.tonearm_ffi.Stream
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.stream

/** The app's single player: the iOS Player's queues, one track at a time from the core's stream URL.
 * Reads [quality] per request so a change in Settings applies to the next track. */
// ForwardingSimpleBasePlayer, which the session player is built on, is still marked unstable.
@OptIn(UnstableApi::class)
class Playback(context: Context, private val quality: () -> Quality) : TrackPlayer {
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
    val badgeQuality get() = shownQuality ?: quality()
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

    /** What the MediaSession drives: the player, with next/previous going through this queue (ExoPlayer only ever
     * holds the current track). */
    val sessionPlayer: Player = object : ForwardingSimpleBasePlayer(player) {
        override fun getState(): State {
            val state = super.getState()
            if (state.playlist.isEmpty()) return state
            return state.buildUpon().setAvailableCommands(state.availableCommands.buildUpon().addAll(*queueCommands).build()).build()
        }

        override fun handleSeek(mediaItemIndex: Int, positionMs: Long, seekCommand: Int): ListenableFuture<*> {
            when (seekCommand) {
                Player.COMMAND_SEEK_TO_NEXT, Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM -> next()
                Player.COMMAND_SEEK_TO_PREVIOUS, Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> previous()
                else -> return super.handleSeek(mediaItemIndex, positionMs, seekCommand)
            }
            return Futures.immediateVoidFuture()
        }

        // The session prepares a stopped player when play is pressed. Stopped means the stream failed or a controller
        // stopped it; the item left in the player may still be the previous track's stream, so start the current
        // track afresh.
        override fun handlePrepare(): ListenableFuture<*> {
            if (!fetching) track?.let { start(it, play = player.playWhenReady) }
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
                if (infinite === source) error = friendlyMessage(e)
                return@async false
            }
            if (infinite !== source) return@async false
            queue.startInfinite(batch)?.let { start(it) } != null
        }
        refilling = job
        job.invokeOnCompletion { if (refilling === job) refilling = null }
    }

    override fun playNext(track: Track) {
        if (this.track == null) play(listOf(track)) else queue.playNext(track)
    }

    override fun addToQueue(track: Track) {
        if (this.track == null) play(listOf(track)) else queue.addToQueue(track)
    }

    fun jumpToUpNext(offset: Int) {
        queue.jumpToUpNext(offset)?.let { start(it) }
    }

    fun jumpToUser(offset: Int) {
        queue.jumpToUser(offset)?.let { start(it) }
    }

    fun removeUpNext(offset: Int) = queue.removeUpNext(offset)

    fun removeUser(offset: Int) = queue.removeUser(offset)

    /** An endless queue keeps the source's order. */
    fun setShuffle(on: Boolean) = queue.setShuffle(on, reorder = infinite == null)

    fun cycleRepeat() = queue.cycleRepeat()

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
    }

    private fun advance(auto: Boolean, play: Boolean = true) {
        val t = queue.advance(manual = !auto)
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
            // A track picked during the fetch owns playback now.
            if (infinite !== source || track !== from) return@launch
            fetching = false
            // A Replay tapped during the fetch wins; that track's next end advances again.
            if (auto && player.playbackState != Player.STATE_ENDED) return@launch
            when {
                more -> advance(auto, play = player.playWhenReady)
                auto -> stopAtEnd()
                // The skip found nothing; the player may hold a stale item, so the track is loaded again, paused.
                else -> from?.let { start(it, play = false) }
            }
        }
    }

    /** End of a finite queue: rewound and paused. */
    private fun stopAtEnd() {
        player.pause()
        player.seekToDefaultPosition()
    }

    private fun refillIfNeeded() {
        val source = infinite ?: return
        if (queue.needsRefill) scope.launch { fetchMore(source) }
    }

    /** Appends a new endless batch, skipping tracks already queued; one fetch at a time. */
    private suspend fun fetchMore(source: Shuffle): Boolean {
        if (infinite !== source) return false
        refilling?.let { return it.await() }
        val job = scope.async {
            // The fresh Shuffle starts on the pages the Discover pool came from, so batches can be all
            // already-queued tracks for a while; only an empty batch means the source is exhausted.
            do {
                val batch = try { source.nextBatch(10u) } catch (e: CancellationException) { throw e } catch (e: Exception) { return@async false }
                if (infinite !== source || batch.isEmpty()) return@async false
            } while (!queue.append(batch.filter { it.streamable }))
            true
        }
        refilling = job
        try {
            return job.await()
        } finally {
            if (refilling === job) refilling = null
        }
    }

    private fun replaceInfinite(source: Shuffle?) {
        val old = infinite
        infinite = source
        val pending = refilling
        pending?.cancel()
        refilling = null
        // A cancelled fetch may still be inside the core; free the source once it returns.
        old?.let { scope.launch { pending?.join(); it.close() } }
    }

    private fun tick() {
        // The player keeps the previous track's item until the new stream URL arrives, or if fetching it failed.
        if (fetching || player.currentMediaItem?.mediaId != track?.id) return
        position = player.currentPosition / 1000.0
        if (player.duration != C.TIME_UNSET && player.duration > 0) duration = player.duration / 1000.0
    }

    private fun start(t: Track, play: Boolean = true) {
        request?.cancel()
        position = 0.0
        duration = t.durationSecs.toDouble()
        fetching = true
        // Stopped, not cleared: an empty player drops the media notification and with it the foreground service,
        // which can't be started again while the app is in the background.
        player.stop()
        player.currentMediaItem?.let { player.replaceMediaItem(player.currentMediaItemIndex, it.buildUpon().setMediaMetadata(metadata(t)).build()) }
        error = null
        // Requested before the fetch so a pause tapped while the URL loads still holds after prepare().
        player.playWhenReady = play
        request = scope.launch {
            try {
                val s = stream(t.id, quality())
                shownQuality = s.actualQuality
                // The core only hands out HLS manifests.
                player.setMediaItem(MediaItem.Builder().setMediaId(t.id).setUri(s.url).setMimeType(MimeTypes.APPLICATION_M3U8).setMediaMetadata(metadata(t)).build())
                player.prepare()
                fetching = false
                refillIfNeeded()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                error = e.message ?: e.javaClass.simpleName
                fetching = false
            }
        }
    }

    /** After a failure or a controller's stop there is nothing prepared, so this starts the track again. */
    fun toggle() {
        val t = track ?: return
        val unprepared = error != null || !fetching && player.playbackState == Player.STATE_IDLE
        when (tapAction(unprepared, player.playbackState == Player.STATE_ENDED, player.playWhenReady)) {
            Tap.Retry -> start(t)
            Tap.Replay -> {
                player.seekToDefaultPosition()
                player.play()
            }
            Tap.Pause -> player.pause()
            Tap.Resume -> player.play()
        }
    }

    fun stop() {
        request?.cancel()
        player.pause()
        player.stop()
        player.clearMediaItems()
        position = 0.0
        duration = 0.0
        queue.clear()
        replaceInfinite(null)
        error = null
        fetching = false
    }
}

private val queueCommands = intArrayOf(
    Player.COMMAND_SEEK_TO_NEXT,
    Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
    Player.COMMAND_SEEK_TO_PREVIOUS,
    Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
)

private fun metadata(t: Track) = MediaMetadata.Builder()
    .setTitle(t.displayTitle)
    .setArtist(t.artistLine)
    .setAlbumTitle(t.album?.title)
    .setArtworkUri(t.album?.coverUrlLarge?.let(Uri::parse))
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
internal fun restartsOnPrevious(position: Double, index: Int) = position > 3 || index == 0

internal enum class Tap { Retry, Replay, Pause, Resume }

/** Pauses on `playWhenReady`, not `isPlaying`, so a tap while buffering still pauses. */
internal fun tapAction(unprepared: Boolean, ended: Boolean, playWhenReady: Boolean) = when {
    unprepared -> Tap.Retry
    ended -> Tap.Replay
    playWhenReady -> Tap.Pause
    else -> Tap.Resume
}
