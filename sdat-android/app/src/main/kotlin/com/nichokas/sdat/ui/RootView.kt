package com.nichokas.sdat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nichokas.sdat.core.coreLoaded
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.glass.GlasmioBackdropHost

enum class AppTab(val label: String, val symbol: String) {
    Discover("Discover", "signpost.right.and.left.fill"),
    Search("Search", "magnifyingglass"),
    Collection("My Collection", "music.note.house.fill"),
}

private val capsule = RoundedCornerShape(percent = 50)

/** Placeholder shell: the three root tabs and the bottom chrome (tab bar beside the circular mini-player). */
@Composable
fun RootView() {
    var tab by rememberSaveable { mutableStateOf(AppTab.Discover) }
    val core = remember { coreLoaded() }
    val colors = GlasmioTheme.colors
    GlasmioBackdropHost(
        modifier = Modifier.fillMaxSize(),
        sampledContent = {
            Column(Modifier.fillMaxSize().background(colors.groupedBackground).safeDrawingPadding().padding(20.dp)) {
                Text(tab.label, style = IosTypography.largeTitle)
                Spacer(Modifier.height(8.dp))
                Text(if (core) "Rust core loaded" else "Rust core failed to load", style = IosTypography.body)
            }
        },
        overlayContent = {
            Row(
                Modifier.align(Alignment.BottomCenter).navigationBarsPadding().padding(horizontal = 24.dp, vertical = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                TabBar(tab, onSelect = { tab = it }, Modifier.weight(1f))
                MiniPlayer()
            }
        },
    )
}


@Composable
private fun TabBar(selected: AppTab, onSelect: (AppTab) -> Unit, modifier: Modifier = Modifier) {
    GlassSurface(modifier.height(62.dp), shape = capsule) {
        Row(Modifier.fillMaxSize().padding(4.dp)) {
            AppTab.entries.forEach { t ->
                val tint = if (t == selected) GlasmioTheme.colors.accent else GlasmioTheme.colors.content
                Column(
                    Modifier.weight(1f).fillMaxHeight().clickable { onSelect(t) },
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center,
                ) {
                    SdatIcon(t.symbol, contentDescription = null, size = 22.dp, tint = tint)
                    Text(t.label, color = tint, fontSize = 10.sp, maxLines = 1)
                }
            }
        }
    }
}

/** Circle state of the iOS mini-player; morphing into the capsule comes with playback. */
@Composable
private fun MiniPlayer() {
    GlassSurface(Modifier.size(62.dp), shape = capsule) {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            SdatIcon("play.fill", contentDescription = "Play", size = 24.dp)
        }
    }
}
