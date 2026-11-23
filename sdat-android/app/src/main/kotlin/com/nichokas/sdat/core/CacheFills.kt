package com.nichokas.sdat.core

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.util.Log
import androidx.media3.common.Player
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import uniffi.tonearm_ffi.CachedTrack
import uniffi.tonearm_ffi.FfiException
import uniffi.tonearm_ffi.Track

/** Fills the audio cache while a track plays: one track at a time, only while the stream has headroom, and given up
 * before the current track ends. */
internal class CacheFills(
    context: Context,
    private val player: Player,
    private val scope: CoroutineScope,
    private val prefs: Preferences,
    private val durationMs: () -> Long,
) {
    private val connectivity = context.getSystemService(ConnectivityManager::class.java)

    /** The track whose fill runs now. One at a time, so a fill never halves another's bandwidth. */
    var filling: String? = null
        private set

    /** Whether a whole track may be downloaded now: a skipped next track or a replay of a streamed one costs data a
     * stream would not, so only on an unmetered network or with downloads over cellular allowed. */
    private fun mayFill() = prefs.downloadOverCellular ||
        connectivity.getNetworkCapabilities(connectivity.activeNetwork)?.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED) == true

    /** Fills [t] at the streaming quality as [fillStep] and [mayFill] allow; null when it gives up or fails. */
    suspend fun inTime(t: Track, blocked: () -> Boolean): CachedTrack? {
        while (true) {
            if (!mayFill()) return null
            val headroom = hasHeadroom(player.playbackState == Player.STATE_READY, player.currentPosition, player.bufferedPosition, durationMs())
            when (fillStep(remainingMs(), headroom, blocked())) {
                FillStep.Start -> break
                FillStep.Wait -> delay(FILL_POLL_MS)
                FillStep.GiveUp -> return null
            }
        }
        filling = t.id
        // Undispatched, the core registers the fill before this returns, so the cancel below always reaches it.
        val fill = scope.async(start = CoroutineStart.UNDISPATCHED) { AudioCache.fill(t, prefs.streamingQuality) }
        var landed = false
        try {
            while (fill.isActive) {
                // A stalled stream needs the bandwidth more than the fill.
                if (player.playbackState == Player.STATE_BUFFERING || !mayFill()) return null
                if (fillStep(remainingMs(), headroom = true, blocked = false) == FillStep.GiveUp) return null
                delay(FILL_POLL_MS)
            }
            return fill.await().also { landed = true }
        } catch (e: CancellationException) {
            throw e
        } catch (_: FfiException.Cancelled) {
            return null
        } catch (e: Exception) {
            Log.w("SDAT/audio-cache", "failed to cache track ${t.id}: $e")
            return null
        } finally {
            if (filling == t.id) filling = null
            // The core's fill runs on its own thread: cancelling the call leaves it downloading.
            if (!landed) {
                AudioCache.cancel(t.id)
                fill.cancel()
            }
        }
    }

    private fun remainingMs() = durationMs() - player.currentPosition
}

internal enum class FillStep { Start, Wait, GiveUp }

/** A fill starts once the current track's stream has headroom and no other fill runs, and gives up when only
 * [FILL_DEADLINE_MS] of the current track remain: the next track then streams as before, in time for a gapless start. */
internal fun fillStep(remainingMs: Long, headroom: Boolean, blocked: Boolean) = when {
    remainingMs <= FILL_DEADLINE_MS -> FillStep.GiveUp
    headroom && !blocked -> FillStep.Start
    else -> FillStep.Wait
}

/** Buffered far enough ahead ([HEADROOM_MS], or to the end) that a fill takes no bandwidth the stream needs. */
internal fun hasHeadroom(ready: Boolean, positionMs: Long, bufferedMs: Long, durationMs: Long) =
    ready && bufferedMs - positionMs >= minOf(HEADROOM_MS, maxOf(durationMs - positionMs - 1_000, 0))

internal const val FILL_DEADLINE_MS = 20_000L
internal const val HEADROOM_MS = 30_000L
private const val FILL_POLL_MS = 500L
