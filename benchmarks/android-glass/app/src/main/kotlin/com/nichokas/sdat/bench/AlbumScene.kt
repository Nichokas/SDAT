package com.nichokas.sdat.bench

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ColorMatrix
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassEmphasis
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.glass.GlasmioBackdropHost
import io.github.neoruaa.glasmio.glass.LocalGlassBackdrop
import io.github.neoruaa.glasmio.glass.glassBackdropSource
import io.github.neoruaa.glasmio.glass.render.backdrops.rememberLayerBackdrop
import io.github.neoruaa.glasmio.glass.trackBackdropPosition
import io.github.neoruaa.glasmio.scaffold.GlasmioPinnedPage
import io.github.neoruaa.glasmio.scaffold.rememberGlasmioHeaderScrollEdge
import io.github.neoruaa.glasmio.ui.GlasmioButton
import io.github.neoruaa.glasmio.ui.GlasmioButtonSize
import java.io.File
import kotlin.math.roundToInt

// Album: SDAT's DetailPage with its blurred cover backdrop, glass Play and Shuffle in the scrolled header, and the wide mini player.

@Composable
internal fun Album(covers: List<File>, images: ImageCounter) {
    val list = rememberLazyListState()
    AutoScroll(list, speedDp = 1600f)
    val pastHeader by remember { derivedStateOf { list.firstVisibleItemIndex > 0 } }
    val backdrop = rememberLayerBackdrop()
    val art = covers.first()
    GlasmioBackdropHost(
        Modifier.fillMaxSize(),
        sampledContent = {
            GlasmioPinnedPage(
                title = "Album",
                collapseProgress = if (pastHeader) 1f else 0f,
                topBarBlurProgress = rememberGlasmioHeaderScrollEdge(list),
                backgroundColor = Color.Black,
                background = { CoverBackdrop(art, images, Modifier.matchParentSize().glassBackdropSource(backdrop).trackBackdropPosition(backdrop)) },
            ) { padding ->
                CompositionLocalProvider(LocalGlassBackdrop provides backdrop) {
                    LazyColumn(state = list, modifier = Modifier.fillMaxSize(), contentPadding = PaddingValues(top = padding.calculateTopPadding(), bottom = 120.dp)) {
                        item {
                            Column(Modifier.fillMaxWidth().padding(horizontal = 24.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                                val px = (260 * LocalDensity.current.density).roundToInt()
                                AsyncImage(cover(art, px, false, images), null, Modifier.size(260.dp).shadow(20.dp, RoundedCornerShape(12.dp)).clip(RoundedCornerShape(12.dp)), contentScale = ContentScale.Crop)
                                Text("Album title", style = IosTypography.title2, color = Color.White, modifier = Modifier.padding(top = 16.dp))
                                Row(Modifier.fillMaxWidth().padding(top = 16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                                    GlasmioButton({}, Modifier.weight(1f), emphasis = GlassEmphasis.Prominent, size = GlasmioButtonSize.Large) { Text("Play", color = Color.White) }
                                    GlasmioButton({}, Modifier.weight(1f), size = GlasmioButtonSize.Large) { Text("Shuffle", color = Color.White) }
                                }
                            }
                        }
                        items(60) { i -> CoverRow(covers[(i + 1) % covers.size], "${i + 1}. Track", "Artist", images) }
                    }
                }
            }
        },
        overlayContent = {
            GlassSurface(Modifier.align(Alignment.BottomCenter).fillMaxWidth().navigationBarsPadding().padding(start = 24.dp, end = 24.dp, bottom = 12.dp).height(62.dp), shape = RoundedCornerShape(percent = 50)) {
                WideMini(art, images)
            }
        },
    )
}

/** SDAT's wide mini player content: cover, title and artist, the play ring, and next. */
@Composable
private fun WideMini(art: File, images: ImageCounter) {
    Row(Modifier.fillMaxSize().padding(start = 9.dp, end = 10.dp), horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
        val px = (38 * LocalDensity.current.density).roundToInt()
        AsyncImage(cover(art, px, false, images), null, Modifier.size(38.dp).clip(RoundedCornerShape(6.dp)), contentScale = ContentScale.Crop)
        Column(Modifier.weight(1f)) {
            Text("Track title", style = IosTypography.subheadline, fontWeight = FontWeight.SemiBold, color = Color.White, maxLines = 1)
            Text("Artist", style = IosTypography.caption1, color = Color.White.copy(alpha = 0.6f), maxLines = 1)
        }
        Box(Modifier.size(40.dp)) { ProgressRing(null) }
        Canvas(Modifier.size(36.dp, 40.dp).padding(horizontal = 9.dp, vertical = 13.dp)) {
            val half = size.width / 2
            for (x in listOf(0f, half)) {
                drawPath(Path().apply { moveTo(x, 0f); lineTo(x + half, size.height / 2); lineTo(x, size.height); close() }, Color.White)
            }
        }
    }
}

/** SDAT's DetailPage CoverBackdrop: the cover zoomed and blurred into colour fields. */
@Composable
private fun CoverBackdrop(file: File, images: ImageCounter, modifier: Modifier) {
    Box(modifier.background(Color.Black).clipToBounds()) {
        AsyncImage(
            cover(file, 160, false, images),
            null,
            contentScale = ContentScale.Crop,
            colorFilter = ColorFilter.colorMatrix(ColorMatrix().apply { setToSaturation(1.8f) }),
            modifier = Modifier.fillMaxSize().graphicsLayer { scaleX = 2.2f; scaleY = 2.2f }.blur(50.dp),
        )
        Box(Modifier.fillMaxSize().background(Color.Black.copy(alpha = 0.32f)))
        Box(Modifier.fillMaxSize().background(Brush.verticalGradient(0.5f to Color.Transparent, 1f to Color.Black.copy(alpha = 0.55f))))
    }
}
