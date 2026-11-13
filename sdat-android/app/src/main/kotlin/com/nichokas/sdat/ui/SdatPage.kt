package com.nichokas.sdat.ui

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.ListScroll
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
    GlasmioPinnedListPage(
        title = title,
        modifier = modifier,
        listState = listState,
        onNavigateBack = onBack,
        actions = actions,
        bottomPaddingValues = PaddingValues(bottom = BottomChromeSpace),
        content = content,
    )
}

/** Stand-in for a destination whose screen is not ported yet. */
@Composable
fun PlaceholderPage(title: String) = SdatPage(title, onBack = Chrome::pop) {}
