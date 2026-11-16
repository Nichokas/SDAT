package com.nichokas.sdat.bench

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.scaffold.GlasmioPinnedListPage
import java.io.File
import kotlin.math.abs
import kotlin.math.roundToInt

// Collection: a long list of cover rows, scrolled up and down, with the tab bar collapsing as SDAT's does on scroll.

@Composable
internal fun Collection(glass: Glass, covers: List<File>, images: ImageCounter, seconds: State<Float>) {
    val list = rememberLazyListState()
    AutoScroll(list, speedDp = 2200f)
    // Collapses and expands every 2 s with a 0.3 s morph, as a user scrolling down and back up triggers it.
    val compact = remember {
        derivedStateOf {
            val p = seconds.value % 4f
            when {
                p < 1.7f -> 0f
                p < 2f -> (p - 1.7f) / 0.3f
                p < 3.7f -> 1f
                else -> 1f - (p - 3.7f) / 0.3f
            }
        }
    }
    val rows: LazyListScope.() -> Unit = {
        items(400) { i -> CoverRow(covers[i % covers.size], "Track ${i + 1}", "Artist ${i % 37} · Album ${i % 53}", images) }
    }
    Host(
        glass,
        page = {
            if (glass == Glass.None) {
                LazyColumn(Modifier.fillMaxSize().background(GlasmioTheme.colors.groupedBackground).statusBarsPadding(), state = list, content = rows)
            } else {
                GlasmioPinnedListPage(title = "My Collection", listState = list, bottomPaddingValues = PaddingValues(bottom = 96.dp), content = rows)
            }
        },
        chrome = { BottomChrome(Tab.Collection, compact, null) },
    )
}

@Composable
internal fun CoverRow(file: File, title: String, subtitle: String, images: ImageCounter) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        val px = (56 * LocalDensity.current.density).roundToInt()
        AsyncImage(cover(file, px, false, images), null, Modifier.size(56.dp).clip(RoundedCornerShape(8.dp)), contentScale = ContentScale.Crop)
        Column(Modifier.padding(start = 12.dp)) {
            Text(title, style = IosTypography.body, color = GlasmioTheme.colors.content)
            Text(subtitle, style = IosTypography.subheadline, color = GlasmioTheme.colors.secondaryContent)
        }
    }
}

/** Scrolls [list] at a fixed speed, turning at each end. */
@Composable
internal fun AutoScroll(list: LazyListState, speedDp: Float) {
    val density = LocalDensity.current.density
    LaunchedEffect(list) {
        var down = true
        var last = withFrameNanos { it }
        while (true) {
            withFrameNanos { now ->
                val px = speedDp * density * (now - last) / 1e9f
                last = now
                val moved = list.dispatchRawDelta(if (down) px else -px)
                if (abs(moved) < px / 2) down = !down
            }
        }
    }
}
