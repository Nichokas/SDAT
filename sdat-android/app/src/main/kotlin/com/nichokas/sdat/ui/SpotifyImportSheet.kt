package com.nichokas.sdat.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.SpotifyImporter
import com.nichokas.sdat.core.SpotifyImporter.State
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.ui.GlasmioBottomSheet
import io.github.neoruaa.glasmio.ui.GlasmioGroupedList
import io.github.neoruaa.glasmio.ui.GlasmioGroupedTableFooter
import io.github.neoruaa.glasmio.ui.GlasmioListRow
import io.github.neoruaa.glasmio.ui.GlasmioSectionTitle
import io.github.neoruaa.glasmio.ui.GlasmioTextField
import uniffi.tonearm_ffi.ImportProgress
import uniffi.tonearm_ffi.ImportStage
import uniffi.tonearm_ffi.isSpotifyPlaylistLink

/** iOS SpotifyImportSheet: copies a public Spotify playlist into a new TIDAL playlist. */
@Composable
fun SpotifyImportSheet(onDismiss: () -> Unit) {
    val colors = GlasmioTheme.colors
    var link by rememberSaveable { mutableStateOf("") }
    val start = { if (isSpotifyPlaylistLink(link)) SpotifyImporter.start(link) }
    GlasmioBottomSheet(onDismissRequest = onDismiss, title = "Import from Spotify") {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp).navigationBarsPadding()) {
            when (val state = SpotifyImporter.state) {
                State.Idle, is State.Failed -> {
                    GlasmioGroupedList {
                        GlasmioTextField(
                            value = link,
                            onValueChange = { link = it },
                            placeholder = "Spotify playlist link",
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Go),
                            keyboardActions = KeyboardActions(onGo = { start() }),
                        )
                        GlasmioListRow("Import", titleColor = colors.accent, enabled = isSpotifyPlaylistLink(link), showDisclosure = false, onClick = start)
                        if (state is State.Failed) GlasmioListRow(state.message, titleColor = colors.destructive)
                    }
                    GlasmioGroupedTableFooter("The playlist must be public. In Spotify, use Share, then Copy link to playlist. SDAT creates a TIDAL playlist with the tracks TIDAL has.")
                }
                is State.Running -> GlasmioGroupedList {
                    GlasmioListRow(
                        state.progress.label(),
                        showTopSeparator = false,
                        trailing = { CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp) },
                    )
                    GlasmioListRow("Cancel", titleColor = colors.destructive, showDisclosure = false, onClick = SpotifyImporter::cancel)
                }
                is State.Done -> {
                    val report = state.report
                    GlasmioGroupedList {
                        GlasmioListRow(report.name, detail = "${report.imported} tracks", showTopSeparator = false)
                        GlasmioListRow("Open playlist", titleColor = colors.accent, showDisclosure = false, onClick = {
                            onDismiss()
                            SpotifyImporter.reset()
                            Chrome.push(Route.Playlist(report.playlistId))
                        })
                        GlasmioListRow("Import another", titleColor = colors.accent, showDisclosure = false, onClick = { link = ""; SpotifyImporter.reset() })
                    }
                    if (report.skipped > 0u) GlasmioGroupedTableFooter("${report.skipped} entries were skipped: local files, podcast episodes or tracks Spotify no longer has.")
                    if (report.missing.isNotEmpty()) {
                        GlasmioSectionTitle("Not on TIDAL (${report.missing.size})")
                        GlasmioGroupedList {
                            report.missing.forEachIndexed { i, track -> GlasmioListRow(track, showTopSeparator = i > 0) }
                        }
                    }
                }
            }
        }
    }
}

private fun ImportProgress?.label(): String {
    val stage = when (this?.stage) {
        null, ImportStage.READING -> "Reading from Spotify"
        ImportStage.MATCHING -> "Finding tracks on TIDAL"
        ImportStage.SAVING -> "Creating the playlist"
    }
    return if (this == null || total == 0u) "$stage…" else "$stage… $done of $total"
}
