package com.nichokas.sdat.ui

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlin.math.PI
import kotlin.math.roundToInt

/** iOS dark-mode system blue, SwiftUI's `.blue`. */
private val iosBlue = Color(0xFF0A84FF)

/**
 * iOS ProgressRing: blue arc from 12 o'clock clockwise over a static dashed white ring; spins once a second while loading.
 * [progress] is a lambda so the player's position ticks recompose the ring, not its caller.
 */
@Composable
fun ProgressRing(progress: () -> Double, spinning: Boolean, modifier: Modifier = Modifier, lineWidth: Dp = 2.5.dp) {
    val q by animateFloatAsState(
        if (spinning) 0f else progress().coerceIn(0.0, 1.0).toFloat(),
        tween(250, easing = LinearEasing),
        label = "progress",
    )
    // Composed only while spinning: an idle infinite transition would keep animating unseen.
    val turn = if (spinning) {
        rememberInfiniteTransition(label = "spin").animateFloat(0f, 360f, infiniteRepeatable(tween(1000, easing = LinearEasing)), label = "turn")
    } else {
        null
    }
    Canvas(modifier.graphicsLayer { rotationZ = turn?.value ?: 0f }) {
        val d = size.minDimension
        val w = lineWidth.toPx()
        // Like iOS, the dash period fits a whole number of dashes so the pattern never shifts.
        val circ = PI.toFloat() * maxOf(d, 1f)
        val period = circ / maxOf((circ / 5.5.dp.toPx()).roundToInt(), 8)
        val topLeft = Offset((size.width - d) / 2, (size.height - d) / 2)
        val box = Size(d, d)
        // Dashes only under the unplayed part; the phase keeps them where the full ring would draw them.
        drawArc(
            Color.White.copy(alpha = 0.4f), -90f + 360f * q, 360f * (1 - q), false, topLeft, box,
            style = Stroke(w * 0.8f, cap = StrokeCap.Butt, pathEffect = PathEffect.dashPathEffect(floatArrayOf(period * 2.5f / 5.5f, period * 3f / 5.5f), q * circ)),
        )
        if (q > 0) drawArc(iosBlue, -90f, 360f * q, false, topLeft, box, style = Stroke(w, cap = StrokeCap.Butt))
    }
}
