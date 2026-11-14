package com.nichokas.sdat.core

import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.tonearm_ffi.ScrobbleTrack
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.lastfmNowPlaying
import uniffi.tonearm_ffi.lastfmScrobble
import uniffi.tonearm_ffi.listenbrainzNowPlaying
import uniffi.tonearm_ffi.listenbrainzScrobble
import uniffi.tonearm_ffi.shouldScrobble

/** One play of [track] as iOS counts it: only playback heard in step adds up, so a seek, a stall or a pause adds
 * nothing, and [counts] (the core's rule) decides once when the play becomes a scrobble. */
internal class Listen(
    val track: Track,
    /** Unix seconds. */
    val startedAt: Long,
    private val counts: (listenedSecs: ULong, durationSecs: UInt) -> Boolean = ::shouldScrobble,
) {
    private var listened = 0.0
    private var scrobbled = false

    /** The player moved from [from] to [to] seconds; true on the one call that makes the play count. */
    fun heard(from: Double, to: Double, playing: Boolean): Boolean {
        if (playing && to > from && to - from < 2) listened += to - from
        if (scrobbled || !counts(listened.toULong(), track.durationSecs)) return false
        scrobbled = true
        return true
    }
}

internal fun scrobbleTrack(t: Track) = ScrobbleTrack(
    title = t.displayTitle,
    artist = t.artists.firstOrNull()?.name ?: "",
    album = t.album?.title ?: "",
    url = "https://tidal.com/track/${t.id}",
    durationSecs = t.durationSecs,
)

/** Last.fm and ListenBrainz, each when it is on in Settings and connected. Failures are dropped, as on iOS. */
internal class Scrobbler(private val prefs: Preferences, private val secrets: Secrets) {
    private val scope = MainScope()

    fun nowPlaying(t: Track) {
        val st = scrobbleTrack(t)
        send(prefs.scrobbleLastFm, Secrets.LASTFM_SESSION) { lastfmNowPlaying(it, st) }
        send(prefs.scrobbleListenBrainz, Secrets.LISTENBRAINZ_TOKEN) { listenbrainzNowPlaying(it, st) }
    }

    fun scrobble(l: Listen) {
        val st = scrobbleTrack(l.track)
        send(prefs.scrobbleLastFm, Secrets.LASTFM_SESSION) { lastfmScrobble(it, st, l.startedAt) }
        send(prefs.scrobbleListenBrainz, Secrets.LISTENBRAINZ_TOKEN) { listenbrainzScrobble(it, st, l.startedAt) }
    }

    // The key is read off the main thread: each read is a Keystore decrypt.
    private fun send(on: Boolean, key: String, call: suspend (String) -> Unit) {
        if (!on) return
        scope.launch {
            try {
                withContext(Dispatchers.IO) { secrets.get(key) }?.let { call(it) }
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
            }
        }
    }
}
