package com.nichokas.sdat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import com.nichokas.sdat.core.Session
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.ui.GlasmioButton
import kotlin.coroutines.cancellation.CancellationException
import uniffi.tonearm_ffi.Shuffle
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.homeMixes

private const val POOL_TARGET = 320

/** Covers from the user's home mixes, filled batch by batch like the iOS cover plane. */
@Composable
fun DiscoverView(session: Session, contentPadding: PaddingValues) {
    var tracks by remember { mutableStateOf(emptyList<Track>()) }
    var error by remember { mutableStateOf<String?>(null) }
    var attempt by remember { mutableIntStateOf(0) }
    LaunchedEffect(attempt) {
        error = null
        try {
            val pool = CoverPool()
            Shuffle(homeMixes()).use { source ->
                repeat(10) {
                    pool.add(source.nextBatch(if (pool.tracks.isEmpty()) 30u else 80u))
                    tracks = pool.tracks.toList()
                    if (pool.tracks.size >= POOL_TARGET) return@use
                }
            }
            if (pool.tracks.isEmpty()) error = "Your home mixes have no covers yet."
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (tracks.isEmpty()) error = e.message ?: e.javaClass.simpleName
        }
    }
    val colors = GlasmioTheme.colors
    LazyVerticalGrid(
        columns = GridCells.Adaptive(112.dp),
        modifier = Modifier.fillMaxSize().background(colors.groupedBackground),
        contentPadding = contentPadding,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        item(span = { GridItemSpan(maxLineSpan) }) {
            Text("Discover", style = IosTypography.largeTitle, modifier = Modifier.padding(bottom = 8.dp))
        }
        items(tracks, key = { it.id }) { t ->
            AsyncImage(
                model = t.album?.coverUrl,
                contentDescription = t.album?.title,
                contentScale = ContentScale.Crop,
                modifier = Modifier.aspectRatio(1f).clip(RoundedCornerShape(8.dp)).background(colors.container),
            )
        }
        if (tracks.isEmpty()) {
            item(span = { GridItemSpan(maxLineSpan) }) {
                Box(Modifier.padding(top = 120.dp), contentAlignment = Alignment.Center) {
                    val message = error
                    if (message == null) {
                        CircularProgressIndicator()
                    } else {
                        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                            Text("Couldn't load Discover", style = IosTypography.headline)
                            Text(message, style = IosTypography.footnote, color = colors.secondaryContent)
                            GlasmioButton(onClick = { attempt++ }) { Text("Retry") }
                            GlasmioButton(onClick = session::signOut) { Text("Sign out") }
                        }
                    }
                }
            }
        }
    }
}
