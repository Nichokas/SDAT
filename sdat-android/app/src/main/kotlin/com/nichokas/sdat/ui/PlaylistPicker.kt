package com.nichokas.sdat.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.OwnPlaylists
import com.nichokas.sdat.core.friendlyMessage
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.ui.GlasmioBottomSheet
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.addToPlaylist

// The add outlives the sheet, which closes as soon as a playlist is picked.
private val pickerScope = MainScope()

/** iOS PlaylistPicker: the user's own playlists, freshly loaded; picking one adds [track] to it. */
@Composable
fun PlaylistPicker(track: Track, onDismiss: () -> Unit) {
    var loaded by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        OwnPlaylists.load(force = true)
        loaded = true
    }
    GlasmioBottomSheet(onDismissRequest = onDismiss, title = "Add to Playlist") {
        val list = OwnPlaylists.list
        when {
            list.isNotEmpty() -> LazyColumn(Modifier.fillMaxWidth()) {
                items(list, key = { it.id }) { p ->
                    Row(
                        Modifier.fillMaxWidth()
                            .clickable(role = Role.Button) {
                                onDismiss()
                                pickerScope.launch {
                                    try {
                                        addToPlaylist(p.id, track.id)
                                        Chrome.flash("Added to ${p.name}")
                                    } catch (e: CancellationException) {
                                        throw e
                                    } catch (e: Exception) {
                                        Chrome.flash(friendlyMessage(e))
                                    }
                                }
                            }
                            .padding(horizontal = 20.dp, vertical = 14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(p.name, style = IosTypography.body, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                        Text("${p.trackCount}", style = IosTypography.body, color = GlasmioTheme.colors.secondaryContent, modifier = Modifier.padding(start = 12.dp))
                    }
                    HorizontalDivider(Modifier.padding(start = 20.dp), color = GlasmioTheme.colors.separator)
                }
            }
            loaded -> Unavailable("music.note.list", "No playlists")
            else -> Box(Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
                CircularProgressIndicator(color = GlasmioTheme.colors.secondaryContent)
            }
        }
    }
}
