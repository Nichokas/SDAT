package com.nichokas.sdat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.ui.kit.Button
import com.nichokas.sdat.ui.kit.ButtonSize
import com.nichokas.sdat.ui.kit.Glass
import com.nichokas.sdat.ui.kit.GlassHost
import com.nichokas.sdat.ui.kit.GroupedFooter
import com.nichokas.sdat.ui.kit.GroupedList
import com.nichokas.sdat.ui.kit.IconButton
import com.nichokas.sdat.ui.kit.ListRow
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SectionTitle
import com.nichokas.sdat.ui.kit.Slider
import com.nichokas.sdat.ui.kit.Stepper
import com.nichokas.sdat.ui.kit.Switch
import com.nichokas.sdat.ui.kit.TabBar
import com.nichokas.sdat.ui.kit.TabItem
import com.nichokas.sdat.ui.kit.capsule
import org.junit.Test

class KitScreenshots : Screenshots() {
    @Test fun lightKit() = snap("kit-light") { SdatTheme(isDark = false) { Kit() } }
    @Test fun darkKit() = snap("kit-dark") { SdatTheme(isDark = true) { Kit() } }
}

private val tabs = listOf(
    TabItem(0, "Discover", "signpost.right.and.left.fill"),
    TabItem(1, "Search", "magnifyingglass"),
    TabItem(2, "My Collection", "music.note.house.fill"),
)

@Composable
private fun Kit() {
    val colors = SdatTheme.colors
    GlassHost(
        modifier = Modifier.fillMaxSize(),
        content = {
            Column(Modifier.fillMaxSize().background(colors.groupedBackground).padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                SectionTitle("Playback")
                GroupedList {
                    ListRow("Streaming quality", detail = "Max", showTopSeparator = false, onClick = {})
                    ListRow("Preload next track", trailing = { Switch(true, {}) })
                    ListRow("Extra delay", subtitle = "For Bluetooth speakers", trailing = { Stepper(3, {}, range = 0..50) })
                    ListRow("Sign out", titleColor = colors.destructive, showDisclosure = false, onClick = {})
                }
                GroupedFooter("Footer text explaining the section above.")
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    Chip("Pop") {}
                    Chip("Rock", selected = true) {}
                    Button({}, size = ButtonSize.Medium) { Text("Retry") }
                    IconButton({}) { SdatIcon("ellipsis", "More", size = 20.dp) }
                }
                PlayShuffleButtons({}, {})
                Slider(0.4f, {})
                Box(
                    Modifier.fillMaxWidth().height(260.dp)
                        .background(Brush.linearGradient(listOf(Color(0xFFE53935), Color(0xFF8E24AA), Color(0xFF1E88E5), Color(0xFFFDD835)))),
                )
            }
        },
        overlay = {
            Column(Modifier.padding(start = 24.dp, top = 530.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Glass(shape = capsule) { Text("Added to queue", Modifier.padding(horizontal = 16.dp, vertical = 10.dp)) }
                TabBar(tabs, 0, {}, {}, remember { mutableFloatStateOf(0f) }, 54.dp, Modifier.width(420.dp))
                TabBar(tabs, 2, {}, {}, remember { mutableFloatStateOf(1f) }, 54.dp, Modifier.width(420.dp))
            }
        },
    )
}
