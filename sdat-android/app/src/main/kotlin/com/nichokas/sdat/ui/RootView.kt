package com.nichokas.sdat.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.onLongClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.navigation3.runtime.NavEntry
import com.nichokas.sdat.core.AppTab
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Destination
import com.nichokas.sdat.core.Morph
import com.nichokas.sdat.core.NavState
import com.nichokas.sdat.core.Playback
import com.nichokas.sdat.core.Preferences
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.Secrets
import com.nichokas.sdat.core.Session
import com.nichokas.sdat.core.artistLine
import com.nichokas.sdat.core.coverUrl
import com.nichokas.sdat.core.displayTitle
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlasmioBackdropHost
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.icons.SfSymbol
import io.github.neoruaa.glasmio.scaffold.navigation.GlasmioNavDisplay
import io.github.neoruaa.glasmio.scaffold.navigation.GlasmioNavHeaderMotion
import io.github.neoruaa.glasmio.ui.GlasmioNavigationBar
import io.github.neoruaa.glasmio.ui.GlasmioTabItem
import kotlinx.coroutines.withTimeoutOrNull
import uniffi.tonearm_ffi.Track

private val capsule = RoundedCornerShape(percent = 50)

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

private val tabItems = AppTab.entries.map { GlasmioTabItem(it, it.label, it.symbol) }

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
    GlasmioBackdropHost(
        modifier = Modifier.fillMaxSize(),
        sampledContent = {
            // Each tab keeps its pages' saved state (scroll positions) while another tab is shown.
            stacks.SaveableStateProvider(nav.tab.name) {
                TabStack(nav.tab, nav.stack) { route ->
                    when (route) {
                        null -> TabRoot(nav.tab, session, playback)
                        Route.Settings -> SettingsView(session, prefs, secrets, onBack = Chrome::pop)
                        else -> PlaceholderPage(route.title)
                    }
                }
            }
        },
        overlayContent = {
            if (!nav.inDetail) ProfileButton(session, { Chrome.push(Route.Settings) }, Modifier.align(Alignment.TopEnd).statusBarsPadding().padding(end = 16.dp, top = 4.dp))
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
private fun TabRoot(tab: AppTab, session: Session, playback: Playback) = when (tab) {
    AppTab.Discover -> DiscoverView(session, onPlay = playback::playDiscover)
    AppTab.Search, AppTab.Collection -> SdatPage(tab.label, collapsesChrome = true) {}
}

private enum class MiniStyle { Circle, Capsule, Wide }

/**
 * iOS bottom chrome. At a tab root the tab bar sits beside the circle mini-player, and scrolling down morphs them into a
 * tab circle beside a capsule. Pushed pages show only a wide mini-player.
 */
@Composable
private fun BottomChrome(playback: Playback, nav: NavState, modifier: Modifier) {
    val hasMini = playback.track != null
    val morphs = hasMini && !nav.inDetail
    val bar = Chrome.bar
    // Snapped to expanded while the morph is hidden, so a mini-player appearing or a return from a pushed page animates in
    // from the start.
    val clock = remember { Animatable(0f) }
    LaunchedEffect(morphs, bar.collapsed) {
        if (morphs) clock.animateTo(if (bar.collapsed) 1f else 0f, tween(Morph.DURATION_MS, easing = LinearEasing)) else clock.snapTo(0f)
    }
    BoxWithConstraints(modifier.fillMaxWidth().navigationBarsPadding().padding(start = 24.dp, end = 24.dp, bottom = 12.dp)) {
        val f = Morph.frame(maxWidth.value, if (morphs) clock.value else 0f, bar.collapsing)
        // Glasmio lerps the bar from its width constraint to compactSize, which at tabMax is exactly Morph's tab width.
        val compact = rememberUpdatedState(if (morphs) f.tabFraction else 0f)
        if (!nav.inDetail) {
            GlasmioNavigationBar(
                items = tabItems,
                selectedKey = nav.tab,
                onSelected = Chrome::select,
                onExpand = { Chrome.setCollapsed(false) },
                compactProgress = compact,
                compactSize = Morph.ROUND.dp,
                modifier = Modifier.align(Alignment.CenterStart)
                    .then(if (hasMini) Modifier.width(Morph.tabMax(maxWidth.value).dp) else Modifier.fillMaxWidth()),
            )
        }
        if (hasMini) {
            val wide = nav.inDetail
            MiniPlayer(
                playback,
                style = when {
                    wide -> MiniStyle.Wide
                    f.miniFraction >= 0.5f -> MiniStyle.Capsule
                    else -> MiniStyle.Circle
                },
                fraction = if (wide) 1f else f.miniFraction,
                modifier = Modifier.align(Alignment.CenterEnd).then(
                    if (wide) Modifier.fillMaxWidth().height(Morph.CIRCLE.dp)
                    else Modifier.width(f.miniWidth.dp).height((Morph.CIRCLE - 8 * f.miniFraction).dp),
                ),
            )
        }
    }
}

/**
 * The iOS mini-player. Circle: a tap opens the player and a short hold toggles play. Capsule and wide: a tap opens the
 * player and the ring toggles play; wide adds next. [fraction] runs the crossfade from circle (0) to capsule (1).
 */
@Composable
private fun MiniPlayer(playback: Playback, style: MiniStyle, fraction: Float, modifier: Modifier) {
    val track = playback.track
    var showPlayer by remember { mutableStateOf(false) }
    val openPlayer = { showPlayer = true }
    GlassSurface(modifier, shape = capsule) {
        if (track != null) {
            CapsuleContent(
                playback,
                track,
                wide = style == MiniStyle.Wide,
                onOpen = openPlayer,
                modifier = Modifier.align(Alignment.CenterEnd).morphLayer(
                    if (style == MiniStyle.Wide) 1f else Morph.inAlpha(fraction),
                    if (style == MiniStyle.Wide) 1f else Morph.inScale(fraction),
                    pivotX = 1f,
                ),
                enabled = style != MiniStyle.Circle,
            )
        }
        CircleContent(
            playback,
            onOpen = openPlayer,
            modifier = Modifier.align(Alignment.CenterEnd)
                .morphLayer(if (style == MiniStyle.Wide) 0f else Morph.outAlpha(fraction), Morph.outScale(fraction)),
            enabled = style == MiniStyle.Circle,
        )
    }
    if (showPlayer && track != null) PlayerSheet(track) { showPlayer = false }
}

private fun Modifier.morphLayer(alpha: Float, scale: Float, pivotX: Float = 0.5f) = graphicsLayer {
    this.alpha = alpha
    scaleX = scale
    scaleY = scale
    transformOrigin = TransformOrigin(pivotX, 0.5f)
}

@Composable
private fun CircleContent(playback: Playback, onOpen: () -> Unit, modifier: Modifier, enabled: Boolean) {
    val track = playback.track
    val label = playLabel(playback)
    val active = enabled && track != null
    Box(
        modifier.width(Morph.CIRCLE.dp).fillMaxHeight()
            .semantics {
                if (!enabled) return@semantics
                role = Role.Button
                contentDescription = track?.let { "$label ${it.title}" } ?: label
                if (track == null) disabled()
                onClick("Open player") { onOpen(); true }
                onLongClick(label) { playback.toggle(); true }
            }
            // Only while active: the faded circle sits over the capsule's ring and next, and would take their touches.
            .then(if (!active) Modifier else Modifier.pointerInput(Unit) {
                awaitEachGesture {
                    awaitFirstDown()
                    // iOS PressRecognizer.miniHold: released within 0.12 s is a tap, otherwise a hold.
                    var up: PointerInputChange? = null
                    val released = withTimeoutOrNull(120) { up = waitForUpOrCancellation(); true }
                    // A cancelled gesture (finger slid off, or consumed by another gesture) does nothing.
                    if (released == null) playback.toggle() else if (up != null) onOpen()
                }
            }),
        contentAlignment = Alignment.Center,
    ) {
        RingIcon(playback, Morph.CIRCLE.dp)
    }
}

@Composable
private fun CapsuleContent(playback: Playback, track: Track, wide: Boolean, onOpen: () -> Unit, modifier: Modifier, enabled: Boolean) {
    val colors = GlasmioTheme.colors
    Row(
        modifier.fillMaxSize()
            .clickable(enabled = enabled, onClickLabel = "Open player", interactionSource = null, indication = null, onClick = onOpen)
            .padding(start = 9.dp, end = 10.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Cover(track.coverUrl, 38.dp)
        Column(Modifier.weight(1f)) {
            Text(track.displayTitle, style = IosTypography.subheadline, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(track.artistLine, style = IosTypography.caption1, color = colors.secondaryContent, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        val label = playLabel(playback)
        Box(
            Modifier.clickable(enabled = enabled, onClickLabel = label, role = Role.Button, interactionSource = null, indication = null) { playback.toggle() }
                .semantics { contentDescription = label },
        ) { RingIcon(playback, 40.dp) }
        if (wide) {
            Box(
                Modifier.size(36.dp, 40.dp).clickable(enabled = enabled, role = Role.Button, interactionSource = null, indication = null) { playback.next() },
                contentAlignment = Alignment.Center,
            ) { SdatIcon("forward.fill", contentDescription = "Next", size = 18.dp, tint = colors.content) }
        }
    }
}

private fun playLabel(playback: Playback) = when {
    playback.error != null -> "Retry playback"
    playback.playing -> "Pause"
    else -> "Play"
}

@Composable
private fun RingIcon(playback: Playback, size: Dp) {
    val symbol = when {
        playback.error != null -> "exclamationmark.triangle.fill"
        playback.playing -> "pause.fill"
        else -> "play.fill"
    }
    val tint = if (playback.track == null) GlasmioTheme.colors.tertiaryContent else GlasmioTheme.colors.content
    Box(Modifier.size(size), contentAlignment = Alignment.Center) {
        // Hidden, not removed, while loading: the ring spins instead of a spinner.
        SdatIcon(symbol, contentDescription = null, size = size * 0.32f, tint = tint, modifier = Modifier.alpha(if (playback.loading) 0f else 1f))
        ProgressRing({ playback.progress }, playback.loading, Modifier.fillMaxSize().padding(size * 0.14f))
    }
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
