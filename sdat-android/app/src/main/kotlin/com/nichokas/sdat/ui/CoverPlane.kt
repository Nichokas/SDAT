package com.nichokas.sdat.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.exponentialDecay
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.listSaver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import coil3.compose.AsyncImage
import coil3.SingletonImageLoader
import coil3.request.ImageRequest
import coil3.request.crossfade
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.displayTitle
import com.nichokas.sdat.ui.kit.PopupMenu
import com.nichokas.sdat.ui.kit.menuAnchor
import kotlin.math.roundToInt
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Track

private val offsetSaver = listSaver<Offset, Float>({ listOf(it.x, it.y) }, { Offset(it[0], it[1]) })

/** iOS Discover's endless 2D plane: staggered rows of one size each, panned freely with inertia. */
@Composable
fun CoverPlane(tracks: List<Track>, background: Color, placeholder: Color, onTap: (Track) -> Unit, modifier: Modifier = Modifier) {
    // Saved so another tab and back returns to the same spot on the plane.
    var pan by rememberSaveable(stateSaver = offsetSaver) { mutableStateOf(Offset.Zero) }
    var fling by remember { mutableStateOf<Job?>(null) }
    val scope = rememberCoroutineScope()
    val density = LocalDensity.current
    BoxWithConstraints(
        modifier.fillMaxSize().background(background).pointerInput(Unit) {
            awaitEachGesture {
                awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
                fling?.cancel()
            }
        }.pointerInput(Unit) {
            val velocity = VelocityTracker()
            detectDragGestures(
                onDragStart = {
                    velocity.resetTracking()
                },
                onDragEnd = {
                    val v = velocity.calculateVelocity()
                    fling = scope.launch {
                        Animatable(pan, Offset.VectorConverter).animateDecay(Offset(-v.x, -v.y), exponentialDecay()) { pan = value }
                    }
                },
            ) { change, drag ->
                velocity.addPosition(change.uptimeMillis, change.position)
                pan -= drag
            }
        },
    ) {
        if (tracks.isEmpty()) return@BoxWithConstraints
        val plane = remember(tracks.size) { PlaneGeometry(tracks.size) }
        val px = density.density
        val margin = 75f
        val (w, h) = maxWidth.value to maxHeight.value
        val context = LocalContext.current
        // The pan changes every frame of a drag or fling. Composition sees only the set of
        // visible cells, which changes when a tile enters or leaves; the tiles follow the pan
        // in the layout phase.
        val cells by remember(plane, px, w, h) {
            derivedStateOf {
                val view = pan / px
                plane.cells(view.x - margin, view.y - margin, view.x + w + margin, view.y + h + margin)
            }
        }
        LaunchedEffect(plane, tracks, px, w, h) {
            val loader = SingletonImageLoader.get(context)
            var previous: Offset? = null
            snapshotFlow {
                val view = pan / px
                Offset((view.x / margin).roundToInt() * margin, (view.y / margin).roundToInt() * margin)
            }.collectLatest { view ->
                val last = previous
                previous = view
                if (last == null) return@collectLatest
                val delta = view - last
                val ahead = view + delta * (2 * margin / delta.getDistance())
                val visible = cells.toSet()
                val requests = plane.cells(ahead.x - margin, ahead.y - margin, ahead.x + w + margin, ahead.y + h + margin)
                    .asSequence().filterNot(visible::contains).mapNotNull { cell ->
                        val size = plane.frame(cell).size * px
                        tracks[plane.index(cell)].album?.coverUrl?.let { coverUrl(it, size) to size.roundToInt().coerceAtLeast(1) }
                    }.distinct().take(6)
                for ((url, size) in requests) {
                    loader.execute(ImageRequest.Builder(context).data(url).size(size).build())
                }
            }
        }
        for (cell in cells) {
            val frame = plane.frame(cell)
            val track = tracks[plane.index(cell)]
            key(cell, track.id) {
                DiscoverTile(
                    track = track,
                    url = track.album?.coverUrl?.let { coverUrl(it, frame.size * px) },
                    placeholder = placeholder,
                    modifier = Modifier
                        .offset { IntOffset((frame.x * px - pan.x).roundToInt(), (frame.y * px - pan.y).roundToInt()) }
                        .size(frame.size.dp),
                    onTap = { onTap(track) },
                )
            }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun DiscoverTile(track: Track, url: String?, placeholder: Color, modifier: Modifier, onTap: () -> Unit) {
    var bounds by remember { mutableStateOf<Rect?>(null) }
    var open by remember { mutableStateOf(false) }
    val scale by animateFloatAsState(if (open) 1.08f else 1f, label = "cover preview")
    val context = LocalContext.current
    val request = remember(context, url) { ImageRequest.Builder(context).data(url).crossfade(150).build() }
    AsyncImage(
        model = request,
        contentDescription = track.displayTitle,
        contentScale = ContentScale.Crop,
        modifier = modifier.zIndex(if (open) 1f else 0f)
            .onGloballyPositioned { bounds = it.boundsInRoot() }
            .graphicsLayer { scaleX = scale; scaleY = scale }
            .clip(RoundedCornerShape(if (open) 10.dp else 0.dp))
            .background(placeholder)
            .menuAnchor(open)
            .combinedClickable(
                role = Role.Button,
                onClickLabel = if (track.streamable) "Play" else "Track options",
                onLongClickLabel = "Track options",
                onLongClick = { open = true },
                onClick = { if (track.streamable) onTap() else open = true },
            ),
    )
    if (open) {
        LaunchedEffect(track.id) { Favs.check(track.id) }
        PopupMenu(bounds, trackMenuItems(track), title = track.displayTitle, onDismiss = { open = false })
    }
}

private val tidalSize = Regex("""/\d+x\d+\.jpg$""")
private val albumSizes = intArrayOf(80, 160, 320, 640, 1280)

/** Smallest TIDAL album rendition that covers [px] pixels, like iOS CoverURL.sized. */
fun coverUrl(url: String, px: Float): String {
    if (!url.contains("resources.tidal.com/images/") || !tidalSize.containsMatchIn(url)) return url
    val size = albumSizes.firstOrNull { it >= px } ?: albumSizes.last()
    return url.replace(tidalSize, "/${size}x$size.jpg")
}
