package com.nichokas.sdat.ui

import android.content.Context
import android.graphics.Typeface
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.R

private const val SF_PRO_ASSET = "SF-Pro.ttf"

private val inter = FontFamily(
    listOf(400, 500, 600, 700).map { w ->
        Font(R.font.inter, FontWeight(w), variationSettings = FontVariation.Settings(FontVariation.weight(w)))
    },
)
private val materialOutline = FontFamily(Font(R.font.material_symbols_rounded))
private val materialFill = FontFamily(Font(R.font.material_symbols_rounded_fill))

/** The app's single font and icon source: SF Pro when it is in local assets, otherwise Inter and Material Symbols. */
@Immutable
class AppFonts(val sfPro: Typeface?) {
    val text: FontFamily = sfPro?.let { FontFamily(it) } ?: inter
}

val LocalAppFonts = staticCompositionLocalOf { AppFonts(null) }

fun loadAppFonts(context: Context): AppFonts {
    val assets = context.assets
    val sfPro = if (SF_PRO_ASSET in assets.list("").orEmpty()) Typeface.createFromAsset(assets, SF_PRO_ASSET) else null
    return AppFonts(sfPro)
}

@Composable
fun SdatIcon(
    name: String,
    contentDescription: String?,
    modifier: Modifier = Modifier,
    size: Dp = 24.dp,
    tint: Color = LocalContentColor.current,
) {
    val symbol = symbols.getValue(name)
    val sfPro = LocalAppFonts.current.sfPro
    val (family, glyph) = when {
        sfPro != null -> FontFamily(sfPro) to String(Character.toChars(symbol.sfCodePoint))
        name.endsWith(".fill") -> materialFill to symbol.material
        else -> materialOutline to symbol.material
    }
    Text(
        text = glyph,
        modifier = modifier.clearAndSetSemantics { contentDescription?.let { this.contentDescription = it } },
        color = tint,
        fontFamily = family,
        fontSize = with(LocalDensity.current) { size.toSp() },
        maxLines = 1,
        softWrap = false,
    )
}
