package com.nichokas.sdat.bench

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.os.Bundle
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
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

private sealed interface Phase {
    data object Ready : Phase
    data class Running(val rounds: Int) : Phase
    data class Done(val report: String, val score: Score, val file: File?) : Phase
}

class MainActivity : ComponentActivity() {
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
        var covers by remember { mutableStateOf<List<Cover>?>(null) }
        var current by remember { mutableStateOf<Shown?>(null) }
        LaunchedEffect(Unit) {
            covers = withContext(Dispatchers.Default) { covers(File(cacheDir, "covers")) }
            val measured = benchmark(this@MainActivity, rounds) { current = it }
            current = null
            val device = device(this@MainActivity)
            val text = report(device, measured.runs, rounds, measured.reruns)
            Log.i("SdatBench", text)
            val file = withContext(Dispatchers.IO) { save(this@MainActivity, text) }
            onDone(Phase.Done(text, score(measured.runs, device.refreshHz), file))
        }
        val files = covers
        val shown = current
        if (files == null || shown == null) {
            Status("Preparing ${files?.size ?: 0} covers…")
            return
        }
        Box(Modifier.fillMaxSize()) {
            // Keyed, so every run starts the scene from scratch: no state, scroll or cached layout carries over.
            key(shown) { SceneView(shown.scene, files, shown.images) }
            Text(shown.label, color = Color.White, fontSize = 11.sp, modifier = Modifier.safeDrawingPadding().padding(4.dp).background(Color.Black.copy(alpha = 0.5f)).padding(horizontal = 4.dp))
        }
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
        Text(if (file != null) "Saved to ${file.absolutePath}" else "Not saved to a file; use Share or Copy", fontSize = 12.sp)
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

