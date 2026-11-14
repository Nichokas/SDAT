package com.nichokas.sdat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nichokas.sdat.core.Session
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassSurface
import kotlin.coroutines.cancellation.CancellationException
import uniffi.tonearm_ffi.Mix
import uniffi.tonearm_ffi.Shuffle
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.homeMixes

private const val POOL_TARGET = 320

/** The iOS Discover: the home mixes' covers on an endless plane, filled batch by batch. */
@Composable
fun DiscoverView(session: Session, onPlay: (Track, List<Track>, List<Mix>) -> Unit, onMixes: (List<Mix>) -> Unit) {
    var tracks by remember { mutableStateOf(emptyList<Track>()) }
    var mixes by remember { mutableStateOf(emptyList<Mix>()) }
    var error by remember { mutableStateOf<String?>(null) }
    var attempt by remember { mutableIntStateOf(0) }
    LaunchedEffect(attempt) {
        error = null
        val pool = CoverPool()
        try {
            mixes = homeMixes()
            onMixes(mixes)
            Shuffle(mixes).use { source ->
                repeat(10) {
                    pool.add(source.nextBatch(if (pool.tracks.isEmpty()) 30u else 80u))
                    // The plane's layout depends on the pool size, so only the first covers and the
                    // final pool are published: tiles on screen change at most once, as on iOS.
                    if (tracks.isEmpty()) tracks = pool.tracks.toList()
                    if (pool.tracks.size >= POOL_TARGET) return@use
                }
            }
            if (pool.tracks.isEmpty()) error = "Your home mixes have no covers yet."
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (pool.tracks.isEmpty()) error = e.message ?: e.javaClass.simpleName
        }
        tracks = pool.tracks.toList()
    }
    Box(Modifier.fillMaxSize().background(Color.Black)) {
        CoverPlane(tracks, onTap = { onPlay(it, tracks, mixes) })
        // Scrims keep the title and the bottom bar legible over bright covers, as on iOS.
        Box(
            Modifier.fillMaxWidth().background(
                Brush.verticalGradient(0f to Color.Black, 0.35f to Color.Black.copy(alpha = 0.92f), 0.7f to Color.Black.copy(alpha = 0.55f), 1f to Color.Transparent),
            ).statusBarsPadding().height(170.dp),
        )
        Box(
            Modifier.align(Alignment.BottomCenter).fillMaxWidth().background(
                Brush.verticalGradient(0f to Color.Transparent, 0.5f to Color.Black.copy(alpha = 0.5f), 1f to Color.Black.copy(alpha = 0.85f)),
            ).navigationBarsPadding().height(150.dp),
        )
        Text(
            "Discover",
            color = Color.White,
            fontSize = 34.sp,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.statusBarsPadding().padding(start = 20.dp, top = 4.dp).height(44.dp).wrapContentHeight(),
        )
        if (tracks.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                val message = error
                if (message == null) {
                    CircularProgressIndicator(color = Color.White)
                } else {
                    Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Text("Couldn't load Discover", style = IosTypography.headline, color = Color.White)
                        Text(message, style = IosTypography.footnote, color = Color.White.copy(alpha = 0.6f))
                        // Plain buttons: this is the backdrop's sampled layer, where glass would sample itself.
                        TextButton(onClick = { attempt++ }) { Text("Retry", color = Color.White) }
                        TextButton(onClick = session::signOut) { Text("Sign out", color = Color.White) }
                    }
                }
            }
        }
    }
}

/** iOS Discover header's shuffle: the endless Discover queue, once the home mixes are in. */
@Composable
fun ShuffleButton(enabled: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    GlassSurface(modifier.size(44.dp), shape = CircleShape) {
        Box(
            Modifier.fillMaxSize().clickable(enabled = enabled, role = Role.Button, onClickLabel = "Shuffle Discover", onClick = onClick),
            contentAlignment = Alignment.Center,
        ) {
            SdatIcon("shuffle", "Shuffle", size = 20.dp, tint = Color.White.copy(alpha = if (enabled) 1f else 0.4f))
        }
    }
}
