package com.nichokas.sdat.ui.kit

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.ui.SdatIcon

private val barHeight = 52.dp

/**
 * A page under a top bar of glass buttons: back at the start, [actions] at the end, and [title] fading in with
 * [collapseProgress]. While [scrolled], a scrim of [backgroundColor] keeps the bar readable over the content.
 */
@Composable
fun PinnedPage(
    title: String,
    onNavigateBack: (() -> Unit)?,
    collapseProgress: Float,
    scrolled: Boolean,
    backgroundColor: Color,
    modifier: Modifier = Modifier,
    actions: @Composable RowScope.() -> Unit = {},
    background: @Composable BoxScope.() -> Unit = {},
    content: @Composable (PaddingValues) -> Unit,
) {
    val status = WindowInsets.statusBars.asPaddingValues().calculateTopPadding()
    val nav = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
    val edge by animateFloatAsState(if (scrolled) 1f else 0f, label = "edge")
    Box(modifier.fillMaxSize().background(backgroundColor)) {
        background()
        content(PaddingValues(top = status + barHeight, bottom = nav))
        Box(
            Modifier
                .fillMaxWidth()
                .height(status + barHeight + 16.dp)
                .graphicsLayer { alpha = edge }
                .background(Brush.verticalGradient(0.7f to backgroundColor, 1f to backgroundColor.copy(alpha = 0f))),
        )
        Row(
            Modifier.fillMaxWidth().statusBarsPadding().height(barHeight).padding(start = 16.dp, end = 16.dp, top = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.Top,
        ) {
            if (onNavigateBack != null) IconButton(onNavigateBack) { SdatIcon("chevron.left", "Back", size = 26.dp) }
            Box(Modifier.weight(1f).height(44.dp), contentAlignment = Alignment.Center) {
                Text(
                    title,
                    Modifier.graphicsLayer { alpha = collapseProgress },
                    style = SdatType.headline,
                    color = SdatTheme.colors.content,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            actions()
        }
    }
}

/** A list page whose large [title] scrolls up into the top bar. */
@Composable
fun PinnedListPage(
    title: String,
    listState: LazyListState,
    showsLargeTitle: Boolean,
    onNavigateBack: (() -> Unit)?,
    actions: @Composable RowScope.() -> Unit,
    bottomPaddingValues: PaddingValues?,
    modifier: Modifier = Modifier,
    content: LazyListScope.() -> Unit,
) {
    val density = LocalDensity.current
    val pastTitle by remember(showsLargeTitle) {
        derivedStateOf {
            !showsLargeTitle || listState.firstVisibleItemIndex > 0 || listState.firstVisibleItemScrollOffset > with(density) { 36.dp.toPx() }
        }
    }
    val collapse by animateFloatAsState(if (pastTitle) 1f else 0f, label = "title")
    val scrolled by remember { derivedStateOf { listState.canScrollBackward } }
    PinnedPage(
        title = title,
        onNavigateBack = onNavigateBack,
        collapseProgress = collapse,
        scrolled = scrolled,
        backgroundColor = SdatTheme.colors.groupedBackground,
        modifier = modifier,
        actions = actions,
    ) { padding ->
        LazyColumn(
            state = listState,
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(
                top = padding.calculateTopPadding(),
                bottom = padding.calculateBottomPadding() + (bottomPaddingValues?.calculateBottomPadding() ?: 0.dp),
            ),
        ) {
            if (showsLargeTitle) {
                item(key = "large-title") {
                    Text(
                        title,
                        Modifier.padding(start = 16.dp, end = 16.dp, bottom = 8.dp),
                        style = SdatType.largeTitle,
                        color = SdatTheme.colors.content,
                    )
                }
            }
            content()
        }
    }
}
