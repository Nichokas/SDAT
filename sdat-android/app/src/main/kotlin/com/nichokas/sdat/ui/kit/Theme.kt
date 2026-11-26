package com.nichokas.sdat.ui.kit

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import com.nichokas.sdat.ui.inter
import com.nichokas.sdat.ui.interDisplay

/** The iOS system colors SDAT is drawn with. */
@Immutable
data class Palette(
    val isDark: Boolean,
    val accent: Color,
    val content: Color,
    val secondaryContent: Color,
    val tertiaryContent: Color,
    /** Fill of floating glass: translucent enough to show the blurred page under it. */
    val glass: Color,
    val groupedBackground: Color,
    val elevatedBackground: Color,
    val separator: Color,
    val destructive: Color,
    /** Fill of plain buttons and fields that sit on a page, not over it. */
    val fill: Color,
)

private fun palette(isDark: Boolean) = if (isDark) {
    Palette(
        isDark = true,
        accent = Color(0xFFFF4245),
        content = Color.White,
        secondaryContent = Color(0xB2EBEBF5),
        tertiaryContent = Color(0x4DEBEBF5),
        glass = Color(0xFF1C1C1E).copy(alpha = 0.72f),
        groupedBackground = Color.Black,
        elevatedBackground = Color(0xFF1C1C1E),
        separator = Color(0x2BFFFFFF),
        destructive = Color(0xFFFF4245),
        fill = Color(0x3D767680),
    )
} else {
    Palette(
        isDark = false,
        accent = Color(0xFFFF3B30),
        content = Color.Black,
        secondaryContent = Color.Black.copy(alpha = 0.58f),
        tertiaryContent = Color(0x4D3C3C43),
        glass = Color(0xFFF8F8FA).copy(alpha = 0.78f),
        groupedBackground = Color(0xFFF2F2F7),
        elevatedBackground = Color.White,
        separator = Color(0xFFE6E6E6),
        destructive = Color(0xFFFF383C),
        fill = Color(0x1F767680),
    )
}

private val LocalPalette = staticCompositionLocalOf { palette(isDark = true) }

object SdatTheme {
    val colors: Palette
        @Composable @ReadOnlyComposable get() = LocalPalette.current
}

@Composable
fun SdatTheme(isDark: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    val colors = palette(isDark)
    val scheme = if (isDark) {
        darkColorScheme(
            primary = colors.accent, onPrimary = Color.White, background = colors.groupedBackground, onBackground = colors.content,
            surface = colors.elevatedBackground, onSurface = colors.content, onSurfaceVariant = colors.secondaryContent,
            surfaceContainerHigh = colors.elevatedBackground, error = colors.destructive, outlineVariant = colors.separator,
        )
    } else {
        lightColorScheme(
            primary = colors.accent, onPrimary = Color.White, background = colors.groupedBackground, onBackground = colors.content,
            surface = colors.elevatedBackground, onSurface = colors.content, onSurfaceVariant = colors.secondaryContent,
            surfaceContainerHigh = colors.elevatedBackground, error = colors.destructive, outlineVariant = colors.separator,
        )
    }
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
