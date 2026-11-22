package com.nichokas.sdat.ui

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.ListScroll
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.glass.GlasmioBackdropHost
import io.github.neoruaa.glasmio.glass.LocalBlurBackdrop
import io.github.neoruaa.glasmio.scaffold.GlasmioPinnedListPage

/** Room below a list for the floating bottom chrome, as on iOS. */
val BottomChromeSpace = 78.dp

/**
 * List page with a large title that collapses into the top bar as the list scrolls.
 * [onBack] shows a back button; pass `Chrome::pop` on pushed pages.
 * [collapsesChrome] is for tab roots: scrolling down shrinks the tab bar and grows the mini-player.
 */
@Composable
fun SdatPage(
    title: String,
    modifier: Modifier = Modifier,
    listState: LazyListState = rememberLazyListState(),
    onBack: (() -> Unit)? = null,
    collapsesChrome: Boolean = false,
    showsLargeTitle: Boolean = true,
    bottomPaddingValues: PaddingValues? = PaddingValues(bottom = BottomChromeSpace),
    actions: @Composable RowScope.() -> Unit = {},
    content: LazyListScope.() -> Unit,
) {
    if (collapsesChrome) {
        val density = LocalDensity.current.density
        LaunchedEffect(listState, density) {
            snapshotFlow { ListScroll(listState.firstVisibleItemIndex, listState.firstVisibleItemScrollOffset) }
                .collect { Chrome.rootScrolled(it.dp(density)) }
        }
    }
    // RootView records every page into its backdrop: isolate glass controls on a solid layer to avoid a RenderNode cycle, while keeping the root blur backdrop for popups and dialogs.
    val blurBackdrop = LocalBlurBackdrop.current
    GlasmioBackdropHost(
        sampledColor = GlasmioTheme.colors.groupedBackground,
        modifier = modifier.fillMaxSize(),
    ) {
        CompositionLocalProvider(LocalBlurBackdrop provides blurBackdrop) {
            GlasmioPinnedListPage(
                title = title,
                listState = listState,
                showsLargeTitle = showsLargeTitle,
                onNavigateBack = onBack,
                actions = actions,
                bottomPaddingValues = bottomPaddingValues,
                content = content,
            )
        }
    }
}

/** Stand-in for a destination whose screen is not ported yet. */
@Composable
fun PlaceholderPage(title: String) = SdatPage(title, onBack = Chrome::pop) {}
