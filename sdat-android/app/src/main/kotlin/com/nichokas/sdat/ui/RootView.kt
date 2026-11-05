package com.nichokas.sdat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.runtime.LaunchedEffect
import com.nichokas.sdat.core.Playback
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
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
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
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableFloatStateOf
import io.github.neoruaa.glasmio.ui.GlasmioTabItem
import io.github.neoruaa.glasmio.ui.GlasmioNavigationBar
import io.github.neoruaa.glasmio.icons.SfSymbol
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.glass.GlasmioBackdropHost

// Glasmio's tab bar takes its own symbol set, which has no signpost or music.note.house, so these are the closest.
enum class AppTab(val label: String, val symbol: SfSymbol) {
    Discover("Discover", SfSymbol.SafariFilled),
    Search("Search", SfSymbol.Search),
    Collection("My Collection", SfSymbol.MusicNoteSquareStackFilled),
}

private val capsule = RoundedCornerShape(percent = 50)

/** Login until signed in, then the three root tabs and the bottom chrome (tab bar beside the circular mini-player). */
@Composable
fun RootView(session: Session, playback: Playback) {
    LaunchedEffect(session.signedIn) { if (!session.signedIn) playback.stop() }
    val colors = GlasmioTheme.colors
    if (!session.signedIn) {
        // Glass controls throw outside a backdrop host.
        return GlasmioBackdropHost(
            modifier = Modifier.fillMaxSize(),
            sampledContent = { Box(Modifier.fillMaxSize().background(colors.groupedBackground)) },
            overlayContent = { LoginView(session) },
        )
    }
    var tab by rememberSaveable { mutableStateOf(AppTab.Discover) }
    val expanded = remember { mutableFloatStateOf(0f) }
    GlasmioBackdropHost(
        modifier = Modifier.fillMaxSize(),
        sampledContent = {
            if (tab == AppTab.Discover) {
                DiscoverView(session, onPlay = playback::play)
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
                GlasmioNavigationBar(
                    items = tabItems,
                    selectedKey = tab,
                    onSelected = { tab = it },
                    onExpand = {},
                    compactProgress = expanded,
                    compactSize = 62.dp,
                    modifier = Modifier.weight(1f),
                )
                MiniPlayer(playback)
            }
        },
    )
}


private val tabItems = AppTab.entries.map { GlasmioTabItem(it, it.label, it.symbol) }

/** Circle state of the iOS mini-player: play/pause for the current track; the capsule morph comes later. */
@Composable
private fun MiniPlayer(playback: Playback) {
    val track = playback.track
    val (symbol, label) = when {
        playback.error != null -> "exclamationmark.triangle.fill" to "Playback failed, tap to retry"
        playback.playing -> "pause.fill" to "Pause"
        else -> "play.fill" to "Play"
    }
    val description = track?.let { "$label ${it.title}" } ?: label
    GlassSurface(Modifier.size(62.dp), shape = capsule) {
        Box(
            Modifier.fillMaxSize().clickable(enabled = track != null, role = Role.Button, onClick = playback::toggle)
                .semantics { contentDescription = description },
            contentAlignment = Alignment.Center,
        ) {
            if (playback.loading) {
                CircularProgressIndicator(Modifier.size(24.dp), strokeWidth = 2.dp)
            } else {
                val tint = if (track == null) GlasmioTheme.colors.tertiaryContent else GlasmioTheme.colors.content
                SdatIcon(symbol, contentDescription = null, size = 24.dp, tint = tint)
            }
        }
    }
}
