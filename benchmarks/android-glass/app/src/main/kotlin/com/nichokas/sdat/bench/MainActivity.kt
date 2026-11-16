package com.nichokas.sdat.bench

import android.app.ActivityManager
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.os.PowerManager
import android.os.SystemClock
import android.util.Log
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withContext

private const val WARMUP_MS = 2_000L
private const val MEASURE_MS = 6_000L

private sealed interface Phase {
    data object Ready : Phase
    data class Running(val rounds: Int) : Phase
    data class Done(val report: String, val score: Score, val file: File?) : Phase
}

class MainActivity : ComponentActivity() {
    private val images = ImageCounter()

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        setContent {
            var phase by remember { mutableStateOf<Phase>(Phase.Ready) }
            when (val p = phase) {
                Phase.Ready -> Menu { rounds -> phase = Phase.Running(rounds) }
                is Phase.Running -> Runner(p.rounds) { done -> phase = done }
                is Phase.Done -> Results(p) { phase = Phase.Ready }
            }
        }
    }

    @Composable
    private fun Runner(rounds: Int, onDone: (Phase.Done) -> Unit) {
        var covers by remember { mutableStateOf<List<File>?>(null) }
        var current by remember { mutableStateOf<Pair<Scene, String>?>(null) }
        LaunchedEffect(Unit) {
            val files = withContext(Dispatchers.Default) { covers(File(cacheDir, "covers")) }
            covers = files
            val power = getSystemService(PowerManager::class.java)
            val runs = mutableListOf<Run>()
            val total = rounds * scenes.size
            var reruns = 0
            // A scene measured while the app is paused (Home, Back, screen off) draws nothing; it is measured again.
            var paused = false
            val observer = LifecycleEventObserver { _, event -> if (event == Lifecycle.Event.ON_PAUSE) paused = true }
            lifecycle.addObserver(observer)
            try {
                FrameRecorder(window).use { recorder ->
                    for (round in 1..rounds) {
                        // Each round starts further along the list, so heat and caches favour no scene.
                        val order = scenes.indices.map { scenes[(it + (round - 1) * scenes.size / 3) % scenes.size] }
                        for (scene in order) {
                            var attempt = 0
                            while (true) {
                                lifecycle.currentStateFlow.first { it.isAtLeast(Lifecycle.State.RESUMED) }
                                paused = false
                                current = scene to "${runs.size + 1}/$total · round $round" + if (attempt > 0) " · again" else ""
                                val run = measure(scene, round, recorder, power)
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
                lifecycle.removeObserver(observer)
            }
            current = null
            val device = device()
            val text = report(device, runs, rounds, reruns)
            Log.i("SdatBench", text)
            val file = withContext(Dispatchers.IO) {
                runCatching {
                    val stamp = SimpleDateFormat("yyyyMMdd-HHmmss", Locale.US).format(Date())
                    File(getExternalFilesDir("results"), "bench-$stamp.txt").apply { writeText(text) }
                }.getOrNull()
            }
            onDone(Phase.Done(text, score(runs, device.refreshHz), file))
        }
        val files = covers
        val shown = current
        if (files == null || shown == null) {
            Status("Preparing ${files?.size ?: 0} covers…")
            return
        }
        Box(Modifier.fillMaxSize()) {
            // Keyed, so every run starts the scene from scratch: no state, scroll or cached layout carries over.
            key(shown) { SceneView(shown.first, files, images) }
            Text(shown.second, color = Color.White, fontSize = 11.sp, modifier = Modifier.safeDrawingPadding().padding(4.dp).background(Color.Black.copy(alpha = 0.5f)).padding(horizontal = 4.dp))
        }
    }

    private suspend fun measure(scene: Scene, round: Int, recorder: FrameRecorder, power: PowerManager): Run {
        val thermalStart = power.currentThermalStatus
        images.loaded = 0
        images.failed = 0
        val load = CpuLoad(scene.busyCores)
        try {
            delay(WARMUP_MS)
            val warmup = recorder.take(WARMUP_MS / 1000.0)
            val start = SystemClock.uptimeMillis()
            delay(MEASURE_MS)
            val stats = recorder.take((SystemClock.uptimeMillis() - start) / 1000.0)
            return Run(scene, round, stats, warmup.maxMs, thermalStart, power.currentThermalStatus, images.loaded, images.failed)
        } finally {
            load.stop()
        }
    }

    private fun device(): Device {
        val memory = ActivityManager.MemoryInfo().also { getSystemService(ActivityManager::class.java).getMemoryInfo(it) }
        val metrics = resources.displayMetrics
        return Device(
            name = "${Build.MANUFACTURER} ${Build.MODEL}",
            soc = "${Build.SOC_MANUFACTURER} ${Build.SOC_MODEL}",
            android = "${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT})",
            ramGb = memory.totalMem / 1e9,
            lowRam = getSystemService(ActivityManager::class.java).isLowRamDevice,
            refreshHz = display.refreshRate,
            screen = "${metrics.widthPixels}x${metrics.heightPixels} @${metrics.densityDpi} dpi",
            glasmio = BuildConfig.GLASMIO,
        )
    }

    @Composable
    private fun Menu(onStart: (Int) -> Unit) = Page {
        Text("SDAT glass benchmark", style = MaterialTheme.typography.headlineSmall)
        Text(
            "Runs ${scenes.size} scenes of SDAT, from a steady Discover pan to a fast fling with busy cores, each with live glass, " +
                "with Glasmio's reduced transparency and without glass. Each scene warms up for ${WARMUP_MS / 1000} s and is measured for " +
                "${MEASURE_MS / 1000} s. Plug the phone in or keep it charged, close other apps, and do not touch the screen while it runs.",
        )
        Button({ onStart(3) }) { Text("Full run, 3 rounds (about ${3 * scenes.size * (WARMUP_MS + MEASURE_MS) / 60_000 + 1} min)") }
        OutlinedButton({ onStart(1) }) { Text("Quick run, 1 round") }
    }

    @Composable
    private fun Results(done: Phase.Done, onAgain: () -> Unit) = Page {
        val (report, score, file) = done
        ScoreCard(score)
        FpsChart(score)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button({
                startActivity(Intent.createChooser(Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, report), "Share results"))
            }) { Text("Share") }
            OutlinedButton({
                getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("SDAT benchmark", report))
            }) { Text("Copy") }
            OutlinedButton(onAgain) { Text("Again") }
        }
        if (file != null) Text("Saved to ${file.absolutePath}", fontSize = 12.sp)
        Text(report, fontFamily = FontFamily.Monospace, fontSize = 11.sp)
    }

    @Composable
    private fun Status(text: String) = Page { Text(text) }

    @Composable
    private fun Page(content: @Composable () -> Unit) {
        MaterialTheme(darkColorScheme()) {
            Column(
                Modifier.fillMaxSize().background(Color.Black).safeDrawingPadding().verticalScroll(rememberScrollState()).padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                CompositionLocalProvider(LocalContentColor provides Color.White) { content() }
            }
        }
    }
}

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
