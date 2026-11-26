package com.nichokas.sdat.ui

import android.content.Context
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
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
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
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
    // Tiles derive their menu anchor from this instead of each tracking its own position, which would run a
    // callback per tile on every frame of a pan.
    var origin by remember { mutableStateOf(Offset.Zero) }
    BoxWithConstraints(
        modifier.fillMaxSize().background(background).onGloballyPositioned { origin = it.positionInRoot() }.pointerInput(Unit) {
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
        // One rendition and decode size for every row, so a cover is fetched, decoded and cached once, and a
        // tile entering the screen finds it in the memory cache on its first frame.
        val coverPx = (plane.largestSize * px).roundToInt()
        // The pan changes every frame of a drag or fling. Composition sees only the set of
        // visible cells, which changes when a tile enters or leaves; the tiles follow the pan
        // in the layout phase.
        val cells by remember(plane, px, w, h) {
            derivedStateOf {
                val view = pan / px
                plane.cells(view.x - margin, view.y - margin, view.x + w + margin, view.y + h + margin)
            }
        }
        // Every cover reaches the disk cache before a fling can, a few at a time so the tiles on screen are
        // not queued behind the whole pool.
        LaunchedEffect(tracks, coverPx) {
            val loader = SingletonImageLoader.get(context)
            val permits = Semaphore(4)
            for (track in tracks) {
                val url = track.album?.coverUrl?.let { CoverUrl.sized(it, coverPx) } ?: continue
                launch { permits.withPermit { loader.execute(coverRequest(context, url, coverPx)) } }
            }
        }
        for (cell in cells) {
            val frame = plane.frame(cell)
            val track = tracks[plane.index(cell)]
            key(cell, track.id) {
                DiscoverTile(
                    track = track,
                    url = track.album?.coverUrl?.let { CoverUrl.sized(it, coverPx) },
                    px = coverPx,
                    placeholder = placeholder,
                    modifier = Modifier
                        .offset { IntOffset((frame.x * px - pan.x).roundToInt(), (frame.y * px - pan.y).roundToInt()) }
                        .size(frame.size.dp),
                    bounds = { Rect(origin + Offset(frame.x * px - pan.x, frame.y * px - pan.y), Size(frame.size * px, frame.size * px)) },
                    onTap = { onTap(track) },
                )
            }
        }
    }
}

private val tileShape = RoundedCornerShape(14.dp)

private fun coverRequest(context: Context, url: String?, px: Int) =
    ImageRequest.Builder(context).data(url).size(px).crossfade(150).build()

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun DiscoverTile(track: Track, url: String?, px: Int, placeholder: Color, modifier: Modifier, bounds: () -> Rect, onTap: () -> Unit) {
    var menu by remember { mutableStateOf<Rect?>(null) }
    val open = menu != null
    val scale by animateFloatAsState(if (open) 1.08f else 1f, label = "cover preview")
    val context = LocalContext.current
    val request = remember(context, url, px) { coverRequest(context, url, px) }
    AsyncImage(
        model = request,
        contentDescription = track.displayTitle,
        contentScale = ContentScale.Crop,
        modifier = modifier.zIndex(if (open) 1f else 0f)
            .graphicsLayer { scaleX = scale; scaleY = scale; shape = tileShape; clip = true }
            .background(placeholder)
            .menuAnchor(open)
            .combinedClickable(
                role = Role.Button,
                onClickLabel = if (track.streamable) "Play" else "Track options",
                onLongClickLabel = "Track options",
                onLongClick = { menu = bounds() },
                onClick = { if (track.streamable) onTap() else menu = bounds() },
            ),
    )
    if (open) {
        LaunchedEffect(track.id) { Favs.check(track.id) }
        PopupMenu(menu, trackMenuItems(track), title = track.displayTitle, onDismiss = { menu = null })
    }
}
