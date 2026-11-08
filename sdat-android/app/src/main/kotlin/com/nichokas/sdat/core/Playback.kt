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
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.stream

/** The app's single player: the iOS Player's Discover queue, one track at a time from the core's stream URL.
 * Reads [quality] per request so a change in Settings applies to the next track. */
// ForwardingSimpleBasePlayer, which the session player is built on, is still marked unstable.
@OptIn(UnstableApi::class)
class Playback(context: Context, private val quality: () -> Quality) {
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

    var track by mutableStateOf<Track?>(null)
        private set
    var playing by mutableStateOf(false)
        private set
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

        // The session prepares a stopped player when play is pressed. Stopped means the stream failed, and the
        // item left in the player may still be the previous track's stream, so start the current track afresh.
        override fun handlePrepare(): ListenableFuture<*> {
            if (!fetching) track?.let { start(it, play = player.playWhenReady) }
            return Futures.immediateVoidFuture()
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

    /** Tapping a Discover cover: that song, the rest of the visible pool, then the endless Discover shuffle. */
    fun playDiscover(t: Track, pool: List<Track>, mixes: List<Mix>) {
        queue.discover(t, pool.filter { it.streamable })
        replaceInfinite(Shuffle(mixes))
        start(t)
    }

    fun next() {
        if (track != null) advance(auto = false)
    }

    fun previous() {
        val i = queue.index ?: return
        if (restartsOnPrevious(position, i)) {
            player.seekTo(0)
        } else {
            queue.index = i - 1
            queue.current?.let { start(it) }
        }
    }

    private fun advance(auto: Boolean, play: Boolean = true) {
        val t = queue.next
        if (t != null) {
            queue.index = queue.index!! + 1
            start(t, play)
            return
        }
        // Endless queue ran dry before the refill landed: fetch, then go on.
        val source = infinite ?: return stopAtEnd()
        val from = track
        val wasPlaying = player.playWhenReady
        fetching = true
        scope.launch {
            val more = fetchMore(source)
            // A track picked during the fetch owns playback now.
            if (infinite !== source || track !== from) return@launch
            fetching = false
            // A Replay tapped during the fetch wins; that track's next end advances again.
            if (auto && player.playbackState != Player.STATE_ENDED) return@launch
            // A pause tapped during the fetch holds; Next from a paused track still plays, as it does without a fetch.
            if (more) advance(auto, play = player.playWhenReady || !wasPlaying) else stopAtEnd()
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
        track = t
        error = null
        // Requested before the fetch so a pause tapped while the URL loads still holds after prepare().
        player.playWhenReady = play
        request = scope.launch {
            try {
                val url = stream(t.id, quality()).url
                // The core only hands out HLS manifests.
                player.setMediaItem(MediaItem.Builder().setMediaId(t.id).setUri(url).setMimeType(MimeTypes.APPLICATION_M3U8).setMediaMetadata(metadata(t)).build())
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

    /** After a failure there is nothing prepared, so this starts the track again. */
    fun toggle() {
        val t = track ?: return
        when (tapAction(error != null, player.playbackState == Player.STATE_ENDED, player.playWhenReady)) {
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
        track = null
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

internal fun progress(position: Double, duration: Double) = if (duration > 0) minOf(position / duration, 1.0) else 0.0

/** Like iOS: past 3 s, or on the first track, previous goes back to its start; otherwise to the track before. */
internal fun restartsOnPrevious(position: Double, index: Int) = position > 3 || index == 0

internal enum class Tap { Retry, Replay, Pause, Resume }

/** Pauses on `playWhenReady`, not `isPlaying`, so a tap while buffering still pauses. */
internal fun tapAction(failed: Boolean, ended: Boolean, playWhenReady: Boolean) = when {
    failed -> Tap.Retry
    ended -> Tap.Replay
    playWhenReady -> Tap.Pause
    else -> Tap.Resume
}
