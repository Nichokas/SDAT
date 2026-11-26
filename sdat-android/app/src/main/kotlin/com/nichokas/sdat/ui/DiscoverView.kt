package com.nichokas.sdat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Discover
import com.nichokas.sdat.core.DiscoverCache
import com.nichokas.sdat.core.Session
import com.nichokas.sdat.ui.kit.Button
import com.nichokas.sdat.ui.kit.ButtonSize
import com.nichokas.sdat.ui.kit.IconButton
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import com.nichokas.sdat.ui.kit.SheetPage
import uniffi.tonearm_ffi.Mix
import uniffi.tonearm_ffi.Track

/** The iOS Discover: the home mixes' covers on an endless plane, shown from the last pool and refreshed for the next launch. */
@Composable
fun DiscoverView(session: Session, onPlay: (Track, List<Track>, List<Mix>) -> Unit) {
    val context = LocalContext.current
    LaunchedEffect(Unit) { Discover.load(DiscoverCache.of(context)) }
    val tracks = Discover.tracks
    val colors = SdatTheme.colors
    val background = colors.sheet
    SheetPage("Discover", onNavigateBack = null) {
        CoverPlane(tracks, background, colors.fill, onTap = { onPlay(it, tracks, Discover.mixes) })
        // The plane fades into the bottom bar, which has the sheet's colour.
        Box(Modifier.align(Alignment.BottomCenter).fillMaxWidth().height(120.dp).background(Brush.verticalGradient(0f to background.copy(alpha = 0f), 1f to background)))
        if (tracks.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                val message = Discover.error
                if (message == null) {
                    CircularProgressIndicator(color = colors.content)
                } else {
                    Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Text("Couldn't load Discover", style = SdatType.headline, color = colors.content)
                        Text(message, style = SdatType.footnote, color = colors.secondaryContent)
                        Button({ Discover.load(DiscoverCache.of(context)) }, size = ButtonSize.Medium) { Text("Retry") }
                        Button(session::signOut, size = ButtonSize.Medium) { Text("Sign out") }
                    }
                }
            }
        }
    }
}

/** iOS Discover header's shuffle: the endless Discover queue, once the home mixes are in. */
@Composable
fun ShuffleButton(enabled: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    IconButton(onClick, modifier.size(44.dp), enabled = enabled) { SdatIcon("shuffle", "Shuffle Discover", size = 22.dp) }
}
