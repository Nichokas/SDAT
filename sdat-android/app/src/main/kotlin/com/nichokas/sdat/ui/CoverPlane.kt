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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
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
import kotlin.math.floor
import kotlin.math.roundToInt
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Track

/** iOS Discover's endless 2D plane: staggered rows of one size each, panned freely with inertia. */
@Composable
fun CoverPlane(tracks: List<Track>, onTap: (Track) -> Unit, modifier: Modifier = Modifier) {
    var pan by remember { mutableStateOf(Offset.Zero) }
    var fling by remember { mutableStateOf<Job?>(null) }
    val scope = rememberCoroutineScope()
    val density = LocalDensity.current
    BoxWithConstraints(
        modifier.fillMaxSize().background(Color.Black).pointerInput(Unit) {
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
        val view = pan / px
        val margin = 75f
        for (cell in plane.cells(view.x - margin, view.y - margin, view.x + maxWidth.value + margin, view.y + maxHeight.value + margin)) {
            val frame = plane.frame(cell)
            val track = tracks[plane.index(cell)]
            key(cell) {
                AsyncImage(
                    model = track.album?.coverUrl?.let { coverUrl(it, frame.size * px) },
                    contentDescription = track.title,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier
                        .offset { IntOffset(((frame.x - view.x) * px).roundToInt(), ((frame.y - view.y) * px).roundToInt()) }
                        .size(frame.size.dp)
                        .background(Color.White.copy(alpha = 0.07f))
                        .clickable(enabled = track.streamable) { onTap(track) },
                )
            }
        }
    }
}

data class Cell(val c: Int, val r: Int)

data class Frame(val x: Float, val y: Float, val size: Float)

/** Plane math ported from iOS PlaneScrollView, in dp. */
class PlaneGeometry(private val count: Int) {
    private val sizes = floatArrayOf(96f, 140f, 118f, 100f, 132f, 150f, 108f)
    private val gap = 12f
    private val period = sizes.sum() + gap * sizes.size
    private val rows = maxOf(12, kotlin.math.sqrt(count / 1.4).toInt())
    private val cols = maxOf(1, (count + rows - 1) / rows)

    fun rowSize(r: Int) = sizes[r.mod(sizes.size)]

    fun rowY(r: Int): Float {
        val k = floor(r.toDouble() / sizes.size).toInt()
        var y = k * period
        for (i in 0 until r - k * sizes.size) y += sizes[i] + gap
        return y
    }

    /** Same hash as iOS (`r &* 2_654_435_761` on a 64-bit Int), so rows stagger alike. */
    fun rowOffset(r: Int): Float {
        val h = ((r.toLong() * 2_654_435_761L).toULong() % 1000uL).toFloat() / 1000f
        return h * (rowSize(r) + gap)
    }

    fun frame(cell: Cell): Frame {
        val size = rowSize(cell.r)
        return Frame(cell.c * (size + gap) - rowOffset(cell.r), rowY(cell.r), size)
    }

    fun cells(minX: Float, minY: Float, maxX: Float, maxY: Float): List<Cell> {
        val out = mutableListOf<Cell>()
        var r = floor(minY / period).toInt() * sizes.size - 1
        while (rowY(r) <= maxY) {
            val size = rowSize(r)
            val step = size + gap
            val off = rowOffset(r)
            if (rowY(r) + size >= minY) {
                val c0 = floor((minX + off - size) / step).toInt()
                val c1 = floor((maxX + off) / step).toInt()
                for (c in c0..maxOf(c0, c1)) out += Cell(c, r)
            }
            r++
        }
        return out
    }

    /** The pool tiles the plane as one cols×rows block, so a repeat is a block away. */
    // Rounding cols up and wrapping keeps every pooled track on the plane.
    fun index(cell: Cell) = (cell.r.mod(rows) * cols + cell.c.mod(cols)) % count
}

private val tidalSize = Regex("""/\d+x\d+\.jpg$""")
private val albumSizes = intArrayOf(80, 160, 320, 640, 1280)

/** Smallest TIDAL album rendition that covers [px] pixels, like iOS CoverURL.sized. */
fun coverUrl(url: String, px: Float): String {
    if (!url.contains("resources.tidal.com/images/") || !tidalSize.containsMatchIn(url)) return url
    val size = albumSizes.firstOrNull { it >= px } ?: albumSizes.last()
    return url.replace(tidalSize, "/${size}x$size.jpg")
}
