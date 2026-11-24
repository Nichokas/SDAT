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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.min
import coil3.compose.AsyncImage
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Fav
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.friendlyMessage
import com.nichokas.sdat.ui.kit.Button
import com.nichokas.sdat.ui.kit.ButtonSize
import com.nichokas.sdat.ui.kit.IconButton
import com.nichokas.sdat.ui.kit.MenuAction
import com.nichokas.sdat.ui.kit.PinnedPage
import com.nichokas.sdat.ui.kit.PopupMenu
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import uniffi.tonearm_ffi.FavouriteKind
import uniffi.tonearm_ffi.Track

/**
 * A pushed page over the blurred [backdropUrl], whose title shows in the top bar once the first item scrolls away.
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
    val scrolled by remember { derivedStateOf { listState.canScrollBackward } }
    PinnedPage(
        title = title,
        onNavigateBack = Chrome::pop,
        actions = actions,
        collapseProgress = collapse,
        scrolled = scrolled,
        backgroundColor = Color.Black,
        background = { CoverBackdrop(backdropUrl, Modifier.matchParentSize()) },
    ) { padding ->
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

/** The cover zoomed in and blurred into colour fields, darkened towards the bottom; black without a cover. */
@Composable
fun CoverBackdrop(url: String?, modifier: Modifier = Modifier) {
    Box(modifier.background(Color.Black).clipToBounds()) {
        if (url != null) {
            val context = LocalContext.current
            val request = remember(url, context) { backdropRequest(context, url, saturation = 1.8f, radius = 5) }
            AsyncImage(
                model = request,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                // Well past the screen: only the cover's centre shows, never an edge.
                modifier = Modifier.fillMaxSize().graphicsLayer { scaleX = 2.2f; scaleY = 2.2f },
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
        SdatIcon("wifi.exclamationmark", null, size = 40.dp, tint = SdatTheme.colors.secondaryContent)
        Text("Couldn't load", style = SdatType.title2, fontWeight = FontWeight.Bold)
        Text(message, style = SdatType.body, color = SdatTheme.colors.secondaryContent, textAlign = TextAlign.Center)
        Button(onRetry, Modifier.padding(top = 8.dp), size = ButtonSize.Medium) { Text("Retry") }
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
    onRemoved: (String) -> Unit,
    onRetry: (() -> Unit)? = null,
    paginationError: String? = null,
    onRetryMore: (() -> Unit)? = null,
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
                IconButton(toggleFav) {
                    SdatIcon(if (isFav) "checkmark" else "plus", if (isFav) "Remove from Collection" else "Add to Collection", size = 20.dp)
                }
            }
            val menu = buildList {
                // An empty list queues nothing, so it offers nothing to confirm.
                if (tracks.isNotEmpty()) {
                    add(MenuAction("Play Next", "text.line.first.and.arrowtriangle.forward", onClick = {
                        if (!Live.userPlayNext(tracks)) { player.playNext(tracks); Chrome.flash("Playing next") }
                    }))
                    add(MenuAction("Add to Queue", "text.line.last.and.arrowtriangle.forward", onClick = {
                        if (!Live.userEnqueue(tracks)) { player.addToQueue(tracks); Chrome.flash("Added to queue") }
                    }))
                    addAll(downloadMenuItems(tracks))
                }
                if (favourite != null) {
                    add(MenuAction(if (isFav) "Remove from Collection" else "Add to Collection", if (isFav) "star.slash" else "star", separatorBefore = isNotEmpty(), onClick = toggleFav))
                }
            }
            if (menu.isNotEmpty()) DetailMenu(title, menu)
        },
    ) {
        item(key = "header") {
            Column(Modifier.fillMaxWidth().padding(horizontal = ListGutter, vertical = 8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Artwork(cover, mosaic)
                Text(title, style = SdatType.title2, fontWeight = FontWeight.Bold, textAlign = TextAlign.Center, modifier = Modifier.padding(top = 18.dp))
                if (subtitleRoute != null) {
                    Text(
                        subtitle,
                        style = SdatType.body,
                        color = SdatTheme.colors.accent,
                        textAlign = TextAlign.Center,
                        modifier = Modifier.padding(top = 6.dp).clickable(role = Role.Button) { Chrome.push(subtitleRoute) },
                    )
                } else {
                    Text(subtitle, style = SdatType.body, color = SdatTheme.colors.secondaryContent, textAlign = TextAlign.Center, modifier = Modifier.padding(top = 6.dp))
                }
                PlayShuffleButtons(
                    play = { player.playInOrder(tracks, title) },
                    shuffle = { player.playShuffled(tracks, title) },
                    enabled = tracks.isNotEmpty(),
                    modifier = Modifier.padding(top = 18.dp),
                )
                if (paginationError != null && onRetryMore != null) {
                    Text("Couldn't load all tracks", style = SdatType.body, modifier = Modifier.padding(top = 16.dp))
                    Text(paginationError, style = SdatType.footnote, color = SdatTheme.colors.secondaryContent, textAlign = TextAlign.Center)
                    Button(onRetryMore, Modifier.padding(top = 8.dp), size = ButtonSize.Medium) { Text("Retry") }
                }
            }
        }
        trackItems(tracks, source = title, showCover = !numbered, numbered = numbered, ownPlaylistId = ownPlaylistId, onRemoved = onRemoved)
        if (loadingMore) {
            item(key = "loading") {
                Box(Modifier.fillMaxWidth().padding(vertical = 12.dp), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(Modifier.size(24.dp), color = SdatTheme.colors.secondaryContent, strokeWidth = 2.dp)
                }
            }
        }
        item(key = "footer") {
            Text(footer, style = SdatType.footnote, color = SdatTheme.colors.secondaryContent, modifier = Modifier.padding(start = ListGutter, top = 8.dp, end = ListGutter))
        }
        if (onRetry != null) {
            item(key = "retry") {
                Button(onRetry, Modifier.padding(start = ListGutter, top = 12.dp), size = ButtonSize.Medium) { Text("Retry") }
            }
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
fun DetailMenu(title: String, items: List<MenuAction>) {
    var bounds by remember { mutableStateOf<Rect?>(null) }
    var open by remember { mutableStateOf(false) }
    IconButton({ open = true }, Modifier.onGloballyPositioned { bounds = it.boundsInRoot() }) {
        SdatIcon("ellipsis", "More", size = 20.dp)
    }
    if (open) {
        PopupMenu(
            anchorBounds = bounds,
            actions = items,
            title = title,
            onDismiss = { open = false },
        )
    }
}
