package com.nichokas.sdat.ui

import android.view.HapticFeedbackConstants
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nichokas.sdat.core.Discover
import com.nichokas.sdat.core.DiscoverCache
import com.nichokas.sdat.core.Session
import com.nichokas.sdat.ui.kit.Glass
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import uniffi.tonearm_ffi.Mix
import uniffi.tonearm_ffi.Track

/** The iOS Discover: the home mixes' covers on an endless plane, shown from the last pool and refreshed for the next launch. */
@Composable
fun DiscoverView(session: Session, onPlay: (Track, List<Track>, List<Mix>) -> Unit) {
    val context = LocalContext.current
    val view = LocalView.current
    LaunchedEffect(Unit) { Discover.load(DiscoverCache.of(context)) }
    val tracks = Discover.tracks
    val colors = SdatTheme.colors
    val background = colors.groupedBackground
    Box(Modifier.fillMaxSize().background(background)) {
        CoverPlane(tracks, background, colors.content.copy(alpha = 0.07f), onTap = {
            view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
            onPlay(it, tracks, Discover.mixes)
        })
        // Scrims keep the title and the bottom bar legible over bright covers, as on iOS.
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
        Text(
            "Discover",
            color = colors.content,
            fontSize = 34.sp,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.statusBarsPadding().padding(start = 20.dp, top = 4.dp).height(44.dp).wrapContentHeight(),
        )
        if (tracks.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                val message = Discover.error
                if (message == null) {
                    CircularProgressIndicator(color = colors.content)
                } else {
                    Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Text("Couldn't load Discover", style = SdatType.headline, color = colors.content)
                        Text(message, style = SdatType.footnote, color = colors.secondaryContent)
                        // Plain buttons: this is the backdrop's sampled layer, where glass would sample itself.
                        TextButton(onClick = { Discover.load(DiscoverCache.of(context)) }) { Text("Retry", color = colors.content) }
                        TextButton(onClick = session::signOut) { Text("Sign out", color = colors.content) }
                    }
                }
            }
        }
    }
}

/** iOS Discover header's shuffle: the endless Discover queue, once the home mixes are in. */
@Composable
fun ShuffleButton(enabled: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    Glass(modifier.size(44.dp), shape = CircleShape) {
        Box(
            Modifier.fillMaxSize().clickable(enabled = enabled, role = Role.Button, onClickLabel = "Shuffle Discover", onClick = onClick),
            contentAlignment = Alignment.Center,
        ) {
            SdatIcon("shuffle", "Shuffle", size = 20.dp, tint = SdatTheme.colors.content.copy(alpha = if (enabled) 1f else 0.4f))
        }
    }
}
