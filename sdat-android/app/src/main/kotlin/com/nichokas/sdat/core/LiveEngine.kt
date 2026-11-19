package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.math.abs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Track

internal interface LivePlaybackPort {
    val currentId: String?
    val engineId: String?
    val requested: Boolean
    val advancing: Boolean
    val logicalPlaying: Boolean
    val loading: Boolean
    val fetching: Boolean
    val hasError: Boolean
    val suppressed: Boolean
    val idle: Boolean
    val ended: Boolean
    val ready: Boolean
    val failed: Boolean
    val positionMs: Long
    val logicalPositionMs: Long
    val durationMs: Long
    val bufferedMs: Long

    fun playEngine()
    fun pauseEngine()
    fun seekEngine(ms: Long)
    fun setLogicalPosition(ms: Long)
    fun setLogicalPlaying(playing: Boolean)
    fun requestPause()
    fun seek(ms: Long)
    fun toggle()
    fun load(track: Track, atMs: Long, playing: Boolean)
    fun followingChanged()
}

internal class LiveEngine(private val port: LivePlaybackPort, private val scope: CoroutineScope, private val uptime: () -> Long) : LiveFollower {
    var liveFollowing = false
        set(on) {
            field = on
            port.followingChanged()
        }
    var liveTarget: (() -> HostAt?)? = null
    override var liveSeeking = false
        private set
    override var liveSettledAt: Long? = null
        private set
    override var systemPaused = false
        private set
    var aligning by mutableStateOf(false)
        private set
    private var liveJob: Job? = null
    private var liveResumePending = false
    private var loadResumeAtMs: Long? = null

    override val currentId get() = port.currentId
    override val isPlaying get() = port.logicalPlaying
    override val loading get() = port.loading
    override val livePositionMs get() = port.positionMs
    override val liveHasItem get() = !port.ended && (port.fetching || !port.idle)
    override val liveAdvancing get() = port.advancing
    val livePublishedPositionMs get() =
        loadResumeAtMs ?: if (port.engineId == currentId) port.positionMs else port.logicalPositionMs
    val livePublishedPlaying get() = isPlaying && !loading && !port.suppressed && loadResumeAtMs == null && !port.hasError

    override fun livePlay(track: Track, atMs: Long, playing: Boolean) = port.load(track, atMs, playing)
    override fun seekTo(ms: Long) = port.seek(ms)
    override fun pause() = port.requestPause()

    fun clearSystemPause() { systemPaused = false }

    fun playRequestChanged(playWhenReady: Boolean, system: Boolean) {
        if (playWhenReady) systemPaused = false else if (system) systemPaused = true
    }

    fun shouldAlign(playing: Boolean) = liveFollowing && playing && liveTarget != null

    fun loadingFrom(atMs: Long) {
        liveSettledAt = null
        loadResumeAtMs = atMs
    }

    fun clearLoadPosition() { loadResumeAtMs = null }

    fun prepareAlignment(on: Boolean) { aligning = on }

    fun startAlignment() {
        if (!aligning) return
        liveJob = scope.launch {
            liveAlign()
            aligning = false
            liveSettledAt = uptime()
        }
    }

    fun dropLiveStart(): Boolean {
        if (!liveResumePending && !aligning) return false
        cancelLive()
        port.pauseEngine()
        syncPlaying()
        return true
    }

    fun syncPlaying() {
        port.setLogicalPlaying(port.requested && !port.ended || liveResumePending)
    }

    fun cancelLive() {
        liveJob?.cancel()
        liveJob = null
        liveSeeking = false
        liveResumePending = false
        aligning = false
    }

    fun liveEnded() {
        liveFollowing = false
        liveTarget = null
        val waiting = liveResumePending || aligning
        cancelLive()
        if (waiting) port.pauseEngine()
        syncPlaying()
    }

    /**
     * Large drift correction: a plain seek to where the host is, while playing. The destination is re-read when the
     * seek starts, and a slow seek that still lands behind is followed by one more aiming ahead by the time the first
     * took (capped), so it does not leave an error that triggers another seek. The speed is never changed.
     */
    override fun liveSeek(ms: Long) {
        if (liveSeeking || currentId == null) return
        liveSeeking = true
        liveJob = scope.launch {
            var lead = 0L
            for (attempt in 0 until 2) {
                val dest = maxOf((liveTarget?.invoke()?.ms ?: ms) + lead, 0)
                val started = uptime()
                port.seekEngine(dest)
                port.setLogicalPosition(dest)
                awaitPlaying()
                val host = liveTarget?.invoke()
                if (host == null || !host.playing || host.ms - port.positionMs <= LATE_TOLERANCE_MS) break
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
        while (!port.advancing && port.requested && uptime() < deadline) delay(20)
    }

    /**
     * The host resumed after a pause: start in step with it. Too far from it (more than the buffer kept while paused
     * covers) it joins like a fresh start instead: playing from the old spot would be the wrong part of the song.
     */
    override fun liveResume() {
        if (isPlaying) return
        val host = liveTarget?.invoke()
        val unprepared = port.hasError || port.idle || port.ended
        if (host == null || !host.playing || unprepared) {
            port.toggle()
            return
        }
        val far = abs(port.positionMs - host.ms) > RESUME_WINDOW_MS
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
            port.pauseEngine()
            val t = liveTarget?.invoke() ?: return false
            if (!t.playing) {
                port.seekEngine(t.ms)
                return false
            }
            val duration = port.durationMs
            val startAt = (t.ms + JOIN_LEAD_MS).let { if (duration > 0) minOf(it, maxOf(duration - 1_000, t.ms)) else it }
            port.seekEngine(startAt)
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
                port.seekEngine(now.ms)
                return false
            }
            if (now.ms - startAt > LATE_TOLERANCE_MS) return@attempt
            if (systemPaused) return false
            port.playEngine()
            return true
        }
        // Very slow network: start where the host is and let the player buffer.
        val t = liveTarget?.invoke() ?: return false
        port.seekEngine(t.ms)
        if (!t.playing || systemPaused) return false
        port.playEngine()
        return true
    }

    /** Plays once the host reaches the paused position; a host already past it is followed at once (within tolerance). */
    private suspend fun startInSync() {
        val host = liveTarget?.invoke() ?: return
        val startAt = uptime() + port.positionMs - host.ms
        while (true) {
            val now = liveTarget?.invoke()
            if (now == null || !now.playing || systemPaused) return
            val left = startAt - uptime()
            if (left <= 0) break
            delay(minOf(50, left))
        }
        port.playEngine()
    }

    /** Waits until [MIN_BUFFER_MS] is loaded after [fromMs] (or up to the end of the track), bounded. */
    private suspend fun awaitBuffer(fromMs: Long) {
        val deadline = uptime() + BUFFER_TIMEOUT_MS
        while (uptime() < deadline && !port.failed) {
            val duration = port.durationMs
            val need = if (duration > 0) minOf(MIN_BUFFER_MS, maxOf(duration - fromMs - 500, 0)) else MIN_BUFFER_MS
            if (port.ready && port.bufferedMs - fromMs >= need) return
            delay(100)
        }
    }
}

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
