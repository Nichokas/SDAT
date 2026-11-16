package com.nichokas.sdat.bench

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.glass.GlasmioBackdropHost
import io.github.neoruaa.glasmio.ui.GlasmioSlider
import java.io.File
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlinx.coroutines.delay

// Player: SDAT's PlayerSheet content (the sheet's own window frame is left out, so this window's metrics see it),
// swiping between Now Playing and the queue while the slider moves.

@Composable
internal fun Player(covers: List<File>, images: ImageCounter, seconds: State<Float>) {
    val pager = rememberPagerState { 2 }
    LaunchedEffect(pager) {
        while (true) {
            delay(900)
            pager.animateScrollToPage(1 - pager.currentPage)
        }
    }
    val art = covers[1]
    GlasmioBackdropHost(
        Modifier.fillMaxSize(),
        sampledContent = {
            Box(Modifier.fillMaxSize().background(Color.Black)) {
                AsyncImage(cover(art, 320, false, images), null, contentScale = ContentScale.Crop, modifier = Modifier.fillMaxSize().blur(80.dp).graphicsLayer { alpha = 0.6f })
            }
        },
        overlayContent = {
            HorizontalPager(pager, Modifier.fillMaxSize()) { page ->
                if (page == 0) NowPlaying(art, images, seconds) else Queue(covers, images)
            }
        },
    )
}

@Composable
private fun NowPlaying(art: File, images: ImageCounter, seconds: State<Float>) {
    Column(Modifier.fillMaxSize().statusBarsPadding().padding(24.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        val px = (320 * LocalDensity.current.density).roundToInt()
        AsyncImage(cover(art, px, false, images), null, Modifier.fillMaxWidth().aspectRatio(1f).shadow(28.dp, RoundedCornerShape(16.dp)).clip(RoundedCornerShape(16.dp)), contentScale = ContentScale.Crop)
        Text("Track title", style = IosTypography.title3, color = Color.White, modifier = Modifier.padding(top = 24.dp))
        Text("Artist", style = IosTypography.body, color = Color.White.copy(alpha = 0.6f))
        Spacer(Modifier.height(14.dp))
        val position by remember { derivedStateOf { (seconds.value * 4f) % 240f } }
        GlasmioSlider(value = position, onValueChange = {}, valueRange = 0f..240f)
        Row(Modifier.padding(top = 20.dp), horizontalArrangement = Arrangement.spacedBy(46.dp)) {
            repeat(3) { GlassSurface(Modifier.size(56.dp), shape = CircleShape) {} }
        }
        Spacer(Modifier.height(16.dp))
        // A pulse like the cover's play/pause spring, so the page redraws every frame.
        Box(Modifier.size(8.dp).graphicsLayer { val s = 1f + 0.3f * sin(seconds.value * 6f); scaleX = s; scaleY = s }.background(Color.White, CircleShape))
    }
}

@Composable
private fun Queue(covers: List<File>, images: ImageCounter) {
    LazyColumn(Modifier.fillMaxSize().statusBarsPadding(), contentPadding = PaddingValues(bottom = 24.dp)) {
        items(80) { i -> CoverRow(covers[(i + 5) % covers.size], "Up next ${i + 1}", "Artist", images) }
    }
}
