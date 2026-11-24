package com.nichokas.sdat.ui

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.widget.Toast
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.core.net.toUri
import androidx.lifecycle.compose.LifecycleResumeEffect
import com.nichokas.sdat.R
import com.nichokas.sdat.core.AudioCache
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.CrashReporting
import com.nichokas.sdat.core.Downloads
import com.nichokas.sdat.core.Fmt
import com.nichokas.sdat.core.Preferences
import com.nichokas.sdat.core.Region
import com.nichokas.sdat.core.Route
import com.nichokas.sdat.core.Secrets
import com.nichokas.sdat.core.Session
import com.nichokas.sdat.core.detail
import com.nichokas.sdat.core.eraseAll
import com.nichokas.sdat.core.label
import com.nichokas.sdat.core.qualities
import com.nichokas.sdat.ui.kit.AlertButton
import com.nichokas.sdat.ui.kit.AlertButtonLayout
import com.nichokas.sdat.ui.kit.AlertButtonRole
import com.nichokas.sdat.ui.kit.AlertDialog
import com.nichokas.sdat.ui.kit.GroupedFooter
import com.nichokas.sdat.ui.kit.GroupedList
import com.nichokas.sdat.ui.kit.ListRow
import com.nichokas.sdat.ui.kit.Menu
import com.nichokas.sdat.ui.kit.MenuAction
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import com.nichokas.sdat.ui.kit.SectionTitle
import com.nichokas.sdat.ui.kit.Switch
import com.nichokas.sdat.ui.kit.TextField
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Quality
import uniffi.tonearm_ffi.audioCacheLimitPresets
import uniffi.tonearm_ffi.lastfmBeginAuth
import uniffi.tonearm_ffi.lastfmFinishAuth
import uniffi.tonearm_ffi.lastfmUsername

/** iOS SettingsView: same sections, rows and confirmations. */
@Composable
fun SettingsView(session: Session, prefs: Preferences, secrets: Secrets, crashReporting: CrashReporting, setQuality: (Quality) -> Unit, setPreload: (Boolean) -> Unit, onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var confirmLogout by remember { mutableStateOf(false) }
    var confirmErase by remember { mutableStateOf(false) }
    var confirmClearCache by remember { mutableStateOf(false) }
    var showSpotifyImport by remember { mutableStateOf(false) }
    var lastFmUser by remember { mutableStateOf(secrets.get(Secrets.LASTFM_USER)) }
    var lastFmPending by rememberSaveable { mutableStateOf<String?>(null) }
    var lastFmError by remember { mutableStateOf<String?>(null) }
    var listenBrainzToken by remember { mutableStateOf("") }
    var hasListenBrainz by remember { mutableStateOf(secrets.get(Secrets.LISTENBRAINZ_TOKEN) != null) }
    var browserOpened by rememberSaveable { mutableStateOf(false) }
    val destructive = SdatTheme.colors.destructive
    LaunchedEffect(Unit) { Region.loadAutomatic() }
    LaunchedEffect(Unit) { AudioCache.refresh() }

    // iOS finishes the Last.fm handshake when its Safari sheet closes; here, when the browser hands back.
    // Reads the state live: the browser can pause the app before a recomposition would restart a keyed effect.
    LifecycleResumeEffect(Unit) {
        val pending = lastFmPending
        if (pending != null && browserOpened) {
            browserOpened = false
            scope.launch {
                try {
                    val sk = lastfmFinishAuth(pending)
                    if (sk == null) {
                        lastFmError = "Last.fm access wasn't approved."
                    } else {
                        secrets.set(Secrets.LASTFM_SESSION, sk)
                        val name = runCatching { lastfmUsername(sk) }.getOrNull() ?: "Last.fm"
                        secrets.set(Secrets.LASTFM_USER, name)
                        lastFmUser = name
                        lastFmError = null
                    }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    lastFmError = e.message ?: e.javaClass.simpleName
                } finally {
                    lastFmPending = null
                }
            }
        }
        onPauseOrDispose { if (lastFmPending != null) browserOpened = true }
    }

    SdatPage(title = "Settings", onBack = onBack, showsLargeTitle = false, bottomPaddingValues = null) {
        item {
            Section("Account") {
                ListRow("Sessions and devices", showTopSeparator = false, onClick = { context.open("https://account.tidal.com/") }, showDisclosure = false)
                ListRow("Log out", titleColor = destructive, onClick = { confirmLogout = true }, showDisclosure = false)
            }
        }
        item {
            Section("Playback") {
                QualityRow("Streaming quality", prefs.streamingQuality, setQuality)
                ToggleRow("Preload next tracks", prefs.preloadNext, setPreload)
            }
        }
        item {
            Section("Downloads") {
                ListRow("Download quality", detail = "Highest available", showTopSeparator = false)
                ToggleRow("Download over cellular", prefs.downloadOverCellular, { prefs.downloadOverCellular = it })
                ListRow("Manage downloads", detail = Fmt.bytes(Downloads.totalSize), onClick = { Chrome.push(Route.Downloads) })
            }
        }
        item {
            val usage = AudioCache.usage
            Section("Cache") {
                ListRow(
                    "Used",
                    detail = if (AudioCache.on) "${Fmt.cacheBytes(usage.bytes.toLong())} of ${Fmt.cacheBytes(AudioCache.limit)} · ${usage.tracks} tracks" else "Off",
                    showTopSeparator = false,
                )
                ListRow("Size limit", trailing = { CacheLimitMenu(AudioCache.limit, AudioCache::setLimit) })
                ListRow("Clear Cache", titleColor = destructive, enabled = usage.tracks > 0u, showDisclosure = false, onClick = { confirmClearCache = true })
            }
            GroupedFooter("Played and upcoming tracks stay on disk, so they start sooner and play without a connection. When the cache is full, the tracks played longest ago go first.")
        }
        item {
            Section("Scrobbling") {
                ToggleRow("Last.fm", prefs.scrobbleLastFm, { prefs.scrobbleLastFm = it }, first = true)
                if (prefs.scrobbleLastFm) {
                    val user = lastFmUser
                    if (user != null) {
                        ListRow("Connected as", detail = user)
                        ListRow("Disconnect Last.fm", titleColor = destructive, showDisclosure = false, onClick = {
                            secrets.set(Secrets.LASTFM_SESSION, null)
                            secrets.set(Secrets.LASTFM_USER, null)
                            lastFmUser = null
                        })
                    } else {
                        ListRow(
                            if (lastFmPending == null) "Connect Last.fm" else "Connecting…",
                            titleColor = SdatTheme.colors.accent,
                            enabled = lastFmPending == null,
                            showDisclosure = false,
                            onClick = {
                                scope.launch {
                                    try {
                                        val auth = lastfmBeginAuth()
                                        browserOpened = false
                                        lastFmPending = auth.token
                                        if (!context.open(auth.url)) lastFmPending = null
                                    } catch (e: CancellationException) {
                                        throw e
                                    } catch (e: Exception) {
                                        lastFmError = e.message ?: e.javaClass.simpleName
                                        lastFmPending = null
                                    }
                                }
                            },
                        )
                        lastFmError?.let { ListRow(it, titleColor = destructive) }
                    }
                }
                ToggleRow("ListenBrainz", prefs.scrobbleListenBrainz, { prefs.scrobbleListenBrainz = it })
                if (prefs.scrobbleListenBrainz) {
                    if (hasListenBrainz) {
                        ListRow("Remove ListenBrainz token", titleColor = destructive, showDisclosure = false, onClick = {
                            secrets.set(Secrets.LISTENBRAINZ_TOKEN, null)
                            hasListenBrainz = false
                        })
                    } else {
                        TextField(
                            value = listenBrainzToken,
                            onValueChange = { listenBrainzToken = it },
                            placeholder = "ListenBrainz user token",
                            visualTransformation = PasswordVisualTransformation(),
                            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                            keyboardActions = KeyboardActions(onDone = {
                                val t = listenBrainzToken.trim()
                                if (t.isNotEmpty()) {
                                    try {
                                        secrets.set(Secrets.LISTENBRAINZ_TOKEN, t)
                                        listenBrainzToken = ""
                                        hasListenBrainz = true
                                    } catch (_: Exception) {
                                        Toast.makeText(context, "Couldn't save the ListenBrainz token", Toast.LENGTH_LONG).show()
                                    }
                                }
                            }),
                        )
                    }
                }
            }
        }
        item {
            Section("Library") {
                ListRow("Import Spotify playlist", showTopSeparator = false, onClick = { showSpotifyImport = true })
            }
        }
        item {
            Section("TIDAL") {
                ListRow("Country / Region", detail = Region.label, showTopSeparator = false, onClick = { Chrome.push(Route.Country) })
            }
            GroupedFooter("Catalogue region for TIDAL requests. Applies to pages you open next.")
        }
        item {
            Section("Developer") {
                ListRow("Diagnostics", showTopSeparator = false, onClick = { Chrome.push(Route.Diagnostics) })
                ListRow("Erase all app data", titleColor = destructive, showDisclosure = false, onClick = { confirmErase = true })
            }
            GroupedFooter("Signs out and deletes the TIDAL session, Last.fm and ListenBrainz tokens, settings, caches and stored files.")
        }
        item {
            Section("About") {
                ListRow("Version", detail = context.versionName(), showTopSeparator = false)
                ListRow("Source code", titleColor = SdatTheme.colors.accent, showDisclosure = false, onClick = { context.open("https://github.com/nichokas/Tonearm") })
            }
        }
        // Room for the floating tab bar, which stays over Settings as on iOS.
        item { Spacer(Modifier.navigationBarsPadding().height(86.dp)) }
    }

    if (showSpotifyImport) SpotifyImportSheet(onDismiss = { showSpotifyImport = false })
    if (confirmErase) {
        Confirm("Erase all app data?", "This cannot be undone.", "Erase everything", onDismiss = { confirmErase = false }) {
            eraseAll(context, session, prefs, secrets, crashReporting)
        }
    }
    if (confirmClearCache) {
        Confirm("Clear the cache?", "Cached tracks are deleted. They download again when you play them.", "Clear Cache", onDismiss = { confirmClearCache = false }, onConfirm = { AudioCache.purge(keepPlaying = true) })
    }
    if (confirmLogout) {
        Confirm("Log out of TIDAL?", null, "Log out", onDismiss = { confirmLogout = false }, onConfirm = session::signOut)
    }
}

@Composable
internal fun Section(title: String, content: @Composable () -> Unit) {
    SectionTitle(title)
    GroupedList { content() }
}

@Composable
internal fun ToggleRow(title: String, checked: Boolean, onChange: (Boolean) -> Unit, first: Boolean = false) {
    ListRow(title, showTopSeparator = !first, trailing = { Switch(checked, onChange) })
}

@Composable
private fun QualityRow(title: String, selection: Quality, onSelect: (Quality) -> Unit) {
    ListRow(title, showTopSeparator = false, trailing = {
        QualityMenu(selection, onSelect) { toggle ->
            Row(
                Modifier.clickable(role = Role.Button, onClickLabel = "Maximum quality", onClick = toggle),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                QualityLogo(selection, 15.dp)
                SdatIcon("chevron.up.chevron.down", contentDescription = null, size = 12.dp, tint = SdatTheme.colors.secondaryContent)
            }
        }
    })
}

/** iOS QualityMenu: the streaming-quality picker behind [label], which opens it with the toggle it is given. */
@Composable
fun QualityMenu(selection: Quality, onSelect: (Quality) -> Unit, label: @Composable (toggle: () -> Unit) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Menu(
        expanded = open,
        onExpandedChange = { open = it },
        actions = qualities.map { q ->
            MenuAction(q.label, systemName = if (q == selection) "checkmark" else null, onClick = { onSelect(q) }, subtitle = q.detail)
        },
        anchor = label,
    )
}

@Composable
private fun CacheLimitMenu(selection: Long, onSelect: (Long) -> Unit) {
    var open by remember { mutableStateOf(false) }
    val presets = remember { audioCacheLimitPresets().map { it.toLong() } }
    val label = { bytes: Long -> if (bytes == 0L) "Off" else Fmt.cacheBytes(bytes) }
    Menu(expanded = open, onExpandedChange = { open = it }, actions = presets.map { bytes ->
        MenuAction(label(bytes), systemName = if (bytes == selection) "checkmark" else null, onClick = { onSelect(bytes) })
    }, anchor = { toggle ->
        Row(
            Modifier.clickable(role = Role.Button, onClickLabel = "Size limit", onClick = toggle),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(label(selection), style = SdatType.body, color = SdatTheme.colors.secondaryContent)
            SdatIcon("chevron.up.chevron.down", contentDescription = null, size = 12.dp, tint = SdatTheme.colors.secondaryContent)
        }
    })
}

/** The iOS quality badge images; scale matches iOS QualityLogo. */
@Composable
fun QualityLogo(quality: Quality, height: Dp) {
    val (res, scale) = when (quality) {
        Quality.HI_FI -> R.drawable.quality_hires to 1.25f
        Quality.CD -> R.drawable.quality_cd to 0.95f
        Quality.AAC -> R.drawable.quality_aac to 1.3f
    }
    Image(painterResource(res), contentDescription = quality.label, modifier = Modifier.height(height * scale))
}

@Composable
private fun Confirm(title: String, message: String?, action: String, onDismiss: () -> Unit, onConfirm: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = title,
        message = message,
        buttonLayout = AlertButtonLayout.Stacked,
        buttons = listOf(
            AlertButton(action, { onDismiss(); onConfirm() }, AlertButtonRole.Destructive),
            AlertButton("Cancel", onDismiss, AlertButtonRole.Cancel),
        ),
    )
}

private fun Context.open(url: String): Boolean = try {
    startActivity(Intent(Intent.ACTION_VIEW, url.toUri()))
    true
} catch (_: ActivityNotFoundException) {
    Toast.makeText(this, "No app can open this link", Toast.LENGTH_LONG).show()
    false
}

private fun Context.versionName() = packageManager.getPackageInfo(packageName, 0).versionName ?: "dev"
