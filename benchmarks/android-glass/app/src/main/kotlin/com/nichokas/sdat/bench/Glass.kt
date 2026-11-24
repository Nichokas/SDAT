package com.nichokas.sdat.bench

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.State
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.layout
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.lerp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.util.lerp
import dev.chrisbanes.haze.HazeInput
import dev.chrisbanes.haze.HazePerformanceMode
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.HazeColorEffect
import dev.chrisbanes.haze.blur.hazeBlur
import dev.chrisbanes.haze.hazeSource
import dev.chrisbanes.haze.rememberHazeState

// SDAT's ui/kit glass, colours and tab bar, reduced to what the scenes draw. Keep the glass recipe in step with
// sdat-android's ui/kit/Glass.kt, or the benchmark measures something SDAT does not ship.

data class Colors(val content: Color, val secondaryContent: Color, val accent: Color, val glass: Color, val groupedBackground: Color, val separator: Color, val fill: Color)

private fun palette(dark: Boolean) = if (dark) {
    Colors(Color.White, Color(0xB2EBEBF5), Color(0xFFFF4245), Color(0xFF1C1C1E).copy(alpha = 0.72f), Color.Black, Color(0x2BFFFFFF), Color(0x3D767680))
} else {
    Colors(Color.Black, Color.Black.copy(alpha = 0.58f), Color(0xFFFF3B30), Color(0xFFF8F8FA).copy(alpha = 0.78f), Color(0xFFF2F2F7), Color(0xFFE6E6E6), Color(0x1F767680))
}

private val LocalColors = staticCompositionLocalOf { palette(dark = true) }
private val LocalHaze = staticCompositionLocalOf<HazeState?> { null }

val colors: Colors @Composable get() = LocalColors.current

@Composable
fun Theme(dark: Boolean, content: @Composable () -> Unit) = CompositionLocalProvider(LocalColors provides palette(dark), content = content)

object Type {
    val largeTitle = TextStyle(fontSize = 34.sp, lineHeight = 41.sp, fontWeight = FontWeight.Bold)
    val title2 = TextStyle(fontSize = 22.sp, lineHeight = 28.sp, fontWeight = FontWeight.Bold)
    val title3 = TextStyle(fontSize = 20.sp, lineHeight = 25.sp)
    val body = TextStyle(fontSize = 17.sp, lineHeight = 22.sp)
    val subheadline = TextStyle(fontSize = 15.sp, lineHeight = 20.sp)
    val caption1 = TextStyle(fontSize = 12.sp, lineHeight = 16.sp)
}

val capsule = RoundedCornerShape(percent = 50)

/** SDAT's GlassHost for [Glass.Blur], the same layout with nothing recorded for [Glass.Tint], and no chrome for [Glass.None]. */
@Composable
internal fun Host(glass: Glass, page: @Composable BoxScope.() -> Unit, chrome: @Composable BoxScope.() -> Unit) {
    when (glass) {
        Glass.None -> Box(Modifier.fillMaxSize()) { page() }
        Glass.Tint -> Box(Modifier.fillMaxSize()) { page(); chrome() }
        Glass.Blur -> {
            val haze = rememberHazeState()
            Box(Modifier.fillMaxSize()) {
                Box(Modifier.fillMaxSize().hazeSource(haze)) { page() }
                CompositionLocalProvider(LocalHaze provides haze) { chrome() }
            }
        }
    }
}

@Composable
fun Modifier.glass(shape: Shape): Modifier {
    val c = colors
    val haze = LocalHaze.current
    val base = shadow(10.dp, shape, ambientColor = Color.Black.copy(alpha = 0.12f), spotColor = Color.Black.copy(alpha = 0.18f)).clip(shape)
    val fill = if (haze == null) base.background(c.glass) else base.hazeBlur(
        HazeInput.Sources(haze),
        HazeBlurStyle {
            blurRadius(24.dp)
            backgroundColor(c.groupedBackground)
            colorEffects(listOf(HazeColorEffect.tint(c.glass)))
        },
        performanceMode = HazePerformanceMode.Performance,
        expandLayerBounds = false,
    )
    return fill.border(0.5.dp, c.separator, shape)
}

private val barHeight = 62.dp

/** SDAT's TabBar: three tabs in a glass capsule that [compact] shrinks to a [compactSize] circle. */
@Composable
internal fun TabBar(labels: List<String>, selected: Int, compact: State<Float>, compactSize: Dp, modifier: Modifier) {
    Box(modifier.height(barHeight), contentAlignment = Alignment.CenterStart) {
        Box(
            Modifier
                .layout { measurable, constraints ->
                    val p = compact.value
                    val w = lerp(constraints.maxWidth.toFloat(), compactSize.toPx(), p).toInt()
                    val h = lerp(barHeight, compactSize, p).roundToPx()
                    val placeable = measurable.measure(Constraints.fixed(w, h))
                    layout(w, h) { placeable.place(0, 0) }
                }
                .glass(capsule),
        ) {
            Row(Modifier.fillMaxSize().graphicsLayer { alpha = 1f - compact.value }.padding(4.dp)) {
                labels.forEachIndexed { i, label ->
                    val tint = if (i == selected) colors.accent else colors.content
                    Column(
                        Modifier.weight(1f).fillMaxHeight().clip(capsule).then(if (i == selected) Modifier.background(colors.fill) else Modifier),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center,
                    ) {
                        // Stands in for the Material Symbols glyph, which costs about the same to draw.
                        Box(Modifier.size(22.dp).background(tint, CircleShape))
                        Text(label, style = TextStyle(fontSize = 10.sp, fontWeight = FontWeight.SemiBold), color = tint, maxLines = 1)
                    }
                }
            }
            Box(Modifier.fillMaxSize().graphicsLayer { alpha = compact.value }, contentAlignment = Alignment.Center) {
                Box(Modifier.size(22.dp).background(colors.accent, CircleShape))
            }
        }
    }
}
