package com.nichokas.sdat.ui.kit

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.lerp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.util.lerp
import com.nichokas.sdat.core.Morph
import com.nichokas.sdat.ui.SdatIcon
import kotlin.math.abs
import kotlin.math.roundToInt

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
    val haptics = LocalHapticFeedback.current
    val rtl = LocalLayoutDirection.current == LayoutDirection.Rtl
    val activeIndex = items.indexOfFirst { it.key == selectedKey }
    val currentIndex by rememberUpdatedState(activeIndex)
    val selectTab by rememberUpdatedState(onSelected)
    var dragPosition by remember(items) { mutableStateOf<Float?>(null) }
    val highlightedIndex = dragPosition?.roundToInt() ?: activeIndex
    val indicatorIndex by animateFloatAsState(highlightedIndex.toFloat(), label = "Tab indicator")
    val iconBounce = remember { Animatable(1f) }
    var previousIndex by remember { mutableIntStateOf(activeIndex) }
    LaunchedEffect(activeIndex) {
        if (previousIndex != activeIndex && activeIndex >= 0) {
            previousIndex = activeIndex
            iconBounce.snapTo(0.87f)
            iconBounce.animateTo(1f, spring(dampingRatio = 0.4f))
        }
    }
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
                    .padding(4.dp)
                    .selectableGroup()
                    .then(if (!compact && items.size > 1) Modifier.pointerInput(items, rtl, haptics) {
                        fun highlight(x: Float) {
                            val slot = (x / size.width * items.size - 0.5f).coerceIn(0f, items.lastIndex.toFloat())
                            val position = if (rtl) items.lastIndex - slot else slot
                            if (position.roundToInt() != (dragPosition?.roundToInt() ?: currentIndex)) {
                                haptics.performHapticFeedback(HapticFeedbackType.SegmentTick)
                            }
                            dragPosition = position
                        }

                        try {
                            detectHorizontalDragGestures(
                                onDragEnd = {
                                    val index = dragPosition?.roundToInt()
                                    dragPosition = null
                                    if (index != null && index != currentIndex) selectTab(items[index].key)
                                },
                                onDragCancel = { dragPosition = null },
                            ) { change, _ -> highlight(change.position.x) }
                        } finally {
                            dragPosition = null
                        }
                    } else Modifier)
                    .drawBehind {
                        if (highlightedIndex < 0) return@drawBehind
                        val width = size.width / items.size
                        val dragged = dragPosition
                        val index = dragged ?: indicatorIndex
                        val position = if (rtl) items.lastIndex - index else index
                        val stretch = dragged?.let { width * (0.15f + 0.2f * abs(it - highlightedIndex)) } ?: 0f
                        val lensWidth = width + stretch
                        drawRoundRect(
                            color = colors.fill,
                            topLeft = Offset((position * width - stretch / 2).coerceIn(0f, (size.width - lensWidth).coerceAtLeast(0f)), 0f),
                            size = Size(lensWidth, size.height),
                            cornerRadius = CornerRadius(minOf(lensWidth, size.height) / 2),
                        )
                    },
            ) {
                items.forEachIndexed { index, item ->
                    val selected = item.key == selectedKey
                    Column(
                        Modifier
                            .weight(1f)
                            .fillMaxHeight()
                            .clip(capsule)
                            .selectable(selected, enabled = !compact, role = Role.Tab) {
                                if (!selected) haptics.performHapticFeedback(HapticFeedbackType.SegmentTick)
                                onSelected(item.key)
                            },
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center,
                    ) {
                        val tint = if (index == highlightedIndex) colors.accent else colors.content
                        SdatIcon(item.symbol, null, Modifier.graphicsLayer {
                            scaleX = if (selected) iconBounce.value else 1f
                            scaleY = scaleX
                        }, size = 26.dp, tint = tint, weight = if (selected) FontWeight.SemiBold else FontWeight.Medium)
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
                    // Absent rather than disabled: a disabled clickable still takes the taps meant for the tabs below.
                    .then(if (compact) Modifier.clickable(role = Role.Button, onClickLabel = "Show tabs", onClick = onExpand) else Modifier),
                contentAlignment = Alignment.Center,
            ) {
                SdatIcon(selected.symbol, selected.label, size = 26.dp, tint = colors.accent, weight = FontWeight.SemiBold)
            }
        }
    }
}
