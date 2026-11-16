package com.nichokas.sdat.bench

import android.os.Handler
import android.os.HandlerThread
import android.view.FrameMetrics
import android.view.Window
import java.util.concurrent.ConcurrentLinkedQueue
import kotlin.math.ceil

/** A 60 Hz frame's budget: the target SDAT has to meet on any screen. */
const val BUDGET_60_NS = 16_666_667L

/** A gap that lost a 60 Hz frame; the margin absorbs vsync jitter on a 60 Hz display. */
private const val LATE_60_NS = BUDGET_60_NS + 2_000_000L

/**
 * One rendered frame from FrameMetrics, in nanoseconds. [uiNs] is the main thread (input, animation, layout, draw),
 * [renderNs] the RenderThread (sync, command issue, swap) and [gpuNs] the GPU; a stage is negative where the phone does
 * not report it.
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
    /**
     * Frames shown more than a 60 Hz frame after the one before, whatever the display's rate. Measured between
     * vsyncs, not by a frame's own duration: a pipelined frame can take longer than 16.7 ms and still keep 120 fps.
     */
    val over60Percent: Double,
    /** Each stage is null when the phone does not report it; the limiter is the slowest stage that is reported. */
    val uiP90Ms: Double?,
    val renderP90Ms: Double?,
    val gpuP90Ms: Double?,
    val lostReports: Int,
) {
    val limiter get() = limiter(uiP90Ms, renderP90Ms, gpuP90Ms)
}

/** The slowest of the stages that were reported, or null when none was. */
fun limiter(ui: Double?, render: Double?, gpu: Double?): Limiter? =
    listOf(Limiter.UiThread to ui, Limiter.RenderThread to render, Limiter.Gpu to gpu).filter { it.second != null }.maxByOrNull { it.second!! }?.first

/** A drawn frame's vsync and the time since the frame before it, reported or rebuilt from a lost report. */
private data class Tick(val vsyncNs: Long, val intervalNs: Long)

/**
 * Every drawn frame, reported or not. A report says how many reports were lost since the one before it, so those
 * frames lie between the two vsyncs and are rebuilt evenly across that gap. The first report has no previous vsync,
 * so reports lost before it cannot be placed and are left out.
 */
private fun ticks(frames: List<Frame>): List<Tick> = frames.flatMap { f ->
    if (f.intervalNs <= 0) listOf(Tick(f.vsyncNs, 0))
    else (f.dropped downTo 0).map { k -> Tick(f.vsyncNs - f.intervalNs / (f.dropped + 1) * k, f.intervalNs / (f.dropped + 1)) }
}

/** Nearest-rank percentile of an ascending list. */
fun percentile(sorted: List<Long>, p: Double): Long =
    if (sorted.isEmpty()) 0 else sorted[(ceil(p / 100 * sorted.size).toInt() - 1).coerceIn(0, sorted.lastIndex)]

/**
 * The window [start, end) of [frames], cut by vsync time: durations come from the reports in it, the rate and the
 * time between frames from every frame drawn in it, lost reports included.
 */
fun stats(frames: List<Frame>, seconds: Double, start: Long = Long.MIN_VALUE, end: Long = Long.MAX_VALUE): Stats {
    val ticks = ticks(frames).filter { it.vsyncNs in start until end }
    @Suppress("NAME_SHADOWING") val frames = frames.filter { it.vsyncNs in start until end }
    fun stage(of: (Frame) -> Long) = frames.map(of).filter { it >= 0 }.sorted().let { if (it.isEmpty()) null else percentile(it, 90.0) / 1e6 }
    val intervals = ticks.map { it.intervalNs }.filter { it > 0 }.sorted()
    fun share(test: (Frame) -> Boolean) = if (frames.isEmpty()) 0.0 else 100.0 * frames.count(test) / frames.size
    val total = frames.map { it.totalNs }.sorted()
    return Stats(
        frames = frames.size,
        fps = if (seconds > 0) ticks.size / seconds else 0.0,
        p50Ms = percentile(total, 50.0) / 1e6,
        p90Ms = percentile(total, 90.0) / 1e6,
        p99Ms = percentile(total, 99.0) / 1e6,
        p99IntervalMs = percentile(intervals, 99.0) / 1e6,
        maxMs = (total.lastOrNull() ?: 0) / 1e6,
        jankPercent = share { it.totalNs > it.deadlineNs },
        over60Percent = if (intervals.isEmpty()) 0.0 else 100.0 * intervals.count { it > LATE_60_NS } / intervals.size,
        uiP90Ms = stage { it.uiNs },
        renderP90Ms = stage { it.renderNs },
        gpuP90Ms = stage { it.gpuNs },
        lostReports = ticks.size - frames.size,
    )
}

/** The stage's total, or -1 when any part of it is unreported (FrameMetrics' -1). */
private fun FrameMetrics.sum(vararg ids: Int): Long = ids.map { getMetric(it) }.let { if (it.any { v -> v < 0 }) -1 else it.sum() }

/** Every frame the window renders from creation to [close]; FrameMetrics reports on its own thread, a little late. */
class FrameRecorder(private val window: Window) : AutoCloseable {
    private val thread = HandlerThread("frame-metrics").apply { start() }
    private val frames = ConcurrentLinkedQueue<Frame>()
    private var lastVsync = 0L // Only touched on the metrics thread.

    // Every report is kept, so each interval and its lost reports run back to the report just before it. The window's
    // first draw happened long before the recorder, which starts once the covers are ready.
    private val listener = Window.OnFrameMetricsAvailableListener { _, m, dropped ->
        val vsync = m.getMetric(FrameMetrics.INTENDED_VSYNC_TIMESTAMP)
        val interval = if (lastVsync > 0) vsync - lastVsync else 0
        lastVsync = vsync
        frames += Frame(
            totalNs = m.getMetric(FrameMetrics.TOTAL_DURATION),
            // UNKNOWN_DELAY is the wait for a busy main thread before the frame could start.
            uiNs = m.sum(FrameMetrics.UNKNOWN_DELAY_DURATION, FrameMetrics.INPUT_HANDLING_DURATION, FrameMetrics.ANIMATION_DURATION, FrameMetrics.LAYOUT_MEASURE_DURATION, FrameMetrics.DRAW_DURATION),
            renderNs = m.sum(FrameMetrics.SYNC_DURATION, FrameMetrics.COMMAND_ISSUE_DURATION, FrameMetrics.SWAP_BUFFERS_DURATION),
            gpuNs = m.getMetric(FrameMetrics.GPU_DURATION),
            deadlineNs = m.getMetric(FrameMetrics.DEADLINE),
            vsyncNs = vsync,
            intervalNs = interval,
            dropped = dropped,
        )
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
