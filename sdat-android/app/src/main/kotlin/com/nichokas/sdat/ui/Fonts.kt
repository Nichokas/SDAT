package com.nichokas.sdat.ui

import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontSynthesis
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nichokas.sdat.R

private fun interAt(opticalSize: Float) = FontFamily(
    listOf(400, 500, 600, 700, 800).map { weight ->
        Font(
            R.font.inter,
            FontWeight(weight),
            variationSettings = FontVariation.Settings(
                FontVariation.weight(weight),
                FontVariation.Setting("opsz", opticalSize),
            ),
        )
    },
)

val inter = interAt(14f)
val interDisplay = interAt(32f)

private fun materialSymbols(fill: Float) = FontFamily(
    listOf(400, 500, 600, 700).map { weight ->
        Font(
            R.font.material_symbols_rounded,
            FontWeight(weight),
            variationSettings = FontVariation.Settings(
                FontVariation.weight(weight),
                FontVariation.Setting("FILL", fill),
            ),
        )
    },
)

private val materialOutline = materialSymbols(0f)
private val materialFill = materialSymbols(1f)

@Composable
fun SdatIcon(
    name: String,
    contentDescription: String?,
    modifier: Modifier = Modifier,
    size: Dp = 24.dp,
    tint: Color = LocalContentColor.current,
    weight: FontWeight = LocalTextStyle.current.fontWeight ?: FontWeight.Normal,
) {
    val slash = if (name == "star.slash") Modifier.starSlash(tint) else Modifier
    Text(
        text = symbols.getValue(name),
        modifier = modifier.then(slash).clearAndSetSemantics { contentDescription?.let { this.contentDescription = it } },
        color = tint,
        fontFamily = if (name.endsWith(".fill")) materialFill else materialOutline,
        fontWeight = weight,
        fontSize = with(LocalDensity.current) { size.toSp() },
        style = TextStyle(letterSpacing = 0.sp, fontSynthesis = FontSynthesis.None),
        maxLines = 1,
        softWrap = false,
    )
}

private fun Modifier.starSlash(tint: Color) =
    graphicsLayer(compositingStrategy = CompositingStrategy.Offscreen).drawWithContent {
        drawContent()
        val extent = size.minDimension
        val diagonal = Offset(extent * 0.34f, extent * 0.34f)
        val start = center - diagonal
        val end = center + diagonal
        drawLine(tint, start, end, extent * 0.16f, StrokeCap.Round, blendMode = BlendMode.Clear)
        drawLine(tint, start, end, extent * 0.08f, StrokeCap.Round)
    }
