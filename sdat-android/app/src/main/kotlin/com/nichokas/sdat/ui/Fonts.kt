package com.nichokas.sdat.ui

import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
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

val inter = FontFamily(
    listOf(400, 500, 600, 700).map { w ->
        Font(R.font.inter, FontWeight(w), variationSettings = FontVariation.Settings(FontVariation.weight(w)))
    },
)
private val materialOutline = FontFamily(Font(R.font.material_symbols_rounded))
private val materialFill = FontFamily(Font(R.font.material_symbols_rounded_fill))

@Composable
fun SdatIcon(
    name: String,
    contentDescription: String?,
    modifier: Modifier = Modifier,
    size: Dp = 24.dp,
    tint: Color = LocalContentColor.current,
) {
    Text(
        text = symbols.getValue(name),
        modifier = modifier.clearAndSetSemantics { contentDescription?.let { this.contentDescription = it } },
        color = tint,
        fontFamily = if (name.endsWith(".fill")) materialFill else materialOutline,
        fontSize = with(LocalDensity.current) { size.toSp() },
        maxLines = 1,
        softWrap = false,
    )
}
