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
import java.net.URI
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Quality
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.stream

/** The app's single player: one track at a time from the core's stream URL (no queue yet). */
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

    var track by mutableStateOf<Track?>(null)
        private set
    var playing by mutableStateOf(false)
        private set
    private var fetching by mutableStateOf(false)
    private var buffering by mutableStateOf(false)
    val loading get() = fetching || buffering
    var error by mutableStateOf<String?>(null)
        private set

    init {
        player.addListener(object : Player.Listener {
            override fun onIsPlayingChanged(isPlaying: Boolean) {
                playing = isPlaying
            }

            override fun onPlaybackStateChanged(state: Int) {
                buffering = state == Player.STATE_BUFFERING
            }

            override fun onPlayerError(e: PlaybackException) {
                error = e.errorCodeName
            }
        })
    }

    fun play(t: Track) {
        request?.cancel()
        player.stop()
        track = t
        error = null
        fetching = true
        request = scope.launch {
            try {
                val url = stream(t.id, quality).url
                player.setMediaItem(MediaItem.Builder().setUri(url).setMimeType(mimeType(url)).build())
                player.prepare()
                player.play()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                error = e.message ?: e.javaClass.simpleName
            } finally {
                fetching = false
            }
        }
    }

    /** After a failure there is nothing prepared, so this starts the track again. */
    fun toggle() {
        val t = track ?: return
        when {
            error != null -> play(t)
            player.isPlaying -> player.pause()
            else -> player.play()
        }
    }

    fun stop() {
        request?.cancel()
        player.stop()
        player.clearMediaItems()
        track = null
        error = null
    }
}

/** HLS manifests need an explicit type when the URL carries a query string. */
internal fun mimeType(url: String): String? =
    if (runCatching { URI(url).path }.getOrNull()?.endsWith(".m3u8") == true) MimeTypes.APPLICATION_M3U8 else null
