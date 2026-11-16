package com.nichokas.sdat.bench

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import java.util.Locale

private val Good = Color(0xFF34C759)
private val Fair = Color(0xFFFFB020)
private val Bad = Color(0xFFFF453A)
private val Muted = Color(0xFF8E8E93)

private fun Double.f() = String.format(Locale.US, "%.0f", this)

private fun color(fps: Double, meets60: Boolean) = when {
    meets60 -> Good
    fps >= 45 -> Fair
    else -> Bad
}

@Composable
fun ScoreCard(score: Score) {
    val worst = score.worst
    Column {
        Text("Worst case", color = Muted, fontSize = 13.sp)
        Row(verticalAlignment = Alignment.Bottom) {
            Text(worst.fps.f(), color = color(worst.fps, worst.meets60), fontSize = 64.sp, fontWeight = FontWeight.Bold)
            Text(" fps", fontSize = 22.sp, modifier = Modifier.padding(bottom = 12.dp))
        }
        Text(worst.label, fontSize = 13.sp)
        Spacer(Modifier.height(8.dp))
        Text("1% low ${score.worstLow.low1.f()} fps · ${score.worstLow.label}", fontSize = 13.sp)
        Text("${score.passing} of ${score.scenes.size} scenes hold 60 fps", fontSize = 13.sp)
    }
}

/** Median fps per scene as bars, the 1% low as a white tick, and the 60 fps line dashed. */
@Composable
fun FpsChart(score: Score) {
    val top = maxOf(score.refreshHz.toDouble(), 60.0, score.scenes.maxOf { it.fps })
    Column {
        Text("fps per scene · bar median, tick 1% low, dashed 60 fps", color = Muted, fontSize = 11.sp)
        for (s in score.scenes) {
            Spacer(Modifier.height(6.dp))
            Text(s.label, fontSize = 11.sp)
            Row(verticalAlignment = Alignment.CenterVertically) {
                Canvas(Modifier.weight(1f).height(14.dp)) {
                    fun x(fps: Double) = (fps / top).toFloat().coerceIn(0f, 1f) * size.width
                    drawRoundRect(Color.White.copy(alpha = 0.08f), cornerRadius = CornerRadius(4f))
                    drawRoundRect(color(s.fps, s.meets60), size = Size(x(s.fps), size.height), cornerRadius = CornerRadius(4f))
                    drawLine(Color.White, Offset(x(s.low1), 0f), Offset(x(s.low1), size.height), strokeWidth = 3f)
                    drawLine(
                        Color.White.copy(alpha = 0.6f), Offset(x(60.0), -4f), Offset(x(60.0), size.height + 4f),
                        strokeWidth = 2f, pathEffect = PathEffect.dashPathEffect(floatArrayOf(6f, 4f)),
                    )
                }
                Text(s.fps.f(), fontSize = 12.sp, modifier = Modifier.width(36.dp).padding(start = 6.dp))
            }
        }
    }
}
