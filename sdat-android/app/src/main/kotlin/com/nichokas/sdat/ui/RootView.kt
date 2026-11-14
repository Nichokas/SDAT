package com.nichokas.sdat.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.spring
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.navigation3.runtime.NavEntry
import com.nichokas.sdat.core.AppTab
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Destination
import com.nichokas.sdat.core.Playback
import com.nichokas.sdat.core.Preferences
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.Secrets
import com.nichokas.sdat.core.Session
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlasmioBackdropHost
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.icons.SfSymbol
import io.github.neoruaa.glasmio.scaffold.navigation.GlasmioNavDisplay
import io.github.neoruaa.glasmio.scaffold.navigation.GlasmioNavHeaderMotion
import io.github.neoruaa.glasmio.ui.GlasmioTabItem
import uniffi.tonearm_ffi.Mix

private val AppTab.label get() = when (this) {
    AppTab.Discover -> "Discover"
    AppTab.Search -> "Search"
    AppTab.Collection -> "My Collection"
}

// Glasmio's tab bar takes its own symbol set, which has no signpost or music.note.house, so these are the closest.
private val AppTab.symbol get() = when (this) {
    AppTab.Discover -> SfSymbol.SafariFilled
    AppTab.Search -> SfSymbol.Search
    AppTab.Collection -> SfSymbol.MusicNoteSquareStackFilled
}

internal val tabItems = AppTab.entries.map { GlasmioTabItem(it, it.label, it.symbol) }

/** Login until signed in, then one back stack per tab under the bottom chrome and the toast. */
@Composable
fun RootView(session: Session, playback: Playback, prefs: Preferences, secrets: Secrets) {
    LaunchedEffect(session.signedIn) {
        if (!session.signedIn) {
            playback.stop()
            Chrome.reset()
        }
    }
    val colors = GlasmioTheme.colors
    if (!session.signedIn) {
        // Glass controls throw outside a backdrop host.
        return GlasmioBackdropHost(
            modifier = Modifier.fillMaxSize(),
            sampledContent = { Box(Modifier.fillMaxSize().background(colors.groupedBackground)) },
            overlayContent = { LoginView(session) },
        )
    }
    val nav = Chrome.nav
    val stacks = rememberSaveableStateHolder()
    // The Discover header's shuffle sits in the overlay, where glass can sample the page, so the mixes live here.
    var mixes by remember { mutableStateOf(emptyList<Mix>()) }
    val discoverMixes: (List<Mix>) -> Unit = {
        mixes = it
        playback.resumeInfinite(it)
    }
    GlasmioBackdropHost(
        modifier = Modifier.fillMaxSize(),
        sampledContent = {
            // Each tab keeps its pages' saved state (scroll positions) while another tab is shown.
            stacks.SaveableStateProvider(nav.tab.name) {
                TabStack(nav.tab, nav.stack) { route ->
                    when (route) {
                        null -> TabRoot(nav.tab, session, playback, discoverMixes)
                        Route.Settings -> SettingsView(session, prefs, secrets, playback::setStreamingQuality, playback::setPreloadNext, onBack = Chrome::pop)
                        else -> PlaceholderPage(route.title)
                    }
                }
            }
        },
        overlayContent = {
            if (!nav.inDetail) ProfileButton(session, { Chrome.push(Route.Settings) }, Modifier.align(Alignment.TopEnd).statusBarsPadding().padding(end = 16.dp, top = 4.dp))
            if (!nav.inDetail && nav.tab == AppTab.Discover) {
                ShuffleButton(mixes.isNotEmpty(), { playback.playInfinite(mixes) }, Modifier.align(Alignment.TopEnd).statusBarsPadding().padding(end = 70.dp, top = 4.dp))
            }
            if (!nav.inSettings) BottomChrome(playback, nav, Modifier.align(Alignment.BottomCenter))
            Toast(Chrome.toast, Modifier.align(Alignment.TopCenter))
        },
    )
}

private val Route.title get() = when (this) {
    is Route.Album -> "Album"
    is Route.Playlist -> "Playlist"
    is Route.Artist -> "Artist"
    is Route.Explore -> title
    Route.CollectionTracks -> "Tracks"
    Route.CollectionAlbums -> "Albums"
    Route.CollectionArtists -> "Artists"
    Route.CollectionPlaylists -> "Playlists"
    Route.Downloads -> "Downloads"
    Route.Settings -> "Settings"
}

/** iOS push and pop transitions, swipe back and predictive back for one tab; [page] gets null for the tab root. */
@Composable
private fun TabStack(tab: AppTab, stack: List<Destination>, page: @Composable (Route?) -> Unit) {
    val backStack: List<Any> = listOf(tab) + stack
    val motion = remember { GlasmioNavHeaderMotion(contentKey(backStack.last())) }
    GlasmioNavDisplay(
        backStack = backStack,
        motion = motion,
        onBack = Chrome::pop,
        entryProvider = { key -> NavEntry(key, contentKey(key)) { page((key as? Destination)?.route) } },
    )
}

private fun contentKey(key: Any): Any = if (key is Destination) key.id else key.toString()

@Composable
private fun TabRoot(tab: AppTab, session: Session, playback: Playback, onMixes: (List<Mix>) -> Unit) = when (tab) {
    AppTab.Discover -> DiscoverView(session, onPlay = playback::playDiscover, onMixes = onMixes)
    AppTab.Search, AppTab.Collection -> SdatPage(tab.label, collapsesChrome = true) {}
}

@Composable
private fun Toast(text: String?, modifier: Modifier) {
    AnimatedContent(
        targetState = text,
        modifier = modifier.statusBarsPadding().padding(top = 8.dp),
        transitionSpec = {
            (slideInVertically(spring()) { -it } + fadeIn(spring())) togetherWith (slideOutVertically(spring()) { -it } + fadeOut(spring()))
        },
        label = "toast",
    ) { shown ->
        if (shown != null) {
            GlassSurface(shape = capsule) {
                Text(shown, style = IosTypography.subheadline, fontWeight = FontWeight.SemiBold, modifier = Modifier.padding(horizontal = 16.dp, vertical = 10.dp))
            }
        }
    }
}
