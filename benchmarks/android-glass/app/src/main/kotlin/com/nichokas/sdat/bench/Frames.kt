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
 * [renderNs] the RenderThread (sync, command issue, swap) and [gpuNs] the GPU, negative where the phone does not report it.
 * [vsyncNs] is its intended vsync on [System.nanoTime]'s clock. [intervalNs] is the time since the previous reported
 * frame's vsync, so it includes skipped vsyncs, and spans [dropped] frames whose reports were lost before this one.
 */
data class Frame(
    val totalNs: Long,
    val uiNs: Long,
    val renderNs: Long,
    val gpuNs: Long,
    val deadlineNs: Long,
    val vsyncNs: Long = 0,
    val intervalNs: Long = 0,
    val dropped: Int = 0,
)

enum class Limiter { UiThread, RenderThread, Gpu }

data class Stats(
    val frames: Int,
    val fps: Double,
    val p50Ms: Double,
    val p90Ms: Double,
    val p99Ms: Double,
    /** The 99th percentile of the time between frames: what the user sees in the slowest 1% of the run. */
    val p99IntervalMs: Double,
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

fun stats(frames: List<Frame>, seconds: Double): Stats {
    val lostReports = frames.sumOf { it.dropped }
    // An interval that spans lost reports is shared evenly by the frames in it, so a gap is neither dropped nor
    // charged to one frame.
    val intervals = frames.filter { it.intervalNs > 0 }.flatMap { f -> List(f.dropped + 1) { f.intervalNs / (f.dropped + 1) } }.sorted()
    fun p90(of: (Frame) -> Long) = percentile(frames.map(of).sorted(), 90.0) / 1e6
    val gpu = frames.map { it.gpuNs }.filter { it >= 0 }.sorted()
    fun share(test: (Frame) -> Boolean) = if (frames.isEmpty()) 0.0 else 100.0 * frames.count(test) / frames.size
    val total = frames.map { it.totalNs }.sorted()
    return Stats(
        frames = frames.size,
        // A dropped report is a frame that was drawn but not reported, so it still counts toward the rate.
        fps = if (seconds > 0) (frames.size + lostReports) / seconds else 0.0,
        p50Ms = percentile(total, 50.0) / 1e6,
        p90Ms = percentile(total, 90.0) / 1e6,
        p99Ms = percentile(total, 99.0) / 1e6,
        p99IntervalMs = percentile(intervals, 99.0) / 1e6,
        maxMs = (total.lastOrNull() ?: 0) / 1e6,
        jankPercent = share { it.totalNs > it.deadlineNs },
        over60Percent = share { it.totalNs > BUDGET_60_NS },
        uiP90Ms = p90 { it.uiNs },
        renderP90Ms = p90 { it.renderNs },
        gpuP90Ms = if (gpu.isEmpty()) null else percentile(gpu, 90.0) / 1e6,
        lostReports = lostReports,
    )
}

/** Every frame the window renders from creation to [close]; FrameMetrics reports on its own thread, a little late. */
class FrameRecorder(private val window: Window) : AutoCloseable {
    private val thread = HandlerThread("frame-metrics").apply { start() }
    private val frames = ConcurrentLinkedQueue<Frame>()
    // Only touched on the metrics thread.
    private var lastVsync = 0L
    private var pendingDropped = 0
    private val listener = Window.OnFrameMetricsAvailableListener { _, m, dropped ->
        val vsync = m.getMetric(FrameMetrics.INTENDED_VSYNC_TIMESTAMP)
        val interval = if (lastVsync > 0) vsync - lastVsync else 0
        lastVsync = vsync
        pendingDropped += dropped
        if (m.getMetric(FrameMetrics.FIRST_DRAW_FRAME) == 0L) {
            frames += Frame(
                totalNs = m.getMetric(FrameMetrics.TOTAL_DURATION),
                uiNs = m.getMetric(FrameMetrics.INPUT_HANDLING_DURATION) + m.getMetric(FrameMetrics.ANIMATION_DURATION) +
                    m.getMetric(FrameMetrics.LAYOUT_MEASURE_DURATION) + m.getMetric(FrameMetrics.DRAW_DURATION),
                renderNs = m.getMetric(FrameMetrics.SYNC_DURATION) + m.getMetric(FrameMetrics.COMMAND_ISSUE_DURATION) +
                    m.getMetric(FrameMetrics.SWAP_BUFFERS_DURATION),
                gpuNs = m.getMetric(FrameMetrics.GPU_DURATION),
                deadlineNs = m.getMetric(FrameMetrics.DEADLINE),
                vsyncNs = vsync,
                intervalNs = interval,
                dropped = pendingDropped,
            )
            pendingDropped = 0
        }
    }

    init {
        window.addOnFrameMetricsAvailableListener(listener, Handler(thread.looper))
    }

    /** The frames reported since the last drain; callers pick a window by [Frame.vsyncNs]. */
    fun drain(): List<Frame> = generateSequence { frames.poll() }.toList()

    override fun close() {
        window.removeOnFrameMetricsAvailableListener(listener)
        thread.quitSafely()
    }
}
