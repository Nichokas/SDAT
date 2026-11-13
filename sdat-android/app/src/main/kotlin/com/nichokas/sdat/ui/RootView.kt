package com.nichokas.sdat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.onLongClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import kotlinx.coroutines.withTimeoutOrNull
import uniffi.tonearm_ffi.Track
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.ui.platform.LocalLayoutDirection
import com.nichokas.sdat.core.Preferences
import com.nichokas.sdat.core.Secrets
import com.nichokas.sdat.core.Session
import androidx.compose.foundation.layout.statusBarsPadding
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
fun RootView(session: Session, playback: Playback, prefs: Preferences, secrets: Secrets) {
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
    var settings by rememberSaveable { mutableStateOf(false) }
    val expanded = remember { mutableFloatStateOf(0f) }
    GlasmioBackdropHost(
        modifier = Modifier.fillMaxSize(),
        sampledContent = {
            if (settings) {
                SettingsView(session, prefs, secrets, onBack = { settings = false })
            } else if (tab == AppTab.Discover) {
                DiscoverView(session, onPlay = playback::playDiscover)
            } else {
                Column(Modifier.fillMaxSize().background(colors.groupedBackground).safeDrawingPadding().padding(20.dp)) {
                    Text(tab.label, style = IosTypography.largeTitle)
                }
            }
        },
        overlayContent = {
            if (!settings) ProfileButton(session, { settings = true }, Modifier.align(Alignment.TopEnd).statusBarsPadding().padding(end = 16.dp, top = 4.dp))
            Row(
                Modifier.align(Alignment.BottomCenter).navigationBarsPadding().padding(horizontal = 24.dp, vertical = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                GlasmioNavigationBar(
                    items = tabItems,
                    selectedKey = tab,
                    onSelected = { tab = it; settings = false },
                    onExpand = {},
                    compactProgress = expanded,
                    compactSize = 62.dp,
                    modifier = Modifier.weight(1f),
                )
                if (!settings) MiniPlayer(playback)
            }
        },
    )
}


private val tabItems = AppTab.entries.map { GlasmioTabItem(it, it.label, it.symbol) }

/** Circle state of the iOS mini-player: tap opens the player, a short hold toggles play/pause; the capsule morph comes later. */
@Composable
private fun MiniPlayer(playback: Playback) {
    val track = playback.track
    var showPlayer by remember { mutableStateOf(false) }
    val (symbol, label) = when {
        playback.error != null -> "exclamationmark.triangle.fill" to "Playback failed, hold to retry"
        playback.playing -> "pause.fill" to "Pause"
        else -> "play.fill" to "Play"
    }
    val description = track?.let { "$label ${it.title}" } ?: label
    GlassSurface(Modifier.size(62.dp), shape = capsule) {
        Box(
            Modifier.fillMaxSize()
                .semantics {
                    role = Role.Button
                    contentDescription = description
                    if (track == null) disabled()
                    onClick("Open player") { showPlayer = true; true }
                    onLongClick(label) { playback.toggle(); true }
                }
                .pointerInput(track != null) {
                    if (track == null) return@pointerInput
                    awaitEachGesture {
                        awaitFirstDown()
                        // iOS PressRecognizer.miniHold: released within 0.12 s is a tap, otherwise a hold.
                        var up: PointerInputChange? = null
                        val released = withTimeoutOrNull(120) { up = waitForUpOrCancellation(); true }
                        // A cancelled gesture (finger slid off, or consumed by another gesture) does nothing.
                        if (released == null) playback.toggle() else if (up != null) showPlayer = true
                    }
                },
            contentAlignment = Alignment.Center,
        ) {
            val tint = if (track == null) GlasmioTheme.colors.tertiaryContent else GlasmioTheme.colors.content
            // Hidden, not removed, while loading: the ring spins instead of a spinner.
            SdatIcon(symbol, contentDescription = null, size = 20.dp, tint = tint, modifier = Modifier.alpha(if (playback.loading) 0f else 1f))
            ProgressRing({ playback.progress }, playback.loading, Modifier.fillMaxSize().padding(62.dp * 0.14f))
        }
    }
    if (showPlayer && track != null) PlayerSheet(track) { showPlayer = false }
}

/** Placeholder for the iOS player sheet until it is ported. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PlayerSheet(track: Track, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.fillMaxWidth().padding(24.dp)) {
            Text(track.title, style = IosTypography.headline)
            Text(track.artists.joinToString { it.name }, style = IosTypography.footnote)
        }
    }
}
