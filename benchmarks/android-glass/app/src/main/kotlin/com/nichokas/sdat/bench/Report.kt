package com.nichokas.sdat.bench

import java.util.Locale

/** One scene shown once: its measured window, and the worst frame of the warm-up before it. */
data class Run(
    val scene: Scene,
    val round: Int,
    val stats: Stats,
    val warmupWorstMs: Double,
    val thermalStart: Int,
    val thermalEnd: Int,
    val imagesLoaded: Int,
    val imagesFailed: Int,
)

data class Device(val name: String, val soc: String, val android: String, val ramGb: Double, val lowRam: Boolean, val refreshHz: Float, val screen: String, val glasmio: String)

private fun Double.f(digits: Int = 1) = String.format(Locale.US, "%.${digits}f", this)

private fun List<Double>.median() = sorted().let { if (it.isEmpty()) 0.0 else if (it.size % 2 == 1) it[it.size / 2] else (it[it.size / 2 - 1] + it[it.size / 2]) / 2 }

private fun List<Double>.spread() = if (size < 2) "" else " (${min().f()}-${max().f()})"

/** Meets 60 fps: at least 57 frames a second, and at most 5% of frames slower than a 60 Hz frame. */
fun meets60(fps: Double, over60: Double, refreshHz: Float) = fps >= minOf(57.0, refreshHz * 0.95) && over60 <= 5.0

private val thermalNames = listOf("none", "light", "moderate", "severe", "critical", "emergency", "shutdown")

fun report(device: Device, runs: List<Run>, rounds: Int): String = buildString {
    appendLine("SDAT glass benchmark")
    appendLine("${device.name} · ${device.soc} · Android ${device.android}")
    appendLine("RAM ${device.ramGb.f()} GB${if (device.lowRam) " (low-RAM device)" else ""} · ${device.screen} · ${device.refreshHz.toDouble().f(0)} Hz")
    appendLine("Glasmio ${device.glasmio} · $rounds round(s), medians with (min-max) across rounds")
    appendLine()
    for ((scene, group) in runs.groupBy { it.scene }) {
        val fps = group.map { it.stats.fps }
        val over = group.map { it.stats.over60Percent }
        val verdict = if (meets60(fps.median(), over.median(), device.refreshHz)) "60 fps OK" else "BELOW 60 fps"
        appendLine("[$verdict] ${scene.label}")
        appendLine("  fps ${fps.median().f()}${fps.spread()} · frames >16.7 ms ${over.median().f()}%${over.spread()} · missed display deadline ${group.map { it.stats.jankPercent }.median().f()}%")
        appendLine("  frame p50 ${group.map { it.stats.p50Ms }.median().f()} · p90 ${group.map { it.stats.p90Ms }.median().f()}${group.map { it.stats.p90Ms }.spread()} · p99 ${group.map { it.stats.p99Ms }.median().f()} · max ${group.maxOf { it.stats.maxMs }.f()} ms")
        val limiters = group.map { it.stats.limiter }.groupingBy { it }.eachCount().maxBy { it.value }.key
        appendLine("  p90 by stage: UI ${group.map { it.stats.uiP90Ms }.median().f()} · RenderThread ${group.map { it.stats.renderP90Ms }.median().f()} · GPU ${group.map { it.stats.gpuP90Ms }.median().f()} ms · limiter $limiters")
        val first = group.minBy { it.round }
        appendLine("  first-use worst frame ${first.warmupWorstMs.f()} ms · images ${group.sumOf { it.imagesLoaded }} loaded, ${group.sumOf { it.imagesFailed }} failed · lost reports ${group.sumOf { it.stats.lostReports }}")
    }
    appendLine()
    val hot = runs.filter { it.thermalEnd >= 2 }
    appendLine(if (hot.isEmpty()) "Thermal: no throttling reported." else "Thermal: ${hot.size} run(s) ended at moderate or worse; later rounds may read slower than the phone is.")
    appendLine()
    appendLine("Runs (round, scene, fps, p90 ms, >16.7 ms %, frames, thermal start-end):")
    for (r in runs) {
        appendLine("  ${r.round} · ${r.scene.label} · ${r.stats.fps.f()} · ${r.stats.p90Ms.f()} · ${r.stats.over60Percent.f()} · ${r.stats.frames} · ${thermalNames.getOrElse(r.thermalStart) { "?" }}-${thermalNames.getOrElse(r.thermalEnd) { "?" }}")
    }
}
