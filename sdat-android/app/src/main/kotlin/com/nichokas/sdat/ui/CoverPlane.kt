package com.nichokas.sdat.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.exponentialDecay
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import kotlin.math.roundToInt
import kotlinx.coroutines.Job
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
            val velocity = VelocityTracker()
            detectDragGestures(
                onDragStart = {
                    velocity.resetTracking()
                    fling?.cancel()
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
        // The pan changes every frame of a drag or fling. Composition sees only the set of
        // visible cells, which changes when a tile enters or leaves; the tiles follow the pan
        // in the layout phase.
        val cells by remember(plane, px, w, h) {
            derivedStateOf {
                val view = pan / px
                plane.cells(view.x - margin, view.y - margin, view.x + w + margin, view.y + h + margin)
            }
        }
        for (cell in cells) {
            val frame = plane.frame(cell)
            val track = tracks[plane.index(cell)]
            key(cell) {
                AsyncImage(
                    model = track.album?.coverUrl?.let { coverUrl(it, frame.size * px) },
                    contentDescription = track.title,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier
                        .offset { IntOffset((frame.x * px - pan.x).roundToInt(), (frame.y * px - pan.y).roundToInt()) }
                        .size(frame.size.dp)
                        .background(placeholder)
                        .clickable(enabled = track.streamable) { onTap(track) },
                )
            }
        }
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
