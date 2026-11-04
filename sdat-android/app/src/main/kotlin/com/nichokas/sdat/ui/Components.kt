package com.nichokas.sdat.ui

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.runtime.withFrameMillis
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.Fmt
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.artistLine
import com.nichokas.sdat.core.displayTitle
import com.nichokas.sdat.core.friendlyMessage
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.ui.GlasmioCascadingMenu
import io.github.neoruaa.glasmio.ui.GlasmioCascadingMenuItem
import kotlin.coroutines.cancellation.CancellationException
import kotlin.math.abs
import kotlin.math.sin
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.removeFromPlaylist
import uniffi.tonearm_ffi.trackRadio

/** What track rows and menus need from the player. */
interface TrackPlayer {
    val current: Track?
    val isPlaying: Boolean
    fun play(tracks: List<Track>, startAt: Int = 0, from: String? = null)
    fun playNext(track: Track)
    fun addToQueue(track: Track)
}

val LocalTrackPlayer = staticCompositionLocalOf<TrackPlayer> { error("No TrackPlayer provided") }

/** Picks the TIDAL image size for a display size: covers come as /<uuid>/<N>x<N>.jpg. */
object CoverUrl {
    /** TIDAL serves different square sizes per resource type. */
    enum class Kind(val sizes: List<Int>) {
        Album(listOf(80, 160, 320, 640, 1280)),
        Artist(listOf(160, 320, 480, 750)),
        Profile(listOf(320)),
    }

    private val sizeSuffix = Regex("""/\d+x\d+\.(jpg|png)$""")

    fun sized(url: String, px: Int, kind: Kind = Kind.Album): String {
        if ("resources.tidal.com/images/" !in url || px <= 0) return url
        val match = sizeSuffix.find(url) ?: return url
        val png = url.endsWith(".png")
        val sizes = if (png) Kind.Profile.sizes else kind.sizes
        val size = sizes.firstOrNull { it >= px } ?: sizes.last()
        return url.replaceRange(match.range, "/${size}x$size.${if (png) "png" else "jpg"}")
    }
}

/** Square cover sized for its frame. */
@Composable
fun Cover(url: String?, size: Dp, modifier: Modifier = Modifier, radius: Dp = 8.dp, circle: Boolean = false) {
    val px = with(LocalDensity.current) { size.roundToPx() }
    val kind = if (circle) CoverUrl.Kind.Artist else CoverUrl.Kind.Album
    Box(
        modifier.size(size).clip(if (circle) CircleShape else RoundedCornerShape(radius)).background(Color.White.copy(alpha = 0.08f)),
        contentAlignment = Alignment.Center,
    ) {
        if (url == null) {
            SdatIcon("music.note", null, size = size / 3, tint = GlasmioTheme.colors.secondaryContent)
        } else {
            AsyncImage(
                model = CoverUrl.sized(url, px, kind),
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize(),
            )
        }
    }
}

/** Animated bars shown on the playing row. */
@Composable
fun Equalizer(playing: Boolean, modifier: Modifier = Modifier) {
    var t by remember { mutableDoubleStateOf(0.0) }
    LaunchedEffect(playing) {
        while (playing) withFrameMillis { t = it / 1000.0 }
    }
    Row(modifier.size(15.dp, 14.dp), horizontalArrangement = Arrangement.spacedBy(2.dp), verticalAlignment = Alignment.Bottom) {
        repeat(3) { i ->
            val h = if (playing) 0.35 + 0.65 * abs(sin(t * (5 + i * 1.7) + i)) else 0.35
            Box(Modifier.width(3.dp).height(14.dp * h.toFloat()).background(GlasmioTheme.colors.accent, CircleShape))
        }
    }
}

@Composable
fun TrackRow(track: Track, modifier: Modifier = Modifier, showCover: Boolean = true, number: Int? = null, coverSize: Dp = 44.dp) {
    val player = LocalTrackPlayer.current
    val colors = GlasmioTheme.colors
    val isCurrent = player.current?.id == track.id
    Row(
        modifier.fillMaxWidth().alpha(if (track.streamable) 1f else 0.4f),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (number != null) {
            Box(Modifier.width(24.dp), contentAlignment = Alignment.Center) {
                if (isCurrent) {
                    Equalizer(player.isPlaying)
                } else {
                    Text("$number", style = IosTypography.subheadline.copy(fontFeatureSettings = "tnum"), color = colors.secondaryContent)
                }
            }
        }
        if (showCover) {
            Box(contentAlignment = Alignment.Center) {
                Cover(track.album?.coverUrl, coverSize, radius = 6.dp)
                if (isCurrent && number == null) {
                    Box(Modifier.size(coverSize).clip(RoundedCornerShape(6.dp)).background(Color.Black.copy(alpha = 0.45f)), contentAlignment = Alignment.Center) {
                        Equalizer(player.isPlaying)
                    }
                }
            }
        }
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(
                track.displayTitle,
                style = IosTypography.body,
                color = if (isCurrent) colors.accent else colors.content,
                fontWeight = if (isCurrent) FontWeight.SemiBold else FontWeight.Normal,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            val secondary = if (isCurrent) colors.accent.copy(alpha = 0.8f) else colors.secondaryContent
            Row(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
                if (track.explicit) SdatIcon("e.square.fill", "Explicit", size = 12.dp, tint = secondary)
                Text(track.artistLine, style = IosTypography.subheadline, color = secondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        Text(
            Fmt.time(track.durationSecs.toDouble()),
            style = IosTypography.subheadline.copy(fontFeatureSettings = "tnum"),
            color = colors.secondaryContent,
        )
    }
}

/** Long-press menu items shared by every track row. */
@Composable
fun trackMenuItems(
    track: Track,
    scope: CoroutineScope,
    ownPlaylistId: String? = null,
    onRemoved: (() -> Unit)? = null,
): List<GlasmioCascadingMenuItem> {
    val player = LocalTrackPlayer.current
    val fav = Favs.known(track.id) ?: false
    return buildList {
        add(GlasmioCascadingMenuItem(if (fav) "Remove from Favourites" else "Add to Favourites", if (fav) "star.slash" else "star", onClick = { Favs.toggle(track.id) }))
        add(GlasmioCascadingMenuItem("Play Next", "text.line.first.and.arrowtriangle.forward", separatorBefore = true, onClick = {
            player.playNext(track); Chrome.flash("Playing next")
        }))
        add(GlasmioCascadingMenuItem("Add to Queue", "text.line.last.and.arrowtriangle.forward", onClick = {
            player.addToQueue(track); Chrome.flash("Added to queue")
        }))
        if (ownPlaylistId != null) {
            add(GlasmioCascadingMenuItem("Remove from Playlist", "minus.circle", destructive = true, onClick = {
                scope.launch {
                    try {
                        removeFromPlaylist(ownPlaylistId, track.id)
                        onRemoved?.invoke()
                        Chrome.flash("Removed from playlist")
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Exception) {
                        Chrome.flash(friendlyMessage(e))
                    }
                }
            }))
        }
        add(GlasmioCascadingMenuItem("Start Radio", "dot.radiowaves.left.and.right", separatorBefore = true, onClick = {
            scope.launch {
                val list = runCatching { trackRadio(track.id)?.nextPage() }.getOrNull() ?: return@launch
                player.play(listOf(track) + list, from = "${track.title} Radio")
            }
        }))
        track.album?.let { a ->
            add(GlasmioCascadingMenuItem("Go to Album", "square.stack", onClick = { Chrome.showPlayer = false; Chrome.push(Route.Album(a.id)) }))
        }
        track.artists.firstOrNull()?.let { ar ->
            add(GlasmioCascadingMenuItem("Go to Artist", "music.microphone", onClick = { Chrome.showPlayer = false; Chrome.push(Route.Artist(ar.id)) }))
        }
    }
}

/** Tap plays, long-press opens the track menu; favourite state is fetched lazily once the row settles on screen. */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun TrackMenuRow(
    track: Track,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    ownPlaylistId: String? = null,
    onRemoved: (() -> Unit)? = null,
    content: @Composable () -> Unit,
) {
    var bounds by remember { mutableStateOf<Rect?>(null) }
    var open by remember { mutableStateOf(false) }
    // Outlives the menu: Glasmio dismisses it before running the chosen action.
    val scope = rememberCoroutineScope()
    LaunchedEffect(track.id) {
        delay(600)
        Favs.check(track.id)
    }
    Box(
        modifier
            .onGloballyPositioned { bounds = it.boundsInRoot() }
            .combinedClickable(onLongClick = { open = true }, onClick = onClick),
    ) { content() }
    if (open) {
        GlasmioCascadingMenu(
            anchorBounds = bounds,
            items = trackMenuItems(track, scope, ownPlaylistId, onRemoved),
            title = track.displayTitle,
            expandedDescription = "Expanded",
            collapsedDescription = "Collapsed",
            onDismiss = { open = false },
        )
    }
}

/** Plays a track list and offers per-row actions, as lazy items so off-screen rows don't check favourites. */
fun LazyListScope.trackList(
    tracks: List<Track>,
    source: String? = null,
    showCover: Boolean = true,
    numbered: Boolean = false,
    ownPlaylistId: String? = null,
    onRemoved: ((Track) -> Unit)? = null,
) {
    // The index keeps duplicate tracks apart; a shifted row gets fresh menu state instead of another track's.
    itemsIndexed(tracks, key = { i, t -> "${t.id}#$i" }) { i, t ->
        val player = LocalTrackPlayer.current
        TrackMenuRow(t, onClick = { player.play(tracks, i, source) }, ownPlaylistId = ownPlaylistId, onRemoved = { onRemoved?.invoke(t) }) {
            TrackRow(t, Modifier.padding(vertical = 6.dp), showCover = showCover, number = if (numbered) i + 1 else null)
        }
    }
}

/** Shimmering placeholder block. */
@Composable
fun Skeleton(modifier: Modifier = Modifier, width: Dp? = null, height: Dp = 14.dp, radius: Dp = 6.dp) {
    val alpha by rememberInfiniteTransition(label = "skeleton").animateFloat(
        0.06f, 0.14f, infiniteRepeatable(tween(900), RepeatMode.Reverse), label = "skeleton",
    )
    Box(
        modifier.then(if (width != null) Modifier.width(width) else Modifier.fillMaxWidth())
            .height(height).background(Color.White.copy(alpha = alpha), RoundedCornerShape(radius)),
    )
}

@Composable
fun RowSkeleton() {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
        Skeleton(width = 44.dp, height = 44.dp)
        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Skeleton(width = 180.dp, height = 12.dp)
            Skeleton(width = 110.dp, height = 10.dp)
        }
    }
}

/** Skeleton page shown while an album/playlist/artist loads. */
@Composable
fun DetailSkeleton(round: Boolean = false) {
    Column(
        Modifier.fillMaxSize().padding(horizontal = 16.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Skeleton(Modifier.padding(top = 20.dp), width = 250.dp, height = 250.dp, radius = if (round) 125.dp else 14.dp)
        Skeleton(width = 200.dp, height = 22.dp)
        Skeleton(width = 130.dp, height = 14.dp)
        Row(Modifier.padding(vertical = 6.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Skeleton(Modifier.weight(1f), height = 48.dp, radius = 14.dp)
            Skeleton(Modifier.weight(1f), height = 48.dp, radius = 14.dp)
        }
        repeat(8) { RowSkeleton() }
    }
}

/** Loads a value once and shows progress / errors. */
@Composable
fun <T : Any> Loader(load: suspend () -> T, content: @Composable (T) -> Unit) {
    var value by remember { mutableStateOf<T?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var attempt by remember { mutableIntStateOf(0) }
    LaunchedEffect(attempt) {
        if (value != null) return@LaunchedEffect
        try {
            value = load()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = friendlyMessage(e)
        }
    }
    val v = value
    val err = error
    when {
        v != null -> content(v)
        err != null -> Column(
            Modifier.fillMaxSize().padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            SdatIcon("wifi.exclamationmark", null, size = 40.dp, tint = GlasmioTheme.colors.secondaryContent)
            Text("Couldn't load", style = IosTypography.title2, fontWeight = FontWeight.Bold)
            Text(err, style = IosTypography.body, color = GlasmioTheme.colors.secondaryContent)
            Chip("Retry") { error = null; attempt++ }
        }
        else -> DetailSkeleton()
    }
}

@Composable
fun MediaRow(title: String, subtitle: String, cover: String?, modifier: Modifier = Modifier, round: Boolean = false) {
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
        Cover(cover, 52.dp, radius = 6.dp, circle = round)
        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(title, style = IosTypography.body, maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (subtitle.isNotEmpty()) {
                Text(subtitle, style = IosTypography.subheadline, color = GlasmioTheme.colors.secondaryContent, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}

@Composable
fun ErrorBanner(text: String, modifier: Modifier = Modifier) {
    val orange = Color(0xFFFF9500)
    Row(modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
        SdatIcon("exclamationmark.triangle.fill", null, size = 14.dp, tint = orange)
        Text(text, style = IosTypography.footnote, color = orange)
    }
}

/** Liquid Glass chip, tinted with the accent when selected. */
@Composable
fun Chip(title: String, selected: Boolean = false, onClick: () -> Unit) {
    GlassSurface(shape = CircleShape) {
        Box(
            Modifier
                .then(if (selected) Modifier.background(GlasmioTheme.colors.accent.copy(alpha = 0.5f)) else Modifier)
                .clickable(role = Role.Button, onClick = onClick)
                .padding(horizontal = 16.dp, vertical = 8.dp),
        ) {
            Text(title, style = IosTypography.subheadline, fontWeight = FontWeight.Medium, color = Color.White)
        }
    }
}
