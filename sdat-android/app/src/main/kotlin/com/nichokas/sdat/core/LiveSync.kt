package com.nichokas.sdat.core

import kotlin.math.abs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.FfiException
import uniffi.tonearm_ffi.LiveCorrection
import uniffi.tonearm_ffi.LiveEnd
import uniffi.tonearm_ffi.LiveGuestInterface
import uniffi.tonearm_ffi.LiveHostInterface
import uniffi.tonearm_ffi.LivePath
import uniffi.tonearm_ffi.LiveUpdate
import uniffi.tonearm_ffi.Track

/** iOS Live.Mode. */
sealed interface LiveMode {
    data object Idle : LiveMode
    data object Starting : LiveMode
    data object Hosting : LiveMode
    data object Joining : LiveMode
    data object Joined : LiveMode
    data class Ended(val message: String) : LiveMode

    val active get() = this == Starting || this == Hosting || this == Joining || this == Joined
}

internal fun LiveMode.after(update: LiveUpdate): LiveMode = when (update) {
    is LiveUpdate.Joined -> if (this == LiveMode.Joining) LiveMode.Joined else this
    is LiveUpdate.Ended -> LiveMode.Ended(endMessage(update.reason))
    else -> this
}

internal fun endMessage(reason: LiveEnd) = when (reason) {
    LiveEnd.HOST_ENDED -> "The host ended the session."
    LiveEnd.HOST_LOST -> "Lost connection to the host. The session ended."
    LiveEnd.LEFT -> "You left the session."
    LiveEnd.BAD_INVITE -> "This invitation isn't valid."
    LiveEnd.INCOMPATIBLE -> "The host is using a different version of SDAT. Update both apps."
    LiveEnd.FULL -> "This session is full."
}

/** Where the host is for the track loaded here, manual delay included. */
data class HostAt(val ms: Long, val playing: Boolean)

/**
 * What a Listen Together guest needs from the player. There is deliberately no way to change the playback speed:
 * drift is only ever corrected with a seek.
 */
interface LiveFollower {
    val currentId: String?
    /** Playing, or about to: the requested state. */
    val isPlaying: Boolean
    val loading: Boolean
    /** False once the local copy of the track ended, so the host's next anchor reloads it even for the same song. */
    val liveHasItem: Boolean
    /** A drift seek or a synchronized resume is running: the position is not trustworthy. */
    val liveSeeking: Boolean
    /** Audio really advances on a ready buffer: only then is the position worth measuring. */
    val liveAdvancing: Boolean
    /** Paused by the system (headphones unplugged, audio focus lost), not by the user. */
    val systemPaused: Boolean
    /** Uptime (ms) when the last start or drift seek finished. */
    val liveSettledAt: Long?
    /** What is heard now; the player already accounts for the audio output latency. */
    val livePositionMs: Long

    /** Plays the host's track at [atMs]; when [playing], starts in step with the host. */
    fun livePlay(track: Track, atMs: Long, playing: Boolean)
    fun liveResume()
    fun liveSeek(ms: Long)
    fun pause()
    fun seekTo(ms: Long)
}

/** Host: publishes the shared queue and playback anchors (iOS hostTick, plus desktop's 5 s refresh). */
internal class HostPublisher(private val host: LiveHostInterface, private val now: () -> Long) {
    private var sentIds: List<String>? = null
    private var itemIds = emptyList<String>()
    private var sent: Anchor? = null

    private data class Anchor(val item: String?, val playing: Boolean, val ms: Long, val at: Long)

    /** [queue]: the playing track first (when there is one), then everything queued after it. */
    fun publish(current: String?, queue: List<String>, playing: Boolean, positionMs: Long) {
        val ids = queue.take(MAX_SHARED)
        var force = false
        if (ids != sentIds) {
            try {
                itemIds = host.setQueue(ids)
                sentIds = ids
                force = true
            } catch (_: FfiException) {
                // Sent again on the next tick.
            }
        }
        val item = if (current == null) null else itemIds.firstOrNull()
        val at = now()
        val s = sent
        if (!force && s != null && s.item == item && s.playing == playing && at - s.at < REFRESH_MS) {
            val expected = s.ms + if (playing) at - s.at else 0
            if (abs(expected - positionMs) < JUMP_MS) return
        }
        host.setPlayback(item, current, positionMs, playing)
        sent = Anchor(item, playing, positionMs, at)
    }

    companion object {
        const val MAX_SHARED = 500
        const val REFRESH_MS = 5_000L
        /** Off by more than this from where the last anchor puts the host: it seeked or stalled. */
        const val JUMP_MS = 250L
    }
}

/**
 * Guest: follows the host's track and position (iOS guestTick). The core decides drift corrections; this only seeks,
 * and only with the user's manual delay as latency: the player's position already is what is heard.
 */
internal class GuestFollower(
    private val guest: LiveGuestInterface,
    private val player: LiveFollower,
    private val scope: CoroutineScope,
    private val resolve: suspend (String) -> Track?,
    private val latencyMs: () -> Long,
    private val now: () -> Long,
    private val log: (String) -> Unit,
) {
    private var switching: String? = null
    /** Bumped per lookup: the host may go A, B, A while the first lookup of A is still running. */
    private var lookup = 0
    /** The last track handed to the player, and when: a load that keeps failing is retried only every [RELOAD_MS]. */
    private var loaded: Pair<String, Long>? = null
    /** Latency the playing position was last aligned with. */
    private var alignedLatencyMs: Long? = null
    // Uptime is never negative, so the first drift log is due at once; MIN_VALUE would overflow the subtraction.
    private var lastDriftLogAt = -10_000L
    private var lastDiagAt = Long.MIN_VALUE
    private var lastPath: LivePath? = null

    fun tick() {
        logDiagnostics()
        val lat = latencyMs()
        val t = guest.target(lat) ?: return
        val trackId = t.trackId ?: run {
            if (player.isPlaying) player.pause()
            return
        }
        // Reload also when our copy ended (host repeating the same song).
        if (player.currentId != trackId || !player.liveHasItem && !player.loading) {
            if (switching == trackId) return
            val at = now()
            loaded?.let { (id, time) -> if (id == trackId && at - time < RELOAD_MS) return }
            // The guest must not keep playing a song the host left, also while (or if) the lookup fails.
            if (player.currentId != trackId && player.isPlaying) player.pause()
            switching = trackId
            val op = ++lookup
            scope.launch {
                try {
                    val track = resolve(trackId)
                    ensureActive()
                    if (op != lookup) return@launch
                    // Re-read: the host may have moved on while resolving.
                    val at = guest.target(latencyMs())?.takeIf { it.trackId == trackId } ?: return@launch
                    // A system pause also holds a new host track: it loads paused until the user or system resumes.
                    if (track != null) {
                        loaded = trackId to now()
                        player.livePlay(track, at.positionMs, at.playing && !player.systemPaused)
                    }
                } finally {
                    // A stale lookup must not clear the marker of the newer one for the same track.
                    if (op == lookup) switching = null
                }
            }
            return
        }
        // The host came back to the loaded track: a lookup still running for another one is stale.
        if (switching != null) {
            switching = null
            lookup++
        }
        // Mid-seek or rebuffering the position is frozen: the error would look huge and cause seek after seek.
        if (player.loading || player.liveSeeking) return
        // Paused by the system: wait for the user or the system to resume instead of restarting on the speaker.
        if (t.playing && !player.isPlaying && player.systemPaused) return
        if (t.playing != player.isPlaying) {
            if (t.playing) {
                player.liveResume()
            } else {
                player.pause()
                player.seekTo(t.positionMs)
            }
            return
        }
        if (!t.playing) {
            if (abs(player.livePositionMs - t.positionMs) > PAUSED_SEEK_MS) player.seekTo(t.positionMs)
            return
        }
        if (!player.liveAdvancing) return
        val at = now()
        player.liveSettledAt?.let { if (at - it < SEEK_SETTLE_MS) return }
        val actual = player.livePositionMs
        // A change of the manual delay stays below the core's tolerance: realign once instead of never applying it.
        val previous = alignedLatencyMs
        if (previous == null) alignedLatencyMs = lat
        if (previous != null && abs(lat - previous) >= LATENCY_REALIGN_MS) {
            alignedLatencyMs = lat
            log("latency changed $previous->$lat ms, realigning")
            player.liveSeek(t.positionMs)
            return
        }
        if (at - lastDriftLogAt >= 10_000) {
            lastDriftLogAt = at
            log("drift errMs=${t.positionMs - actual}")
        }
        // The core ignores errors up to a few seconds, spaces its corrections and follows host seeks at once. A
        // Rate answer is never acted on.
        val seek = guest.correction(lat, actual) as? LiveCorrection.Seek ?: return
        log("drift seek errMs=${seek.positionMs - actual}")
        alignedLatencyMs = lat
        player.liveSeek(seek.positionMs)
    }

    /** The host's position for the track loaded here, or null when it is on another track. */
    fun hostPosition(): HostAt? {
        val lat = latencyMs()
        val t = guest.target(lat) ?: return null
        if (t.trackId == null || t.trackId != player.currentId) return null
        // The player aligns to this answer (start, resume): it is the latency the position now matches.
        alignedLatencyMs = lat
        return HostAt(t.positionMs, t.playing)
    }

    /** Logs the network path when it changes and every 10 s, so a slow same-network session can be told from a relay. */
    private fun logDiagnostics() {
        val at = now()
        val d = guest.diagnostics()
        if (d.path == lastPath && at - lastDiagAt < 10_000) return
        lastPath = d.path
        lastDiagAt = at
        log("path=${d.path} pathRttMs=${d.pathRttMs} clockOffsetMs=${d.clockOffsetMs} clockRttMs=${d.clockRttMs} latencyMs=${latencyMs()}")
    }

    companion object {
        /** A paused guest further than this from the host is moved there (a paused seek is inaudible). */
        const val PAUSED_SEEK_MS = 1_000L
        /** Quiet time after a start or a drift seek before measuring again. */
        const val SEEK_SETTLE_MS = 3_000L
        /** Manual delay change that triggers one realigning seek; smaller ones are jitter. */
        const val LATENCY_REALIGN_MS = 300L
        /** Shorter than any song, so a host repeating one still reloads it as soon as our copy ends. */
        const val RELOAD_MS = 5_000L
    }
}
