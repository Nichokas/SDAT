package com.nichokas.sdat.ui.kit

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LocalContentColor
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.dp
import dev.chrisbanes.haze.HazeInput
import dev.chrisbanes.haze.HazePerformanceMode
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.HazeColorEffect
import dev.chrisbanes.haze.blur.hazeBlur
import dev.chrisbanes.haze.hazeSource
import dev.chrisbanes.haze.rememberHazeState

val capsule = RoundedCornerShape(percent = 50)

private val LocalHaze = staticCompositionLocalOf<HazeState?> { null }

/**
 * Records [content] so that [Glass] in [overlay] blurs it. Glass inside [content] has nothing recorded below it and
 * draws its tint only: a surface that sampled the layer it is drawn into would feed back on itself.
 */
@Composable
fun GlassHost(modifier: Modifier = Modifier, content: @Composable () -> Unit, overlay: @Composable BoxScope.() -> Unit = {}) {
    val haze = rememberHazeState()
    Box(modifier) {
        Box(Modifier.fillMaxSize().hazeSource(haze)) { content() }
        CompositionLocalProvider(LocalHaze provides haze) { overlay() }
    }
}

/** Content inside a [GlassHost]'s overlay that is no longer above its recording, such as a dialog window. */
@Composable
fun NoGlassBackdrop(content: @Composable () -> Unit) = CompositionLocalProvider(LocalHaze provides null, content = content)

/** Frosted glass: the page under it blurred and tinted, with a hairline edge and a soft shadow. */
@Composable
fun Modifier.glass(shape: Shape, tint: Color = SdatTheme.colors.glass): Modifier {
    val colors = SdatTheme.colors
    val haze = LocalHaze.current
    val base = shadow(10.dp, shape, ambientColor = Color.Black.copy(alpha = 0.12f), spotColor = Color.Black.copy(alpha = 0.18f))
        .clip(shape)
    // Performance mode blurs a downscaled copy, which is what keeps glass cheap on mid-range GPUs.
    val fill = if (haze == null) base.background(tint) else base.hazeBlur(
        HazeInput.Sources(haze),
        HazeBlurStyle {
            blurRadius(24.dp)
            backgroundColor(colors.groupedBackground)
            colorEffects(listOf(HazeColorEffect.tint(tint)))
        },
        performanceMode = HazePerformanceMode.Performance,
        expandLayerBounds = false,
    )
    return fill.border(0.5.dp, colors.separator, shape)
}

@Composable
fun Glass(modifier: Modifier = Modifier, shape: Shape = RoundedCornerShape(26.dp), content: @Composable BoxScope.() -> Unit = {}) {
    Box(modifier.glass(shape)) {
        CompositionLocalProvider(LocalContentColor provides SdatTheme.colors.content) { content() }
    }
}
