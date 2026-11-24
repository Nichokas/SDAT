package com.nichokas.sdat.ui

import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Region
import com.nichokas.sdat.ui.kit.GroupedFooter
import com.nichokas.sdat.ui.kit.GroupedList
import com.nichokas.sdat.ui.kit.ListRow
import com.nichokas.sdat.ui.kit.SdatTheme

/** iOS's navigation-link Picker for the TIDAL country: Automatic, then every country. */
@Composable
fun CountryView(onBack: () -> Unit) {
    LaunchedEffect(Unit) { Region.loadAutomatic() }
    SdatPage(title = "Country / Region", onBack = onBack, showsLargeTitle = false, bottomPadding = 0.dp) {
        item {
            GroupedList {
                Choice(Region.automaticLabel, Region.preferred == null, first = true) { Region.select(null) }
            }
            GroupedFooter("Automatic uses your account's country, or your location's when signed out.")
        }
        item {
            GroupedList {
                Region.all.forEachIndexed { i, c ->
                    Choice("${c.name} (${c.code})", Region.preferred == c.code, first = i == 0) { Region.select(c.code) }
                }
            }
        }
        item { Spacer(Modifier.navigationBarsPadding().height(86.dp)) }
    }
}

@Composable
private fun Choice(title: String, selected: Boolean, first: Boolean, onSelect: () -> Unit) {
    ListRow(
        title,
        showTopSeparator = !first,
        showDisclosure = false,
        onClick = onSelect,
        trailing = { if (selected) SdatIcon("checkmark", contentDescription = "Selected", size = 18.dp, tint = SdatTheme.colors.accent) },
    )
}
