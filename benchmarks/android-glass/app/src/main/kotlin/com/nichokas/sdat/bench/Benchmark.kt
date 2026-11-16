package com.nichokas.sdat.bench

import android.app.Activity
import android.app.ActivityManager
import android.os.Build
import android.os.PowerManager
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first

internal const val WARMUP_MS = 2_000L
internal const val MEASURE_MS = 6_000L
private const val REPORT_LAG_MS = 300L

/** The measured runs, and how many scenes were measured again after the app left the foreground. */
class Measured(val runs: List<Run>, val reruns: Int)

/**
 * Shows every scene [rounds] times on [activity]'s window through [show] and measures it. A scene measured while the
 * app was paused (Home, Back, screen off) draws nothing, so it is measured again once the app is back.
 */
suspend fun benchmark(activity: ComponentActivity, images: ImageCounter, rounds: Int, show: (Scene, String) -> Unit): Measured {
    val power = activity.getSystemService(PowerManager::class.java)
    val runs = mutableListOf<Run>()
    val total = rounds * scenes.size
    var reruns = 0
    var paused = false
    val observer = LifecycleEventObserver { _, event -> if (event == Lifecycle.Event.ON_PAUSE) paused = true }
    activity.lifecycle.addObserver(observer)
    try {
        FrameRecorder(activity.window).use { recorder ->
            for (round in 1..rounds) {
                // Each round starts further along the list, so heat and caches favour no scene.
                val order = scenes.indices.map { scenes[(it + (round - 1) * scenes.size / 3) % scenes.size] }
                for (scene in order) {
                    var attempt = 0
                    while (true) {
                        activity.lifecycle.currentStateFlow.first { it.isAtLeast(Lifecycle.State.RESUMED) }
                        paused = false
                        show(scene, "${runs.size + 1}/$total · round $round" + if (attempt > 0) " · retry $attempt" else "")
                        val run = measure(scene, round, recorder, power, images)
                        if (!paused) {
                            runs += run
                            break
                        }
                        attempt++
                        reruns++
                    }
                }
            }
        }
    } finally {
        activity.lifecycle.removeObserver(observer)
    }
    return Measured(runs, reruns)
}

private suspend fun measure(scene: Scene, round: Int, recorder: FrameRecorder, power: PowerManager, images: ImageCounter): Run {
    val thermalStart = power.currentThermalStatus
    images.loaded = 0
    images.failed = 0
    val load = CpuLoad(scene.busyCores)
    try {
        recorder.drain()
        val warmStart = System.nanoTime()
        delay(WARMUP_MS)
        val start = System.nanoTime()
        delay(MEASURE_MS)
        val end = System.nanoTime()
        // Reports arrive on their own thread after their frame, so the window is cut by vsync time once the last are in.
        delay(REPORT_LAG_MS)
        val frames = recorder.drain()
        val warmupWorstMs = (frames.filter { it.vsyncNs in warmStart until start }.maxOfOrNull { it.totalNs } ?: 0) / 1e6
        val stats = stats(frames, (end - start) / 1e9, start, end)
        return Run(scene, round, stats, warmupWorstMs, thermalStart, power.currentThermalStatus, images.loaded, images.failed)
    } finally {
        load.stop()
    }
}

fun device(activity: Activity): Device {
    val memory = ActivityManager.MemoryInfo().also { activity.getSystemService(ActivityManager::class.java).getMemoryInfo(it) }
    val metrics = activity.resources.displayMetrics
    return Device(
        name = "${Build.MANUFACTURER} ${Build.MODEL}",
        soc = "${Build.SOC_MANUFACTURER} ${Build.SOC_MODEL}",
        android = "${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT})",
        ramGb = memory.totalMem / 1e9,
        lowRam = activity.getSystemService(ActivityManager::class.java).isLowRamDevice,
        refreshHz = activity.display.refreshRate,
        screen = "${metrics.widthPixels}x${metrics.heightPixels} @${metrics.densityDpi} dpi",
        glasmio = BuildConfig.GLASMIO,
    )
}

/** The report in the app's results folder, or null when external storage is unavailable. */
fun save(activity: Activity, text: String): File? = runCatching {
    val stamp = SimpleDateFormat("yyyyMMdd-HHmmss", Locale.US).format(Date())
    File(activity.getExternalFilesDir("results"), "bench-$stamp.txt").apply { writeText(text) }
}.getOrNull()

/** Busy threads standing in for audio decoding, a download and the core's network work beside the UI. */
private class CpuLoad(threads: Int) {
    @Volatile private var running = true
    private val workers = List(threads) {
        Thread {
            var x = 1L
            while (running) x = x * 6364136223846793005L + 1442695040888963407L
            if (x == 42L) Log.v("SdatBench", "unreachable")
        }.apply { isDaemon = true; start() }
    }

    fun stop() {
        running = false
        workers.forEach { it.join() }
    }
}
