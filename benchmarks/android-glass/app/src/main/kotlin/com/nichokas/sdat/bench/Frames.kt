package com.nichokas.sdat.bench

import android.os.Handler
import android.os.HandlerThread
import android.view.FrameMetrics
import android.view.Window
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.atomic.AtomicInteger
import kotlin.math.ceil

/** A 60 Hz frame's budget: the target SDAT has to meet on any screen. */
const val BUDGET_60_NS = 16_666_667L

/**
 * One rendered frame from FrameMetrics, in nanoseconds. [uiNs] is the main thread (input, animation, layout, draw),
 * [renderNs] the RenderThread (sync, command issue, swap) and [gpuNs] the GPU, negative where the phone does not report it.
 */
data class Frame(val totalNs: Long, val uiNs: Long, val renderNs: Long, val gpuNs: Long, val deadlineNs: Long)

enum class Limiter { UiThread, RenderThread, Gpu }

data class Stats(
    val frames: Int,
    val fps: Double,
    val p50Ms: Double,
    val p90Ms: Double,
    val p99Ms: Double,
    val maxMs: Double,
    /** Frames that missed this display's own deadline. */
    val jankPercent: Double,
    /** Frames slower than a 60 Hz frame, whatever the display's rate. */
    val over60Percent: Double,
    val uiP90Ms: Double,
    val renderP90Ms: Double,
    /** Null when the phone reports no GPU time; the limiter is then the slower CPU stage. */
    val gpuP90Ms: Double?,
    val lostReports: Int,
) {
    val limiter get() = mapOf(Limiter.UiThread to uiP90Ms, Limiter.RenderThread to renderP90Ms, Limiter.Gpu to (gpuP90Ms ?: -1.0)).maxBy { it.value }.key
}

/** Nearest-rank percentile of an ascending list. */
fun percentile(sorted: List<Long>, p: Double): Long =
    if (sorted.isEmpty()) 0 else sorted[(ceil(p / 100 * sorted.size).toInt() - 1).coerceIn(0, sorted.lastIndex)]

fun stats(frames: List<Frame>, seconds: Double, lostReports: Int = 0): Stats {
    fun p90(of: (Frame) -> Long) = percentile(frames.map(of).sorted(), 90.0) / 1e6
    val gpu = frames.map { it.gpuNs }.filter { it >= 0 }.sorted()
    fun share(test: (Frame) -> Boolean) = if (frames.isEmpty()) 0.0 else 100.0 * frames.count(test) / frames.size
    val total = frames.map { it.totalNs }.sorted()
    return Stats(
        frames = frames.size,
        fps = if (seconds > 0) frames.size / seconds else 0.0,
        p50Ms = percentile(total, 50.0) / 1e6,
        p90Ms = percentile(total, 90.0) / 1e6,
        p99Ms = percentile(total, 99.0) / 1e6,
        maxMs = (total.lastOrNull() ?: 0) / 1e6,
        jankPercent = share { it.totalNs > it.deadlineNs },
        over60Percent = share { it.totalNs > BUDGET_60_NS },
        uiP90Ms = p90 { it.uiNs },
        renderP90Ms = p90 { it.renderNs },
        gpuP90Ms = if (gpu.isEmpty()) null else percentile(gpu, 90.0) / 1e6,
        lostReports = lostReports,
    )
}

/** Every frame the window renders from creation to [close]; FrameMetrics reports on its own thread. */
class FrameRecorder(private val window: Window) : AutoCloseable {
    private val thread = HandlerThread("frame-metrics").apply { start() }
    private val frames = ConcurrentLinkedQueue<Frame>()
    private val lost = AtomicInteger()
    private val listener = Window.OnFrameMetricsAvailableListener { _, m, dropped ->
        lost.addAndGet(dropped)
        if (m.getMetric(FrameMetrics.FIRST_DRAW_FRAME) == 0L) {
            frames += Frame(
                totalNs = m.getMetric(FrameMetrics.TOTAL_DURATION),
                uiNs = m.getMetric(FrameMetrics.INPUT_HANDLING_DURATION) + m.getMetric(FrameMetrics.ANIMATION_DURATION) +
                    m.getMetric(FrameMetrics.LAYOUT_MEASURE_DURATION) + m.getMetric(FrameMetrics.DRAW_DURATION),
                renderNs = m.getMetric(FrameMetrics.SYNC_DURATION) + m.getMetric(FrameMetrics.COMMAND_ISSUE_DURATION) +
                    m.getMetric(FrameMetrics.SWAP_BUFFERS_DURATION),
                gpuNs = m.getMetric(FrameMetrics.GPU_DURATION),
                deadlineNs = m.getMetric(FrameMetrics.DEADLINE),
            )
        }
    }

    init {
        window.addOnFrameMetricsAvailableListener(listener, Handler(thread.looper))
    }

    /** The frames since the last take, over [seconds] of wall time. */
    fun take(seconds: Double): Stats {
        val taken = generateSequence { frames.poll() }.toList()
        return stats(taken, seconds, lost.getAndSet(0))
    }

    override fun close() {
        window.removeOnFrameMetricsAvailableListener(listener)
        thread.quitSafely()
    }
}
