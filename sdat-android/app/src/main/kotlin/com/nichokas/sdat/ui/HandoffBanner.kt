package com.nichokas.sdat.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.spring
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Handoff
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.core.artistLine
import com.nichokas.sdat.core.coverUrl
import com.nichokas.sdat.core.displayTitle
import com.nichokas.sdat.core.elsewhereLine
import com.nichokas.sdat.core.label
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.glass.GlassEmphasis
import io.github.neoruaa.glasmio.glass.GlassSurface
import io.github.neoruaa.glasmio.ui.GlasmioButton
import io.github.neoruaa.glasmio.ui.GlasmioButtonSize
import io.github.neoruaa.glasmio.ui.GlasmioCascadingMenuItem
import kotlinx.coroutines.delay

/** What another device plays or last played, with Continue here. */
@Composable
internal fun HandoffBanner(modifier: Modifier) {
    val shown = Handoff.elsewhere?.takeIf { !Handoff.continuing && !Live.isGuest }
    AnimatedContent(
        targetState = shown,
        modifier = modifier,
        transitionSpec = {
            (slideInVertically(spring()) { it } + fadeIn(spring())) togetherWith (slideOutVertically(spring()) { it } + fadeOut(spring()))
        },
        // Position writes on the other device update the banner in place.
        contentKey = { it?.let { e -> e.device.id to e.track.id } },
        label = "handoff",
    ) { e ->
        if (e == null) return@AnimatedContent
        // Keeps "2 h ago" current while the banner stays up.
        val now by produceState(System.currentTimeMillis()) {
            while (true) {
                delay(60_000)
                value = System.currentTimeMillis()
            }
        }
        GlassSurface(Modifier.fillMaxWidth(), shape = RoundedCornerShape(22.dp)) {
            Row(
                Modifier.padding(10.dp),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Cover(e.track.coverUrl, 44.dp)
                Column(Modifier.weight(1f)) {
                    Text(e.track.displayTitle, style = IosTypography.subheadline, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text(e.track.artistLine, style = IosTypography.caption1, color = GlasmioTheme.colors.secondaryContent, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text(elsewhereLine(e, now), style = IosTypography.caption1, color = GlasmioTheme.colors.accent, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                GlasmioButton(Handoff::continueHere, emphasis = GlassEmphasis.Prominent, size = GlasmioButtonSize.Small) {
                    Text("Continue here", style = IosTypography.subheadline, fontWeight = FontWeight.SemiBold, color = Color.White, maxLines = 1)
                }
            }
        }
    }
}

/** "Send to" entries for the player menu; a guest's playback is the host's. */
internal fun sendToMenuItems(): List<GlasmioCascadingMenuItem> =
    if (Live.isGuest) emptyList()
    else Handoff.devices.mapIndexed { i, d ->
        GlasmioCascadingMenuItem("Send to ${d.label}", "speaker.wave.2", separatorBefore = i == 0, onClick = { Handoff.sendTo(d) })
    }
