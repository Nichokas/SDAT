package com.nichokas.sdat.ui

import android.animation.ValueAnimator
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
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
import androidx.compose.foundation.selection.selectable
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
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.runtime.withFrameMillis
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil3.SingletonImageLoader
import coil3.compose.AsyncImage
import coil3.memory.MemoryCache
import coil3.request.ImageRequest
import coil3.request.crossfade
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Downloads
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.Fmt
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.artistLine
import com.nichokas.sdat.core.displayTitle
import com.nichokas.sdat.core.friendlyMessage
import com.nichokas.sdat.ui.kit.MenuAction
import com.nichokas.sdat.ui.kit.PopupMenu
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import com.nichokas.sdat.ui.kit.capsule
import com.nichokas.sdat.ui.kit.menuAnchor
import kotlin.coroutines.cancellation.CancellationException
import kotlin.math.abs
import kotlin.math.sin
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

    /** Play buttons: the list from its first track, with shuffle off. */
    fun playInOrder(tracks: List<Track>, from: String?)

    /** Shuffle buttons: turns shuffle on and starts at a random track. */
    fun playShuffled(tracks: List<Track>, from: String?)

    /** Bulk actions take the whole list, so the queue is saved once, not once per track. */
    fun playNext(tracks: List<Track>)
    fun addToQueue(tracks: List<Track>)
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

    fun smallerRenditions(url: String, kind: Kind): List<String> {
        if ("resources.tidal.com/images/" !in url || !url.endsWith(".jpg")) return emptyList()
        val match = sizeSuffix.find(url) ?: return emptyList()
        val size = match.value.substring(1).substringBefore('x').toIntOrNull() ?: return emptyList()
        return kind.sizes.asReversed().filter { it < size }
            .map { url.replaceRange(match.range, "/${it}x$it.jpg") }
    }
}

/** Square cover sized for its frame. */
@Composable
fun Cover(url: String?, size: Dp, modifier: Modifier = Modifier, radius: Dp = 8.dp, circle: Boolean = false) {
    val px = with(LocalDensity.current) { size.roundToPx() }
    val kind = if (circle) CoverUrl.Kind.Artist else CoverUrl.Kind.Album
    Box(
        modifier.size(size).clip(if (circle) CircleShape else RoundedCornerShape(radius)).background(SdatTheme.colors.fill),
        contentAlignment = Alignment.Center,
    ) {
        if (url == null) {
            SdatIcon("music.note", null, size = size / 3, tint = SdatTheme.colors.secondaryContent)
        } else {
            val context = LocalContext.current
            val loader = SingletonImageLoader.get(context)
            val localFile = Downloads.coverFile(url)
            val remote = CoverUrl.sized(url, px, kind)
            val animate = ValueAnimator.areAnimatorsEnabled()
            val request = remember(context, loader, localFile, remote, kind, animate) {
                ImageRequest.Builder(context).data(localFile ?: remote).apply {
                    if (localFile == null) {
                        val cache = loader.memoryCache
                        val key = (sequenceOf(remote) + CoverUrl.smallerRenditions(remote, kind))
                            .map { MemoryCache.Key(it) }
                            .firstOrNull { cache?.get(it) != null }
                        placeholderMemoryCacheKey(key)
                        if (key != null) error { cache?.get(key)?.image }
                    }
                    crossfade(if (animate) 150 else 0)
                }.build()
            }
            AsyncImage(
                model = request,
                imageLoader = loader,
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
    val color = SdatTheme.colors.accent
    // Drawn, not laid out: the bars change every frame, and reading the clock in the draw
    // phase skips recomposition and layout.
    Canvas(modifier.size(15.dp, 14.dp)) {
        val bar = 3.dp.toPx()
        repeat(3) { i ->
            val h = size.height * (if (playing) 0.35 + 0.65 * abs(sin(t * (5 + i * 1.7) + i)) else 0.35).toFloat()
            drawRoundRect(color, Offset(i * (bar + 2.dp.toPx()), size.height - h), Size(bar, h), CornerRadius(bar / 2))
        }
    }
}

@Composable
fun TrackRow(track: Track, modifier: Modifier = Modifier, showCover: Boolean = true, number: Int? = null, coverSize: Dp = 44.dp) {
    val player = LocalTrackPlayer.current
    val colors = SdatTheme.colors
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
                    Text("$number", style = SdatType.subheadline.copy(fontFeatureSettings = "tnum"), color = colors.secondaryContent)
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
                style = SdatType.body,
                color = if (isCurrent) colors.accent else colors.content,
                fontWeight = if (isCurrent) FontWeight.SemiBold else FontWeight.Normal,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            val secondary = if (isCurrent) colors.accent.copy(alpha = 0.8f) else colors.secondaryContent
            Row(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
                if (track.explicit) SdatIcon("e.square.fill", "Explicit", size = 12.dp, tint = secondary)
                Text(track.artistLine, style = SdatType.subheadline, color = secondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        DownloadBadge(track.id)
        Text(
            Fmt.time(track.durationSecs.toDouble()),
            style = SdatType.subheadline.copy(fontFeatureSettings = "tnum"),
            color = colors.secondaryContent,
        )
    }
}

/** Long-press menu items shared by every track row. */
@Composable
fun trackMenuItems(
    track: Track,
    ownPlaylistId: String? = null,
    onRemoved: ((String) -> Unit)? = null,
    extra: List<MenuAction> = emptyList(),
): List<MenuAction> {
    val player = LocalTrackPlayer.current
    val fav = Favs.known(track.id) ?: false
    return buildList {
        add(MenuAction(if (fav) "Remove from Favourites" else "Add to Favourites", if (fav) "star.slash" else "star", onClick = { Favs.set(track.id, !fav) }))
        add(MenuAction("Play Next", "text.line.first.and.arrowtriangle.forward", separatorBefore = true, onClick = {
            if (!Live.userPlayNext(listOf(track))) { player.playNext(listOf(track)); Chrome.flash("Playing next") }
        }))
        add(MenuAction("Add to Queue", "text.line.last.and.arrowtriangle.forward", onClick = {
            if (!Live.userEnqueue(listOf(track))) { player.addToQueue(listOf(track)); Chrome.flash("Added to queue") }
        }))
        add(MenuAction("Add to Playlist…", "text.badge.plus", onClick = { Chrome.playlistPickerTrack = track }))
        addAll(downloadMenuItems(listOf(track)))
        addAll(extra)
        val itemId = track.playlistItemId
        if (ownPlaylistId != null && itemId != null) {
            add(MenuAction("Remove from Playlist", "minus.circle", destructive = true, onClick = {
                Chrome.actionScope.launch {
                    try {
                        removeFromPlaylist(ownPlaylistId, track.id, itemId)
                        onRemoved?.invoke(itemId)
                        Chrome.flash("Removed from playlist")
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Exception) {
                        Chrome.flash(friendlyMessage(e))
                    }
                }
            }))
        }
        add(MenuAction("Start Radio", "dot.radiowaves.left.and.right", separatorBefore = true, onClick = {
            Chrome.actionScope.launch {
                try {
                    val list = trackRadio(track.id)?.use { it.nextPage() }
                    if (list.isNullOrEmpty()) Chrome.flash("No radio for this track")
                    else player.play(listOf(track) + list, from = "${track.title} Radio")
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Chrome.flash(friendlyMessage(e))
                }
            }
        }))
        track.album?.let { a ->
            add(MenuAction("Go to Album", "square.stack", onClick = { Chrome.showPlayer = false; Chrome.push(Route.Album(a.id)) }))
        }
        track.artists.firstOrNull()?.let { ar ->
            add(MenuAction("Go to Artist", "music.microphone", onClick = { Chrome.showPlayer = false; Chrome.push(Route.Artist(ar.id)) }))
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
    onRemoved: ((String) -> Unit)? = null,
    extraItems: List<MenuAction> = emptyList(),
    content: @Composable () -> Unit,
) {
    var bounds by remember { mutableStateOf<Rect?>(null) }
    var open by remember { mutableStateOf(false) }
    LaunchedEffect(track.id) {
        delay(600)
        Favs.check(track.id)
    }
    Box(
        modifier
            .onGloballyPositioned { bounds = it.boundsInRoot() }
            .menuAnchor(open)
            .combinedClickable(onLongClick = { open = true }, onClick = onClick),
    ) { content() }
    if (open) {
        PopupMenu(
            anchorBounds = bounds,
            actions = trackMenuItems(track, ownPlaylistId, onRemoved, extraItems),
            title = track.displayTitle,
            onDismiss = { open = false },
        )
    }
}

/** Shimmering placeholder block. */
@Composable
fun Skeleton(modifier: Modifier = Modifier, width: Dp? = null, height: Dp = 14.dp, radius: Dp = 6.dp) {
    val alpha = rememberInfiniteTransition(label = "skeleton").animateFloat(
        0.06f, 0.14f, infiniteRepeatable(tween(900), RepeatMode.Reverse), label = "skeleton",
    )
    // The shimmer is read in the draw phase: a loading page holds dozens of these, and
    // recomposing each one every frame is what a skeleton is there to avoid.
    val tone = SdatTheme.colors.content
    Box(
        modifier.then(if (width != null) Modifier.width(width) else Modifier.fillMaxWidth())
            .height(height)
            .drawBehind { drawRoundRect(tone.copy(alpha = alpha.value), cornerRadius = CornerRadius(radius.toPx())) },
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
            SdatIcon("wifi.exclamationmark", null, size = 40.dp, tint = SdatTheme.colors.secondaryContent)
            Text("Couldn't load", style = SdatType.title2, fontWeight = FontWeight.Bold)
            Text(err, style = SdatType.body, color = SdatTheme.colors.secondaryContent)
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
            Text(title, style = SdatType.body, maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (subtitle.isNotEmpty()) {
                Text(subtitle, style = SdatType.subheadline, color = SdatTheme.colors.secondaryContent, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}

@Composable
fun ErrorBanner(text: String, modifier: Modifier = Modifier) {
    val orange = Color(0xFFFF9500)
    Row(modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
        SdatIcon("exclamationmark.triangle.fill", null, size = 14.dp, tint = orange)
        Text(text, style = SdatType.footnote, color = orange)
    }
}

/** A 36 dp capsule on the fill colour; inverse when selected. */
@Composable
fun Chip(title: String, selected: Boolean = false, onClick: () -> Unit) {
    val colors = SdatTheme.colors
    Box(
        Modifier
            .height(36.dp)
            .clip(capsule)
            .background(if (selected) colors.inverse else colors.fill)
            .selectable(selected, role = Role.Button, onClick = onClick)
            .padding(horizontal = 16.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(title, style = SdatType.subheadline, fontWeight = FontWeight.Medium, color = if (selected) colors.onInverse else colors.content)
    }
}
