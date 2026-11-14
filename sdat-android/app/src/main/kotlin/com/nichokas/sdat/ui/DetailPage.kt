package com.nichokas.sdat.ui

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ColorMatrix
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.min
import coil3.compose.AsyncImage
import com.nichokas.sdat.core.Fav
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.friendlyMessage
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.LocalGlassBackdrop
import io.github.neoruaa.glasmio.glass.glassBackdropSource
import io.github.neoruaa.glasmio.glass.render.backdrops.rememberLayerBackdrop
import io.github.neoruaa.glasmio.glass.trackBackdropPosition
import io.github.neoruaa.glasmio.scaffold.GlasmioPinnedPage
import io.github.neoruaa.glasmio.scaffold.rememberGlasmioHeaderScrollEdge
import io.github.neoruaa.glasmio.ui.GlasmioButton
import io.github.neoruaa.glasmio.ui.GlasmioButtonSize
import io.github.neoruaa.glasmio.ui.GlasmioCascadingMenu
import io.github.neoruaa.glasmio.ui.GlasmioCascadingMenuItem
import io.github.neoruaa.glasmio.ui.GlasmioIconButton
import uniffi.tonearm_ffi.FavouriteKind
import uniffi.tonearm_ffi.Track

/**
 * A pushed page over the blurred [backdropUrl], whose title shows in the top bar once the first item scrolls away.
 * Glass in [content] samples only the backdrop: the page itself is the app's sampled layer.
 */
@Composable
fun DetailScaffold(
    title: String,
    backdropUrl: String?,
    actions: @Composable RowScope.() -> Unit = {},
    content: LazyListScope.() -> Unit,
) {
    val listState = rememberLazyListState()
    val pastHeader by remember { derivedStateOf { listState.firstVisibleItemIndex > 0 } }
    val collapse by animateFloatAsState(if (pastHeader) 1f else 0f, label = "title")
    val glass = rememberLayerBackdrop()
    GlasmioPinnedPage(
        title = title,
        onNavigateBack = Chrome::pop,
        actions = actions,
        collapseProgress = collapse,
        topBarBlurProgress = rememberGlasmioHeaderScrollEdge(listState),
        backgroundColor = Color.Black,
        background = { CoverBackdrop(backdropUrl, Modifier.matchParentSize().glassBackdropSource(glass).trackBackdropPosition(glass)) },
    ) { padding ->
        CompositionLocalProvider(LocalGlassBackdrop provides glass) {
            LazyColumn(
                state = listState,
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(
                    top = padding.calculateTopPadding(),
                    bottom = padding.calculateBottomPadding() + BottomChromeSpace,
                ),
                content = content,
            )
        }
    }
}

/** The cover zoomed in and blurred into colour fields, darkened towards the bottom; black without a cover. */
@Composable
fun CoverBackdrop(url: String?, modifier: Modifier = Modifier) {
    Box(modifier.background(Color.Black).clipToBounds()) {
        if (url != null) {
            AsyncImage(
                // Blurred this far, a small cover looks the same and decodes much faster.
                model = CoverUrl.sized(url, 160),
                contentDescription = null,
                contentScale = ContentScale.Crop,
                colorFilter = ColorFilter.colorMatrix(ColorMatrix().apply { setToSaturation(1.8f) }),
                // Well past the screen: only the cover's centre shows, never an edge.
                modifier = Modifier.fillMaxSize().graphicsLayer { scaleX = 2.2f; scaleY = 2.2f }.blur(50.dp),
            )
            Box(Modifier.fillMaxSize().background(Color.Black.copy(alpha = 0.32f)))
        }
        Box(Modifier.fillMaxSize().background(Brush.verticalGradient(0.5f to Color.Transparent, 1f to Color.Black.copy(alpha = 0.55f))))
    }
}

/** Error page body with a retry. */
@Composable
fun LoadFailed(message: String, onRetry: () -> Unit) {
    Column(
        Modifier.fillMaxWidth().padding(vertical = 80.dp, horizontal = 24.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        SdatIcon("wifi.exclamationmark", null, size = 40.dp, tint = GlasmioTheme.colors.secondaryContent)
        Text("Couldn't load", style = IosTypography.title2, fontWeight = FontWeight.Bold)
        Text(message, style = IosTypography.body, color = GlasmioTheme.colors.secondaryContent, textAlign = TextAlign.Center)
        GlasmioButton(onRetry, Modifier.padding(top = 8.dp), size = GlasmioButtonSize.Medium) { Text("Retry") }
    }
}

/** Album and playlist page: artwork, title, subtitle, Play and Shuffle, the tracks and a footer. */
@Composable
fun DetailPage(
    title: String,
    subtitle: String,
    subtitleRoute: Route?,
    cover: String?,
    mosaic: List<String>,
    tracks: List<Track>,
    loadingMore: Boolean,
    footer: String,
    favourite: Pair<FavouriteKind, String>?,
    numbered: Boolean,
    ownPlaylistId: String?,
    onRemoved: (Track) -> Unit,
) {
    val player = LocalTrackPlayer.current
    val fav = favourite?.let { (kind, id) -> Fav(kind, id) }
    LaunchedEffect(fav) { fav?.let { Favs.check(it) } }
    val isFav = fav?.let(Favs::known) ?: false
    val toggleFav: () -> Unit = { fav?.let { Favs.set(it, !isFav) } }
    DetailScaffold(
        title = title,
        backdropUrl = cover,
        actions = {
            if (favourite != null) {
                GlasmioIconButton(toggleFav) {
                    SdatIcon(if (isFav) "checkmark" else "plus", if (isFav) "Remove from Collection" else "Add to Collection", size = 20.dp)
                }
            }
            DetailMenu(title, buildList {
                add(GlasmioCascadingMenuItem("Play Next", "text.line.first.and.arrowtriangle.forward", onClick = {
                    // With nothing playing, the first Play Next starts its track, so the list would start from its end.
                    if (player.current == null) player.play(tracks) else tracks.asReversed().forEach(player::playNext)
                    Chrome.flash("Playing next")
                }))
                add(GlasmioCascadingMenuItem("Add to Queue", "text.line.last.and.arrowtriangle.forward", onClick = {
                    tracks.forEach(player::addToQueue); Chrome.flash("Added to queue")
                }))
                if (favourite != null) {
                    add(GlasmioCascadingMenuItem(if (isFav) "Remove from Collection" else "Add to Collection", if (isFav) "star.slash" else "star", separatorBefore = true, onClick = toggleFav))
                }
            })
        },
    ) {
        item(key = "header") {
            Column(Modifier.fillMaxWidth().padding(horizontal = ListGutter, vertical = 8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Artwork(cover, mosaic)
                Text(title, style = IosTypography.title2, fontWeight = FontWeight.Bold, textAlign = TextAlign.Center, modifier = Modifier.padding(top = 18.dp))
                if (subtitleRoute != null) {
                    Text(
                        subtitle,
                        style = IosTypography.body,
                        color = GlasmioTheme.colors.accent,
                        textAlign = TextAlign.Center,
                        modifier = Modifier.padding(top = 6.dp).clickable(role = Role.Button) { Chrome.push(subtitleRoute) },
                    )
                } else {
                    Text(subtitle, style = IosTypography.body, color = GlasmioTheme.colors.secondaryContent, textAlign = TextAlign.Center, modifier = Modifier.padding(top = 6.dp))
                }
                PlayShuffleButtons(
                    play = { player.playInOrder(tracks, title) },
                    shuffle = { player.playShuffled(tracks, title) },
                    enabled = tracks.isNotEmpty(),
                    modifier = Modifier.padding(top = 18.dp),
                )
            }
        }
        trackItems(tracks, source = title, showCover = !numbered, numbered = numbered, ownPlaylistId = ownPlaylistId, onRemoved = onRemoved)
        if (loadingMore) {
            item(key = "loading") {
                Box(Modifier.fillMaxWidth().padding(vertical = 12.dp), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(Modifier.size(24.dp), color = GlasmioTheme.colors.secondaryContent, strokeWidth = 2.dp)
                }
            }
        }
        item(key = "footer") {
            Text(footer, style = IosTypography.footnote, color = GlasmioTheme.colors.secondaryContent, modifier = Modifier.padding(start = ListGutter, top = 8.dp, end = ListGutter))
        }
    }
}

/** Up to 270 dp wide; a 2x2 grid when the playlist has four distinct album covers. */
@Composable
private fun Artwork(cover: String?, mosaic: List<String>) {
    BoxWithConstraints(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        val side = min(maxWidth, 270.dp)
        val shape = RoundedCornerShape(14.dp)
        Box(Modifier.shadow(24.dp, shape, ambientColor = Color.Black, spotColor = Color.Black)) {
            if (mosaic.size == 4) {
                Column(Modifier.clip(shape)) {
                    mosaic.chunked(2).forEach { row ->
                        Row { row.forEach { Cover(it, side / 2, radius = 0.dp) } }
                    }
                }
            } else {
                Cover(cover, side, radius = 14.dp)
            }
        }
    }
}

/** The top bar's ellipsis button and the menu it opens. */
@Composable
fun DetailMenu(title: String, items: List<GlasmioCascadingMenuItem>) {
    var bounds by remember { mutableStateOf<Rect?>(null) }
    var open by remember { mutableStateOf(false) }
    GlasmioIconButton({ open = true }, Modifier.onGloballyPositioned { bounds = it.boundsInRoot() }) {
        SdatIcon("ellipsis", "More", size = 20.dp)
    }
    if (open) {
        GlasmioCascadingMenu(
            anchorBounds = bounds,
            items = items,
            title = title,
            expandedDescription = "Expanded",
            collapsedDescription = "Collapsed",
            onDismiss = { open = false },
        )
    }
}
