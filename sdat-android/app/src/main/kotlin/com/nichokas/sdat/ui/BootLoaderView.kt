package com.nichokas.sdat.ui

import android.provider.Settings
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Boot
import kotlin.math.sin
import kotlinx.coroutines.delay

/**
 * First-sign-in loader: the mini-player's dashed ring filling white with the warm-up's progress, and three squares
 * lighting up in sequence inside. With animations off, the squares fade gently together instead of stepping.
 */
@Composable
fun BootLoaderView(progress: Double, size: Dp = 96.dp) {
    val context = LocalContext.current
    val reduceMotion = remember { Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f }
    var t by remember { mutableDoubleStateOf(0.0) }
    LaunchedEffect(reduceMotion) {
        val start = System.nanoTime()
        while (true) {
            t = (System.nanoTime() - start) / 1e9
            delay(if (reduceMotion) 33 else 100)
        }
    }
    Box(
        Modifier.fillMaxSize().background(Color.Black).clearAndSetSemantics {
            contentDescription = "Loading"
            stateDescription = "${(progress * 100).toInt()} percent"
        },
        contentAlignment = Alignment.Center,
    ) {
        Box(Modifier.size(size), contentAlignment = Alignment.Center) {
            ProgressRing({ progress }, spinning = false, Modifier.fillMaxSize(), lineWidth = 3.dp, color = Color.White)
            Row(horizontalArrangement = Arrangement.spacedBy(size * 0.08f)) {
                repeat(3) { i ->
                    Box(Modifier.size(size * 0.1f).graphicsLayer { alpha = squareOpacity(i, t, reduceMotion) }.background(Color.White))
                }
            }
        }
    }
}

/** Steps of 0.3 s: none, 1, 1-2, 1-2-3. */
internal fun squareOpacity(index: Int, seconds: Double, reduceMotion: Boolean): Float =
    if (reduceMotion) (0.55 + 0.45 * (0.5 + 0.5 * sin(seconds * 1.5))).toFloat()
    else if (index < (seconds / 0.3).toInt() % 4) 1f else 0f

/** Over a shell composed right after the warm-up: the loader fading out, as iOS crossfades the two. */
@Composable
fun BootLoaderFadeOut() {
    if (!Boot.fadingOut) return
    val alpha = remember { Animatable(1f) }
    LaunchedEffect(Unit) {
        alpha.animateTo(0f, tween(450, easing = FastOutSlowInEasing))
        Boot.fadedOut()
    }
    Box(Modifier.fillMaxSize().graphicsLayer { this.alpha = alpha.value }) { BootLoaderView(1.0) }
}
