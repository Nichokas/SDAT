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

/** Meets 60 fps: at least 57 frames a second, and at most 5% of frames slower than a 60 Hz frame. A slower display never does. */
fun meets60(fps: Double, over60: Double) = fps >= 57.0 && over60 <= 5.0

/**
 * One scene across rounds: its median fps, and its 1% low, the rate its slowest 1% of frames could hold
 * (1000 / p99 ms, capped at the display's rate).
 */
data class SceneScore(val label: String, val fps: Double, val low1: Double, val meets60: Boolean)

/** The headline: the slowest scene sets the worst case, as the slowest screen sets how SDAT feels. */
data class Score(val scenes: List<SceneScore>, val refreshHz: Float) {
    val worst get() = scenes.minBy { it.fps }
    val worstLow get() = scenes.minBy { it.low1 }
    val passing get() = scenes.count { it.meets60 }
}

fun score(runs: List<Run>, refreshHz: Float) = Score(
    runs.groupBy { it.scene }.map { (scene, group) ->
        val fps = group.map { it.stats.fps }.median()
        val p99 = group.map { it.stats.p99Ms }.median()
        val low1 = if (p99 > 0) minOf(refreshHz.toDouble(), 1000 / p99) else refreshHz.toDouble()
        SceneScore(scene.label, fps, low1, meets60(fps, group.map { it.stats.over60Percent }.median()))
    },
    refreshHz,
)

private val thermalNames = listOf("none", "light", "moderate", "severe", "critical", "emergency", "shutdown")

fun report(device: Device, runs: List<Run>, rounds: Int, reruns: Int = 0): String = buildString {
    appendLine("SDAT glass benchmark")
    appendLine("${device.name} · ${device.soc} · Android ${device.android}")
    appendLine("RAM ${device.ramGb.f()} GB${if (device.lowRam) " (low-RAM device)" else ""} · ${device.screen} · ${device.refreshHz.toDouble().f(0)} Hz")
    appendLine("Glasmio ${device.glasmio} · $rounds round(s), medians with (min-max) across rounds")
    if (device.refreshHz < 59) appendLine("This display runs below 60 Hz, so no scene can reach 60 fps on it.")
    if (reruns > 0) appendLine("$reruns scene(s) were measured again after the app left the foreground.")
    appendLine()
    val score = score(runs, device.refreshHz)
    appendLine("SCORE: worst case ${score.worst.fps.f()} fps (${score.worst.label})")
    appendLine("       1% low ${score.worstLow.low1.f()} fps (${score.worstLow.label})")
    appendLine("       ${score.passing}/${score.scenes.size} scenes hold 60 fps")
    appendLine()
    for ((scene, group) in runs.groupBy { it.scene }) {
        val fps = group.map { it.stats.fps }
        val over = group.map { it.stats.over60Percent }
        val verdict = if (score.scenes.first { it.label == scene.label }.meets60) "60 fps OK" else "BELOW 60 fps"
        appendLine("[$verdict] ${scene.label}")
        appendLine("  fps ${fps.median().f()}${fps.spread()} · frames >16.7 ms ${over.median().f()}%${over.spread()} · missed display deadline ${group.map { it.stats.jankPercent }.median().f()}%")
        appendLine("  frame p50 ${group.map { it.stats.p50Ms }.median().f()} · p90 ${group.map { it.stats.p90Ms }.median().f()}${group.map { it.stats.p90Ms }.spread()} · p99 ${group.map { it.stats.p99Ms }.median().f()} · max ${group.maxOf { it.stats.maxMs }.f()} ms")
        val limiters = group.map { it.stats.limiter }.groupingBy { it }.eachCount().maxBy { it.value }.key
        appendLine("  p90 by stage: UI ${group.map { it.stats.uiP90Ms }.median().f()} · RenderThread ${group.map { it.stats.renderP90Ms }.median().f()} · GPU ${group.mapNotNull { it.stats.gpuP90Ms }.let { if (it.isEmpty()) "n/a" else "${it.median().f()} ms" }} · limiter $limiters")
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
