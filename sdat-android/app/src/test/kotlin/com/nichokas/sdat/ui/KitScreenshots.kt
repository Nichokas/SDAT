package com.nichokas.sdat.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Accent
import com.nichokas.sdat.ui.kit.Button
import com.nichokas.sdat.ui.kit.ButtonSize
import com.nichokas.sdat.ui.kit.GroupedFooter
import com.nichokas.sdat.ui.kit.GroupedList
import com.nichokas.sdat.ui.kit.IconButton
import com.nichokas.sdat.ui.kit.ListRow
import com.nichokas.sdat.ui.kit.Raised
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SectionTitle
import com.nichokas.sdat.ui.kit.SheetPage
import com.nichokas.sdat.ui.kit.Slider
import com.nichokas.sdat.ui.kit.Stepper
import com.nichokas.sdat.ui.kit.Switch
import com.nichokas.sdat.ui.kit.capsule
import org.junit.Test

class KitScreenshots : Screenshots() {
    @Test fun lightKit() = snap("kit-light") { SdatTheme(Accent.Blue, isDark = false) { Kit() } }
    @Test fun darkKit() = snap("kit-dark") { SdatTheme(Accent.Rose, isDark = true) { Kit() } }
}

@Composable
private fun Kit() {
    val colors = SdatTheme.colors
    SheetPage("Settings", onNavigateBack = {}, actions = { IconButton({}) { SdatIcon("ellipsis", "More", size = 22.dp) } }) {
        Column(Modifier.fillMaxSize().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
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
            }
            PlayShuffleButtons({}, {})
            Slider(0.4f, {})
            Raised(shape = capsule, color = colors.inverse, contentColor = colors.onInverse) {
                Text("Added to queue", Modifier.padding(horizontal = 16.dp, vertical = 10.dp))
            }
        }
    }
}
