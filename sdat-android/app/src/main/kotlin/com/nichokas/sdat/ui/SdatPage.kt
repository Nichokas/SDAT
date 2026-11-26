package com.nichokas.sdat.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.pulltorefresh.PullToRefreshDefaults
import androidx.compose.material3.pulltorefresh.pullToRefresh
import androidx.compose.material3.pulltorefresh.rememberPullToRefreshState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SheetPage

/**
 * A list on the page's sheet. [onBack] shows a back button; pass `Chrome::pop` on pushed pages.
 * [pinnedContent] stays above the list while it scrolls; [onRefresh] turns on pull to refresh.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SdatPage(
    title: String,
    modifier: Modifier = Modifier,
    listState: LazyListState = rememberLazyListState(),
    onBack: (() -> Unit)? = null,
    actions: @Composable RowScope.() -> Unit = {},
    pinnedContent: (@Composable () -> Unit)? = null,
    isRefreshing: Boolean = false,
    onRefresh: (() -> Unit)? = null,
    content: LazyListScope.() -> Unit,
) {
    SheetPage(title, onBack, modifier, actions) {
        val refreshState = onRefresh?.let { rememberPullToRefreshState() }
        val refresh = if (refreshState != null && onRefresh != null) {
            Modifier.pullToRefresh(isRefreshing = isRefreshing, state = refreshState, onRefresh = onRefresh)
        } else {
            Modifier
        }
        Box(Modifier.fillMaxSize().then(refresh)) {
            Column(Modifier.fillMaxSize()) {
                pinnedContent?.invoke()
                LazyColumn(
                    Modifier.weight(1f).fillMaxWidth(),
                    state = listState,
                    contentPadding = PaddingValues(top = 8.dp, bottom = 16.dp),
                    content = content,
                )
            }
            if (refreshState != null) {
                PullToRefreshDefaults.Indicator(
                    state = refreshState,
                    isRefreshing = isRefreshing,
                    modifier = Modifier.align(Alignment.TopCenter),
                    containerColor = SdatTheme.colors.elevatedBackground,
                    color = SdatTheme.colors.accent,
                )
            }
        }
    }
}
