package com.nichokas.sdat.core

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MimeTypes
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.exoplayer.ExoPlayer
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

/** The app's single player: the iOS Player's Discover queue, one track at a time from the core's stream URL. */
class Playback(context: Context, private val quality: Quality) {
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
                if (state == Player.STATE_ENDED) advance()
            }

            override fun onPlayerError(e: PlaybackException) {
                error = e.errorCodeName
            }
        })
    }

    /** Tapping a Discover cover: that song, the rest of the visible pool, then the endless Discover shuffle. */
    fun playDiscover(t: Track, pool: List<Track>, mixes: List<Mix>) {
        queue.discover(t, pool)
        replaceInfinite(if (mixes.isEmpty()) null else Shuffle(mixes))
        start(t)
    }

    private fun advance() {
        val t = queue.next
        if (t != null) {
            queue.index = queue.index!! + 1
            start(t)
            return
        }
        // Endless queue ran dry before the refill landed: fetch, then go on.
        val source = infinite ?: return stopAtEnd()
        fetching = true
        scope.launch {
            val more = fetchMore(source)
            if (infinite !== source) return@launch
            fetching = false
            if (more) advance() else stopAtEnd()
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
            repeat(3) {
                val batch = try { source.nextBatch(10u) } catch (e: CancellationException) { throw e } catch (e: Exception) { return@async false }
                if (infinite !== source) return@async false
                if (queue.append(batch)) return@async true
            }
            false
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
        position = player.currentPosition / 1000.0
        if (player.duration != C.TIME_UNSET && player.duration > 0) duration = player.duration / 1000.0
    }

    private fun start(t: Track) {
        request?.cancel()
        position = 0.0
        duration = t.durationSecs.toDouble()
        player.stop()
        track = t
        error = null
        fetching = true
        request = scope.launch {
            try {
                val url = stream(t.id, quality).url
                // The core only hands out HLS manifests.
                player.setMediaItem(MediaItem.Builder().setUri(url).setMimeType(MimeTypes.APPLICATION_M3U8).build())
                player.prepare()
                player.play()
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

internal fun progress(position: Double, duration: Double) = if (duration > 0) minOf(position / duration, 1.0) else 0.0

internal enum class Tap { Retry, Replay, Pause, Resume }

/** Pauses on `playWhenReady`, not `isPlaying`, so a tap while buffering still pauses. */
internal fun tapAction(failed: Boolean, ended: Boolean, playWhenReady: Boolean) = when {
    failed -> Tap.Retry
    ended -> Tap.Replay
    playWhenReady -> Tap.Pause
    else -> Tap.Resume
}
