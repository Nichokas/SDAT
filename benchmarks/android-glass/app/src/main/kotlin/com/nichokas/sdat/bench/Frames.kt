package com.nichokas.sdat.bench

import android.os.Handler
import android.os.HandlerThread
import android.view.FrameMetrics
import android.view.Window
import java.util.concurrent.ConcurrentLinkedQueue
import kotlin.math.ceil

/** A 60 Hz frame's budget: the target SDAT has to meet on any screen. */
const val BUDGET_60_NS = 16_666_667L

/**
 * One rendered frame from FrameMetrics, in nanoseconds. [uiNs] is the main thread (input, animation, layout, draw),
 * [renderNs] the RenderThread (sync, command issue, swap) and [gpuNs] the GPU.
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
    val gpuP90Ms: Double,
    val lostReports: Int,
) {
    val limiter get() = mapOf(Limiter.UiThread to uiP90Ms, Limiter.RenderThread to renderP90Ms, Limiter.Gpu to gpuP90Ms).maxBy { it.value }.key
}

/** Nearest-rank percentile of an ascending list. */
fun percentile(sorted: List<Long>, p: Double): Long =
    if (sorted.isEmpty()) 0 else sorted[(ceil(p / 100 * sorted.size).toInt() - 1).coerceIn(0, sorted.lastIndex)]

fun stats(frames: List<Frame>, seconds: Double, lostReports: Int = 0): Stats {
    fun p90(of: (Frame) -> Long) = percentile(frames.map(of).sorted(), 90.0) / 1e6
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
        gpuP90Ms = p90 { it.gpuNs },
        lostReports = lostReports,
    )
}

/** Every frame the window renders between [begin] and [end]; FrameMetrics reports on its own thread. */
class FrameRecorder(private val window: Window) {
    private val thread = HandlerThread("frame-metrics").apply { start() }
    private val frames = ConcurrentLinkedQueue<Frame>()
    @Volatile private var lost = 0
    private val listener = Window.OnFrameMetricsAvailableListener { _, m, dropped ->
        lost += dropped
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

    fun begin() {
        take(0.0)
        window.addOnFrameMetricsAvailableListener(listener, Handler(thread.looper))
    }

    /** The frames since the last take, over [seconds] of wall time. */
    fun take(seconds: Double): Stats {
        val taken = generateSequence { frames.poll() }.toList()
        val dropped = lost.also { lost = 0 }
        return stats(taken, seconds, dropped)
    }

    fun end() = window.removeOnFrameMetricsAvailableListener(listener)
}
