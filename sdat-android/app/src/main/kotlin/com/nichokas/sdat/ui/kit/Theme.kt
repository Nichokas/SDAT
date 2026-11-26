package com.nichokas.sdat.ui.kit

import android.content.Context
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import com.nichokas.sdat.core.Accent
import com.nichokas.sdat.ui.inter
import com.nichokas.sdat.ui.interDisplay

/** The roles SDAT draws with, all read from one Material colour scheme. */
@Immutable
data class Palette(
    val isDark: Boolean,
    val accent: Color,
    val onAccent: Color,
    val content: Color,
    val secondaryContent: Color,
    val tertiaryContent: Color,
    /** Behind the sheet: the status bar area and page headers. */
    val frame: Color,
    /** The rounded page every screen sits on, and the bottom bar. */
    val sheet: Color,
    /** Cards and menus on the sheet. */
    val elevatedBackground: Color,
    /** Plain buttons, fields and the selected tab's pill. */
    val fill: Color,
    val separator: Color,
    val inverse: Color,
    val onInverse: Color,
    val destructive: Color,
    val onDestructive: Color,
)

private fun ColorScheme.palette(isDark: Boolean) = Palette(
    isDark = isDark,
    accent = primary,
    onAccent = onPrimary,
    content = onSurface,
    secondaryContent = onSurfaceVariant,
    tertiaryContent = onSurfaceVariant.copy(alpha = 0.6f),
    // A dark frame sinks below the sheet; a light one sits just under it.
    frame = if (isDark) surfaceContainerLowest else surfaceContainer,
    sheet = if (isDark) surfaceContainerLow else surfaceContainerLowest,
    elevatedBackground = if (isDark) surfaceContainerHigh else surfaceContainer,
    fill = surfaceContainerHighest,
    separator = outlineVariant,
    inverse = inverseSurface,
    onInverse = inverseOnSurface,
    destructive = error,
    onDestructive = onError,
)

/** The neutrals of a Material scheme tinted towards one accent, for the presets that don't follow the wallpaper. */
private class Tones(
    val primary: Long, val onPrimary: Long,
    val lowest: Long, val low: Long, val container: Long, val high: Long, val highest: Long,
    val onSurface: Long, val onSurfaceVariant: Long, val outlineVariant: Long,
    val inverseSurface: Long, val inverseOnSurface: Long,
)

private fun Tones.scheme(isDark: Boolean): ColorScheme {
    val base = if (isDark) darkColorScheme() else lightColorScheme()
    return base.copy(
        primary = Color(primary), onPrimary = Color(onPrimary),
        background = Color(if (isDark) low else lowest), onBackground = Color(onSurface),
        surface = Color(if (isDark) low else lowest), onSurface = Color(onSurface), onSurfaceVariant = Color(onSurfaceVariant),
        surfaceContainerLowest = Color(lowest), surfaceContainerLow = Color(low), surfaceContainer = Color(container),
        surfaceContainerHigh = Color(high), surfaceContainerHighest = Color(highest),
        outlineVariant = Color(outlineVariant), inverseSurface = Color(inverseSurface), inverseOnSurface = Color(inverseOnSurface),
    )
}

private val presets = mapOf(
    Accent.Blue to (
        Tones(0xFF3F5F90, 0xFFFFFFFF, 0xFFFCFCFF, 0xFFF6F6FC, 0xFFECEDF3, 0xFFE8E9EF, 0xFFE3E4EA, 0xFF1A1C20, 0xFF5D5F66, 0xFFDDDEE4, 0xFF2E3035, 0xFFF1F0F7) to
            Tones(0xFFA8C8FF, 0xFF0A305F, 0xFF0E1013, 0xFF191B1F, 0xFF1E2024, 0xFF2A2C31, 0xFF33353A, 0xFFE3E2E9, 0xFFA9ABB3, 0xFF2F3136, 0xFFE3E2E9, 0xFF1A1C20)
        ),
    Accent.Green to (
        Tones(0xFF366A2D, 0xFFFFFFFF, 0xFFFCFDF7, 0xFFF6F7F1, 0xFFECEEE8, 0xFFE7E9E2, 0xFFE2E4DD, 0xFF1A1C19, 0xFF5C6058, 0xFFDCDED6, 0xFF2F312D, 0xFFF0F1EB) to
            Tones(0xFF9AD58F, 0xFF0A390A, 0xFF0E100D, 0xFF191C18, 0xFF1E211D, 0xFF2A2D28, 0xFF333632, 0xFFE2E3DD, 0xFFA9ADA4, 0xFF2F322D, 0xFFE2E3DD, 0xFF1A1C19)
        ),
    Accent.Rose to (
        Tones(0xFF8F4A4B, 0xFFFFFFFF, 0xFFFFFBFA, 0xFFFAF4F3, 0xFFF3ECEB, 0xFFEEE7E6, 0xFFEAE2E1, 0xFF201A1A, 0xFF655B5A, 0xFFE4DAD9, 0xFF362F2E, 0xFFFBEEEC) to
            Tones(0xFFFFB3B2, 0xFF561D1F, 0xFF120D0D, 0xFF1E1918, 0xFF231E1D, 0xFF302A29, 0xFF3A3433, 0xFFECE0DE, 0xFFB3A8A6, 0xFF352E2D, 0xFFECE0DE, 0xFF201A1A)
        ),
)

/** [Accent.System] is the wallpaper's Material You scheme; minSdk 33 always has one. */
fun colorScheme(accent: Accent, isDark: Boolean, context: Context): ColorScheme {
    val tones = presets[accent] ?: return if (isDark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
    return (if (isDark) tones.second else tones.first).scheme(isDark)
}

private val LocalPalette = staticCompositionLocalOf { presets.getValue(Accent.Blue).second.scheme(true).palette(true) }

object SdatTheme {
    val colors: Palette
        @Composable @ReadOnlyComposable get() = LocalPalette.current
}

@Composable
fun SdatTheme(accent: Accent = Accent.System, isDark: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    val context = LocalContext.current
    val scheme = remember(accent, isDark, context) { colorScheme(accent, isDark, context) }
    val colors = remember(scheme) { scheme.palette(isDark) }
    MaterialTheme(colorScheme = scheme, typography = MaterialTheme.typography.copy(bodyLarge = SdatType.body, bodyMedium = SdatType.subheadline)) {
        CompositionLocalProvider(LocalPalette provides colors, LocalContentColor provides colors.content, content = content)
    }
}

private fun style(size: Int, leading: Int, tracking: Float, weight: Int = 400) = TextStyle(
    fontFamily = if (size >= 20) interDisplay else inter,
    fontSize = size.sp,
    lineHeight = leading.sp,
    letterSpacing = tracking.sp,
    fontWeight = FontWeight(weight),
)

/** Apple's text styles at the default Large size, in Inter. */
object SdatType {
    val largeTitle = style(34, 41, 0.40f, 700)
    val title1 = style(28, 34, 0.38f)
    val title2 = style(22, 28, -0.26f, 700)
    val title3 = style(20, 25, -0.45f)
    val headline = style(17, 22, -0.43f, 600)
    val body = style(17, 22, -0.43f)
    val subheadline = style(15, 20, -0.23f)
    val footnote = style(13, 18, -0.08f)
    val caption1 = style(12, 16, 0f)
    val caption2 = style(11, 13, 0.06f)
}
