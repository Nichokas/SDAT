package com.nichokas.sdat.ui.kit

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.layout
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.lerp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.util.lerp
import com.nichokas.sdat.core.Morph
import com.nichokas.sdat.ui.SdatIcon

/** [symbol] is an icon name from Symbols.kt. */
data class TabItem<T>(val key: T, val label: String, val symbol: String)

private val barHeight = 62.dp

/**
 * The floating glass tab bar. [compactProgress] shrinks it from the full width it is given to a [compactSize] circle
 * holding the selected tab's icon, which [onExpand]s it again.
 */
@Composable
fun <T> TabBar(
    items: List<TabItem<T>>,
    selectedKey: T,
    onSelected: (T) -> Unit,
    onExpand: () -> Unit,
    compactProgress: State<Float>,
    compactSize: Dp,
    modifier: Modifier = Modifier,
) {
    val colors = SdatTheme.colors
    val compact by remember(compactProgress) { derivedStateOf { compactProgress.value >= 0.5f } }
    // The slot keeps the full width so the shrinking bar stays at its start edge instead of being centred in it.
    Box(modifier.height(barHeight), contentAlignment = Alignment.CenterStart) {
        Box(
            Modifier
                .layout { measurable, constraints ->
                    val p = compactProgress.value
                    val w = lerp(constraints.maxWidth.toFloat(), compactSize.toPx(), p).toInt()
                    val h = lerp(barHeight, compactSize, p).roundToPx()
                    val placeable = measurable.measure(Constraints.fixed(w, h))
                    layout(w, h) { placeable.place(0, 0) }
                }
                .glass(capsule),
        ) {
            Row(
                Modifier
                    .fillMaxSize()
                    .graphicsLayer {
                        val p = compactProgress.value
                        alpha = Morph.outAlpha(p)
                        scaleX = Morph.outScale(p)
                        scaleY = scaleX
                    }
                    .padding(4.dp),
            ) {
                items.forEach { item ->
                    val selected = item.key == selectedKey
                    Column(
                        Modifier
                            .weight(1f)
                            .fillMaxHeight()
                            .clip(capsule)
                            .then(if (selected) Modifier.background(colors.fill) else Modifier)
                            .selectable(selected, enabled = !compact, role = Role.Tab) { onSelected(item.key) },
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center,
                    ) {
                        val tint = if (selected) colors.accent else colors.content
                        SdatIcon(item.symbol, null, size = 26.dp, tint = tint)
                        Text(item.label, style = SdatType.caption2.copy(fontSize = 10.sp, fontWeight = FontWeight.SemiBold), color = tint, maxLines = 1)
                    }
                }
            }
            val selected = items.firstOrNull { it.key == selectedKey } ?: items.first()
            Box(
                Modifier
                    .fillMaxSize()
                    .graphicsLayer {
                        val p = compactProgress.value
                        alpha = Morph.inAlpha(p)
                        scaleX = Morph.inScale(p)
                        scaleY = scaleX
                    }
                    .clickable(enabled = compact, role = Role.Button, onClickLabel = "Show tabs", onClick = onExpand),
                contentAlignment = Alignment.Center,
            ) {
                SdatIcon(selected.symbol, selected.label, size = 26.dp, tint = colors.accent)
            }
        }
    }
}
