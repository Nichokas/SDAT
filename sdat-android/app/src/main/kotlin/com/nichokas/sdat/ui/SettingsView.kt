package com.nichokas.sdat.ui

import android.content.Context
import android.content.ActivityNotFoundException
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
import androidx.compose.runtime.Composable
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
import com.nichokas.sdat.core.Preferences
import com.nichokas.sdat.core.Secrets
import com.nichokas.sdat.core.Session
import com.nichokas.sdat.core.detail
import com.nichokas.sdat.core.eraseAll
import com.nichokas.sdat.core.label
import com.nichokas.sdat.core.qualities
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.scaffold.GlasmioPinnedListPage
import io.github.neoruaa.glasmio.ui.GlasmioAlertButtonLayout
import io.github.neoruaa.glasmio.ui.GlasmioAlertButtonRole
import io.github.neoruaa.glasmio.ui.GlasmioAlertButtonSpec
import io.github.neoruaa.glasmio.ui.GlasmioAlertDialog
import io.github.neoruaa.glasmio.ui.GlasmioGroupedList
import io.github.neoruaa.glasmio.ui.GlasmioGroupedTableFooter
import io.github.neoruaa.glasmio.ui.GlasmioListRow
import io.github.neoruaa.glasmio.ui.GlasmioMenu
import io.github.neoruaa.glasmio.ui.GlasmioMenuItem
import io.github.neoruaa.glasmio.ui.GlasmioSectionTitle
import io.github.neoruaa.glasmio.ui.GlasmioSwitch
import io.github.neoruaa.glasmio.ui.GlasmioTextField
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.Quality
import uniffi.tonearm_ffi.lastfmBeginAuth
import uniffi.tonearm_ffi.lastfmFinishAuth
import uniffi.tonearm_ffi.lastfmUsername

/** iOS SettingsView: same sections, rows and confirmations. */
@Composable
fun SettingsView(session: Session, prefs: Preferences, secrets: Secrets, setQuality: (Quality) -> Unit, setPreload: (Boolean) -> Unit, onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var confirmLogout by remember { mutableStateOf(false) }
    var confirmErase by remember { mutableStateOf(false) }
    var lastFmUser by remember { mutableStateOf(secrets.get(Secrets.LASTFM_USER)) }
    var lastFmPending by rememberSaveable { mutableStateOf<String?>(null) }
    var lastFmError by remember { mutableStateOf<String?>(null) }
    var listenBrainzToken by remember { mutableStateOf("") }
    var hasListenBrainz by remember { mutableStateOf(secrets.get(Secrets.LISTENBRAINZ_TOKEN) != null) }
    var browserOpened by rememberSaveable { mutableStateOf(false) }
    val destructive = GlasmioTheme.colors.destructive

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

    GlasmioPinnedListPage(title = "Settings", showsLargeTitle = false, onNavigateBack = onBack) {
        item {
            Section("Account") {
                GlasmioListRow("Sessions and devices", showTopSeparator = false, onClick = { context.open("https://account.tidal.com/") }, showDisclosure = false)
                GlasmioListRow("Log out", titleColor = destructive, onClick = { confirmLogout = true }, showDisclosure = false)
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
                GlasmioListRow("Download quality", detail = "Highest available", showTopSeparator = false)
                ToggleRow("Download over cellular", prefs.downloadOverCellular, { prefs.downloadOverCellular = it })
            }
        }
        item {
            Section("Scrobbling") {
                ToggleRow("Last.fm", prefs.scrobbleLastFm, { prefs.scrobbleLastFm = it }, first = true)
                if (prefs.scrobbleLastFm) {
                    val user = lastFmUser
                    if (user != null) {
                        GlasmioListRow("Connected as", detail = user)
                        GlasmioListRow("Disconnect Last.fm", titleColor = destructive, showDisclosure = false, onClick = {
                            secrets.set(Secrets.LASTFM_SESSION, null)
                            secrets.set(Secrets.LASTFM_USER, null)
                            lastFmUser = null
                        })
                    } else {
                        GlasmioListRow(
                            if (lastFmPending == null) "Connect Last.fm" else "Connecting…",
                            titleColor = GlasmioTheme.colors.accent,
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
                        lastFmError?.let { GlasmioListRow(it, titleColor = destructive) }
                    }
                }
                ToggleRow("ListenBrainz", prefs.scrobbleListenBrainz, { prefs.scrobbleListenBrainz = it })
                if (prefs.scrobbleListenBrainz) {
                    if (hasListenBrainz) {
                        GlasmioListRow("Remove ListenBrainz token", titleColor = destructive, showDisclosure = false, onClick = {
                            secrets.set(Secrets.LISTENBRAINZ_TOKEN, null)
                            hasListenBrainz = false
                        })
                    } else {
                        GlasmioTextField(
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
            Section("Developer") {
                GlasmioListRow("Erase all app data", titleColor = destructive, showTopSeparator = false, showDisclosure = false, onClick = { confirmErase = true })
            }
            GlasmioGroupedTableFooter("Signs out and deletes the TIDAL session, Last.fm and ListenBrainz tokens, settings, caches and stored files.")
        }
        item {
            Section("About") {
                GlasmioListRow("Version", detail = context.versionName(), showTopSeparator = false)
                GlasmioListRow("Source code", titleColor = GlasmioTheme.colors.accent, showDisclosure = false, onClick = { context.open("https://github.com/nichokas/Tonearm") })
            }
        }
        // Room for the floating tab bar, which stays over Settings as on iOS.
        item { Spacer(Modifier.navigationBarsPadding().height(86.dp)) }
    }

    if (confirmErase) {
        Confirm("Erase all app data?", "This cannot be undone.", "Erase everything", onDismiss = { confirmErase = false }) {
            eraseAll(context, session, prefs, secrets)
        }
    }
    if (confirmLogout) {
        Confirm("Log out of TIDAL?", null, "Log out", onDismiss = { confirmLogout = false }, onConfirm = session::signOut)
    }
}

@Composable
private fun Section(title: String, content: @Composable () -> Unit) {
    GlasmioSectionTitle(title)
    GlasmioGroupedList { content() }
}

@Composable
private fun ToggleRow(title: String, checked: Boolean, onChange: (Boolean) -> Unit, first: Boolean = false) {
    GlasmioListRow(title, showTopSeparator = !first, trailing = { GlasmioSwitch(checked, onChange) })
}

@Composable
private fun QualityRow(title: String, selection: Quality, onSelect: (Quality) -> Unit) {
    GlasmioListRow(title, showTopSeparator = false, trailing = {
        QualityMenu(selection, onSelect) { toggle ->
            Row(
                Modifier.clickable(role = Role.Button, onClickLabel = "Maximum quality", onClick = toggle),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                QualityLogo(selection, 15.dp)
                SdatIcon("chevron.up.chevron.down", contentDescription = null, size = 12.dp, tint = GlasmioTheme.colors.secondaryContent)
            }
        }
    })
}

/** iOS QualityMenu: the streaming-quality picker behind [label], which opens it with the toggle it is given. */
@Composable
fun QualityMenu(selection: Quality, onSelect: (Quality) -> Unit, label: @Composable (toggle: () -> Unit) -> Unit) {
    var open by remember { mutableStateOf(false) }
    GlasmioMenu(expanded = open, onExpandedChange = { open = it }, itemCount = qualities.size, anchor = label) { backdrop, close ->
        qualities.forEach { q ->
            GlasmioMenuItem(
                title = q.label,
                subtitle = q.detail,
                onClick = { onSelect(q); close() },
                systemName = if (q == selection) "checkmark" else null,
                backdrop = backdrop,
            )
        }
    }
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
    GlasmioAlertDialog(
        onDismissRequest = onDismiss,
        title = title,
        message = message,
        buttonLayout = GlasmioAlertButtonLayout.Stacked,
        buttons = listOf(
            GlasmioAlertButtonSpec(action, { onDismiss(); onConfirm() }, GlasmioAlertButtonRole.Destructive),
            GlasmioAlertButtonSpec("Cancel", onDismiss, GlasmioAlertButtonRole.Cancel),
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
