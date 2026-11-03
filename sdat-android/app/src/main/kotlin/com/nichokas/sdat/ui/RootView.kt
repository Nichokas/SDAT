package com.nichokas.sdat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.ui.platform.LocalLayoutDirection
import com.nichokas.sdat.core.Session
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

/** Login until signed in, then the three root tabs and the bottom chrome (tab bar beside the circular mini-player). */
@Composable
fun RootView(session: Session) {
    if (!session.signedIn) return LoginView(session)
    var tab by rememberSaveable { mutableStateOf(AppTab.Discover) }
    val colors = GlasmioTheme.colors
    GlasmioBackdropHost(
        modifier = Modifier.fillMaxSize(),
        sampledContent = {
            if (tab == AppTab.Discover) {
                DiscoverView(pagePadding())
            } else {
                Column(Modifier.fillMaxSize().background(colors.groupedBackground).safeDrawingPadding().padding(20.dp)) {
                    Text(tab.label, style = IosTypography.largeTitle)
                }
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


/** Safe area plus room for the floating tab bar. */
@Composable
private fun pagePadding(): PaddingValues {
    val safe = WindowInsets.safeDrawing.asPaddingValues()
    val dir = LocalLayoutDirection.current
    return PaddingValues(
        start = safe.calculateStartPadding(dir) + 20.dp,
        top = safe.calculateTopPadding() + 20.dp,
        end = safe.calculateEndPadding(dir) + 20.dp,
        bottom = safe.calculateBottomPadding() + 96.dp,
    )
}

@Composable
private fun TabBar(selected: AppTab, onSelect: (AppTab) -> Unit, modifier: Modifier = Modifier) {
    GlassSurface(modifier.height(62.dp), shape = capsule) {
        Row(Modifier.fillMaxSize().padding(4.dp).selectableGroup()) {
            AppTab.entries.forEach { t ->
                val tint = if (t == selected) GlasmioTheme.colors.accent else GlasmioTheme.colors.content
                Column(
                    Modifier.weight(1f).fillMaxHeight().selectable(t == selected, role = Role.Tab) { onSelect(t) },
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
