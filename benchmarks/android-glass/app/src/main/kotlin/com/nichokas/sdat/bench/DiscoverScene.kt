package com.nichokas.sdat.bench

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import io.github.neoruaa.glasmio.core.GlasmioTheme
import java.io.File
import kotlin.math.floor
import kotlin.math.roundToInt

// Discover: SDAT's CoverPlane and PlaneGeometry, with the covers from disk.

@Composable
internal fun Discover(glass: Glass, covers: List<File>, images: ImageCounter, seconds: State<Float>, speedDp: Float, cold: Boolean) {
    val noCompact = remember { mutableFloatStateOf(0f) }
    Host(
        glass,
        page = {
            val background = GlasmioTheme.colors.groupedBackground
            Box(Modifier.fillMaxSize().background(background)) {
                Plane(covers, images, seconds, speedDp, cold)
                Box(
                    Modifier.fillMaxWidth().background(
                        Brush.verticalGradient(0f to background, 0.35f to background.copy(alpha = 0.92f), 0.7f to background.copy(alpha = 0.55f), 1f to background.copy(alpha = 0f)),
                    ).statusBarsPadding().height(170.dp),
                )
                Box(
                    Modifier.align(Alignment.BottomCenter).fillMaxWidth().background(
                        Brush.verticalGradient(0f to background.copy(alpha = 0f), 0.5f to background.copy(alpha = 0.5f), 1f to background.copy(alpha = 0.85f)),
                    ).navigationBarsPadding().height(150.dp),
                )
                Text("Discover", color = GlasmioTheme.colors.content, fontSize = 34.sp, fontWeight = FontWeight.Bold, modifier = Modifier.statusBarsPadding().padding(start = 20.dp, top = 4.dp))
            }
        },
        chrome = {
            HeaderButtons()
            BottomChrome(Tab.Discover, noCompact, seconds)
        },
    )
}

@Composable
private fun Plane(covers: List<File>, images: ImageCounter, seconds: State<Float>, speedDp: Float, cold: Boolean) {
    val density = LocalDensity.current.density
    val placeholder = GlasmioTheme.colors.content.copy(alpha = 0.07f)
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val plane = remember(covers.size) { PlaneGeometry(covers.size) }
        val margin = 75f
        val (w, h) = maxWidth.value to maxHeight.value
        // Diagonal, so new covers enter from two edges at once.
        val pan by remember(speedDp) { derivedStateOf { Offset(speedDp * seconds.value, 0.45f * speedDp * seconds.value) } }
        val cells by remember(plane, w, h) {
            derivedStateOf { plane.cells(pan.x - margin, pan.y - margin, pan.x + w + margin, pan.y + h + margin) }
        }
        for (cell in cells) {
            val frame = plane.frame(cell)
            key(cell) {
                AsyncImage(
                    model = cover(covers[plane.index(cell)], (frame.size * density).roundToInt(), cold, images),
                    contentDescription = null,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier
                        .offset { IntOffset(((frame.x - pan.x) * density).roundToInt(), ((frame.y - pan.y) * density).roundToInt()) }
                        .size(frame.size.dp)
                        .background(placeholder),
                )
            }
        }
    }
}

data class Cell(val c: Int, val r: Int)

data class Tile(val x: Float, val y: Float, val size: Float)

/** Copied from SDAT's ui/CoverPlane.kt. */
class PlaneGeometry(private val count: Int) {
    private val sizes = floatArrayOf(96f, 140f, 118f, 100f, 132f, 150f, 108f)
    private val gap = 12f
    private val period = sizes.sum() + gap * sizes.size
    private val rows = maxOf(12, kotlin.math.sqrt(count / 1.4).toInt())
    private val stride = maxOf(1, count / rows)

    fun rowSize(r: Int) = sizes[r.mod(sizes.size)]

    fun rowY(r: Int): Float {
        val k = floor(r.toDouble() / sizes.size).toInt()
        var y = k * period
        for (i in 0 until r - k * sizes.size) y += sizes[i] + gap
        return y
    }

    fun rowOffset(r: Int): Float {
        val h = ((r.toLong() * 2_654_435_761L).toULong() % 1000uL).toFloat() / 1000f
        return h * (rowSize(r) + gap)
    }

    fun frame(cell: Cell): Tile {
        val size = rowSize(cell.r)
        return Tile(cell.c * (size + gap) - rowOffset(cell.r), rowY(cell.r), size)
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

    fun index(cell: Cell) = (cell.c + cell.r.toLong() * stride).mod(count.toLong()).toInt()
}
