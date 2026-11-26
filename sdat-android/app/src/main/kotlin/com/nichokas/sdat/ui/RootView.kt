package com.nichokas.sdat.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.spring
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.DialogProperties
import androidx.navigation3.runtime.NavEntry
import com.nichokas.sdat.core.AppTab
import com.nichokas.sdat.core.AudioCache
import com.nichokas.sdat.core.Boot
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.CrashReporting
import com.nichokas.sdat.core.Destination
import com.nichokas.sdat.core.Discover
import com.nichokas.sdat.core.Downloads
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.core.PageKey
import com.nichokas.sdat.core.Playback
import com.nichokas.sdat.core.Preferences
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.Secrets
import com.nichokas.sdat.core.Session
import com.nichokas.sdat.ui.kit.AlertButton
import com.nichokas.sdat.ui.kit.AlertButtonLayout
import com.nichokas.sdat.ui.kit.AlertButtonRole
import com.nichokas.sdat.ui.kit.AlertDialog
import com.nichokas.sdat.ui.kit.HeaderHeight
import com.nichokas.sdat.ui.kit.Raised
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import com.nichokas.sdat.ui.kit.Stack
import com.nichokas.sdat.ui.kit.capsule

/** Login until signed in, then one back stack per tab under the bottom chrome and the toast. */
@Composable
fun RootView(session: Session, playback: Playback, prefs: Preferences, secrets: Secrets, crashReporting: CrashReporting) {
    LaunchedEffect(session.signedIn) {
        if (!session.signedIn) {
            playback.stop()
            // The sign-out's purge, once nothing plays from the cache, so it leaves no file behind.
            AudioCache.purge(keepPlaying = false)
            Chrome.reset()
        }
    }
    val colors = SdatTheme.colors
    if (!session.signedIn) {
        return Box(Modifier.fillMaxSize().background(colors.sheet)) {
            LoginView(session)
            // Here too, so a fresh install is asked before sign-in.
            if (crashReporting.shouldAsk) CrashReportsPrompt(crashReporting)
        }
    }
    // A fresh sign-in warms the caches before the shell, and with it Discover's first load, starts.
    if (Boot.active) return BootLoaderView(Boot.progress.value)
    val nav = Chrome.nav
    val stacks = rememberSaveableStateHolder()
    val mixes = Discover.mixes
    LaunchedEffect(mixes) { playback.resumeInfinite(mixes) }
    Box(Modifier.fillMaxSize().background(colors.frame)) {
        Column(Modifier.fillMaxSize()) {
            Box(Modifier.weight(1f)) {
                // Each tab keeps its pages' saved state (scroll positions) while another tab is shown.
                stacks.SaveableStateProvider(nav.tab.name) {
                    TabStack(nav.tab, nav.stack) { route ->
                        when (route) {
                            null -> TabRoot(nav.tab, session, playback)
                            Route.Settings -> SettingsView(session, prefs, secrets, crashReporting, playback::setStreamingQuality, playback::setPreloadNext, onBack = Chrome::pop)
                            Route.Diagnostics -> DiagnosticsView(crashReporting, onBack = Chrome::pop)
                            Route.Country -> CountryView(onBack = Chrome::pop)
                            is Route.Explore -> ExploreView(route.path, route.title)
                            is Route.Album -> AlbumView(route.id)
                            is Route.Playlist -> PlaylistView(route.id)
                            is Route.Artist -> ArtistView(route.id)
                            Route.CollectionTracks -> CollectionTracksView()
                            Route.CollectionAlbums -> CollectionAlbumsView()
                            Route.CollectionArtists -> CollectionArtistsView()
                            Route.CollectionPlaylists -> CollectionPlaylistsView()
                            Route.Downloads -> DownloadsView()
                        }
                    }
                }
                if (!nav.inDetail) {
                    // As on iOS, Discover's header has Shuffle where the other tabs show download progress.
                    Row(
                        Modifier.align(Alignment.TopEnd).statusBarsPadding().height(HeaderHeight).padding(end = 16.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        if (nav.tab == AppTab.Discover) ShuffleButton(mixes.isNotEmpty(), { playback.playInfinite(mixes) })
                        else if (Downloads.queue.isActive) DownloadStatusButton()
                        ProfileButton(session, { Chrome.push(Route.Settings) })
                    }
                }
            }
            // Below the page rather than over it, so the banner never hides a list's last rows.
            if (!nav.inSettings) HandoffBanner(Modifier.fillMaxWidth().background(colors.sheet))
            // Settings hides the bar; the sheet still runs under the gesture area.
            if (nav.inSettings) Box(Modifier.fillMaxWidth().background(colors.sheet).navigationBarsPadding())
            else BottomBar(playback, nav)
        }
        if (!nav.inSettings) MiniFanOverlay(playback)
        // After the bottom chrome, so the picker opens over the player sheet too.
        Chrome.playlistPickerTrack?.let { PlaylistPicker(it) { Chrome.playlistPickerTrack = null } }
        if (Live.showSheet) LiveSheet()
        if (Live.showScanner) ScannerSheet { Live.showScanner = false }
        if (crashReporting.shouldAsk) CrashReportsPrompt(crashReporting)
        Toast(Chrome.toast, Modifier.align(Alignment.TopCenter))
        BootLoaderFadeOut()
    }
}

/** iOS push and pop transitions and predictive back for one tab; [page] gets null for the tab root. */
@Composable
private fun TabStack(tab: AppTab, stack: List<Destination>, page: @Composable (Route?) -> Unit) {
    val backStack: List<Any> = listOf(tab) + stack
    Stack(
        backStack = backStack,
        onBack = Chrome::pop,
        entryProvider = { key ->
            NavEntry(key, contentKey(key)) {
                val dest = key as? Destination
                val pageKey = if (dest != null) PageKey.Pushed(dest.id) else PageKey.Root(key as AppTab)
                CompositionLocalProvider(LocalPage provides pageKey) { page(dest?.route) }
            }
        },
    )
}

private fun contentKey(key: Any): Any = if (key is Destination) key.id else key.toString()

@Composable
private fun TabRoot(tab: AppTab, session: Session, playback: Playback) = when (tab) {
    AppTab.Discover -> DiscoverView(session, onPlay = playback::playDiscover)
    AppTab.Search -> SearchView()
    AppTab.Collection -> CollectionView()
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
            Raised(shape = capsule, color = SdatTheme.colors.inverse, contentColor = SdatTheme.colors.onInverse) {
                Text(shown, style = SdatType.subheadline, fontWeight = FontWeight.SemiBold, modifier = Modifier.padding(horizontal = 16.dp, vertical = 10.dp))
            }
        }
    }
}

/** Asked once, on the first launch; only an answer closes it. */
@Composable
private fun CrashReportsPrompt(crashReporting: CrashReporting) {
    AlertDialog(
        onDismissRequest = {},
        title = "Send crash reports?",
        message = "If SDAT crashes or hits an error, send a report to the developer so it can be fixed. Reports include the error, a stack trace, recent log entries, the app version, and the device model and Android version, but no account details. You can change this in Settings > Diagnostics.",
        buttonLayout = AlertButtonLayout.Stacked,
        properties = DialogProperties(dismissOnBackPress = false, dismissOnClickOutside = false, usePlatformDefaultWidth = false),
        buttons = listOf(
            AlertButton("Send reports", { crashReporting.allow(true) }),
            AlertButton("Don't send", { crashReporting.allow(false) }, AlertButtonRole.Cancel),
        ),
    )
}
