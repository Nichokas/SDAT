package com.nichokas.sdat.ui

import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import androidx.core.content.FileProvider
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import com.nichokas.sdat.BuildConfig
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.CrashReporting
import com.nichokas.sdat.core.CrashReports
import com.nichokas.sdat.core.DiagFmt
import com.nichokas.sdat.core.LogFilter
import com.nichokas.sdat.core.label
import com.nichokas.sdat.ui.kit.AlertButton
import com.nichokas.sdat.ui.kit.AlertButtonRole
import com.nichokas.sdat.ui.kit.AlertDialog
import com.nichokas.sdat.ui.kit.AlertFieldGroup
import com.nichokas.sdat.ui.kit.AlertTextField
import com.nichokas.sdat.ui.kit.BottomSheet
import com.nichokas.sdat.ui.kit.GroupedFooter
import com.nichokas.sdat.ui.kit.ListRow
import com.nichokas.sdat.ui.kit.Menu
import com.nichokas.sdat.ui.kit.MenuAction
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import com.nichokas.sdat.ui.kit.Switch
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.tonearm_ffi.DiagEvent
import uniffi.tonearm_ffi.DiagLevel
import uniffi.tonearm_ffi.DiagStats
import uniffi.tonearm_ffi.diagEvents
import uniffi.tonearm_ffi.diagMark
import uniffi.tonearm_ffi.diagPreviousSession
import uniffi.tonearm_ffi.diagRecord
import uniffi.tonearm_ffi.diagReport
import uniffi.tonearm_ffi.diagSetVerbose
import uniffi.tonearm_ffi.diagStats
import uniffi.tonearm_ffi.diagVerbose

private const val REFRESH_MS = 2_000L

private const val KEPT_EVENTS = 2000

// The core keeps 2000 events; rows are plain composables in one column, so only the newest are drawn.
private const val SHOWN_EVENTS = 100

private class Detail(val title: String, val body: String)

/** Settings > Diagnostics: stats, network, the core's log, bug-hunting tools and the crash-report switch. */
@Composable
fun DiagnosticsView(crashReporting: CrashReporting, onBack: () -> Unit) {
    val context = LocalContext.current
    var stats by remember { mutableStateOf<DiagStats?>(null) }
    var events by remember { mutableStateOf<List<DiagEvent>>(emptyList()) }
    var filter by remember { mutableStateOf(LogFilter.All) }
    var verbose by remember { mutableStateOf(diagVerbose()) }
    var detail by remember { mutableStateOf<Detail?>(null) }
    var askMarker by remember { mutableStateOf(false) }
    var previous by remember { mutableStateOf("") }
    val scope = rememberCoroutineScope()

    // Polls only while the page is on screen: the effect leaves with the page, and repeatOnLifecycle with the app's foreground.
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    LaunchedEffect(Unit) { previous = withContext(Dispatchers.IO) { diagPreviousSession() } }
    LaunchedEffect(lifecycle) {
        lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                // The first poll copies up to the core's whole log across FFI; keep it off the UI thread.
                stats = withContext(Dispatchers.IO) { diagStats() }
                val since = events.firstOrNull()?.seq ?: 0uL
                val fresh = withContext(Dispatchers.IO) { diagEvents(since) }
                if (fresh.isNotEmpty()) events = (fresh.asReversed() + events).take(KEPT_EVENTS)
                delay(REFRESH_MS)
            }
        }
    }

    SdatPage(title = "Diagnostics", onBack = onBack, showsLargeTitle = false, bottomPadding = 0.dp) {
        item {
            Section("Overview") {
                ListRow("App version", detail = BuildConfig.VERSION_NAME, showTopSeparator = false)
                ListRow("Android", detail = androidVersion())
                ListRow("Device", detail = deviceName())
                stats?.let {
                    ListRow("Uptime", detail = DiagFmt.duration(it.uptimeMs.toLong()))
                    ListRow("CPU time, user", detail = DiagFmt.cpu(it.cpuUserMs.toLong()))
                    ListRow("CPU time, system", detail = DiagFmt.cpu(it.cpuSystemMs.toLong()))
                    ListRow("Peak memory", detail = DiagFmt.memory(it.maxRssKb.toLong()))
                }
            }
        }
        item {
            Section("Network") {
                val net = stats?.net.orEmpty()
                net.forEachIndexed { i, stat ->
                    val line = DiagFmt.network(stat)
                    ListRow(
                        stat.category.label,
                        detail = if (line == null) "No requests" else "${stat.requests} requests",
                        showTopSeparator = i > 0,
                    )
                    if (line != null) {
                        Text(
                            line,
                            style = SdatType.footnote,
                            color = SdatTheme.colors.secondaryContent,
                            modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 10.dp),
                        )
                    }
                }
            }
        }
        item {
            val shown = events.filter { filter.accepts(it.level) }.take(SHOWN_EVENTS)
            Section("Log") {
                ListRow("Show", showTopSeparator = false, trailing = { FilterMenu(filter) { filter = it } })
                if (shown.isEmpty()) ListRow("No events", titleColor = SdatTheme.colors.secondaryContent)
                shown.forEach { e ->
                    ListRow(
                        e.message,
                        subtitle = "${DiagFmt.clock(e.atMs)} · ${e.level.label} · ${e.target}",
                        titleColor = if (e.level == DiagLevel.ERROR) SdatTheme.colors.destructive else Color.Unspecified,
                        onClick = { detail = Detail(e.level.label, eventText(e)) },
                    )
                }
            }
            GroupedFooter("Newest first. Tap an event for the full message and stack trace.")
        }
        item {
            Section("Bug hunting") {
                ListRow("Verbose logging", showTopSeparator = false, trailing = {
                    Switch(verbose, { verbose = it; diagSetVerbose(it) })
                })
                ListRow("Add marker", showDisclosure = false, onClick = { askMarker = true })
                if (previous.isNotEmpty()) {
                    ListRow("Previous session", onClick = { detail = Detail("Previous session", previous) })
                }
                ListRow("Share report", showDisclosure = false, onClick = { scope.launch { shareReport(context) } })
                if (crashReporting.running) {
                    ListRow("Send test error", showDisclosure = false, onClick = {
                        diagRecord(DiagLevel.ERROR, "diagnostics", "Test error from Diagnostics", Throwable().stackTraceToString())
                        Chrome.flash("Test error sent")
                    })
                }
            }
            GroupedFooter("Markers show up in the log and in reports, to find where something went wrong.")
        }
        if (crashReporting.available) {
            item {
                Section("Crash reports") {
                    ToggleRow("Send crash reports", crashReporting.consent == CrashReports.Allowed, crashReporting::allow, first = true)
                }
                GroupedFooter("Sends the error, a stack trace, the app version, and the device model and Android version to the developer when SDAT crashes or hits an error. No account details.")
            }
        }
    }

    detail?.let { TextSheet(it.title, it.body) { detail = null } }
    if (askMarker) MarkerDialog(onDismiss = { askMarker = false })
}

@Composable
private fun FilterMenu(selection: LogFilter, onSelect: (LogFilter) -> Unit) {
    var open by remember { mutableStateOf(false) }
    Menu(
        expanded = open,
        onExpandedChange = { open = it },
        actions = LogFilter.entries.map { f ->
            MenuAction(f.label, systemName = if (f == selection) "checkmark" else null, onClick = { onSelect(f) })
        },
        anchor = { toggle ->
            Row(
                Modifier.clickable(role = Role.Button, onClickLabel = "Filter the log", onClick = toggle),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(selection.label, style = SdatType.body, color = SdatTheme.colors.secondaryContent)
                SdatIcon("chevron.up.chevron.down", contentDescription = null, size = 12.dp, tint = SdatTheme.colors.secondaryContent)
            }
        },
    )
}

/** A long text to read and copy from: a stack trace or the previous run's log. */
@Composable
private fun TextSheet(title: String, body: String, onDismiss: () -> Unit) {
    BottomSheet(onDismissRequest = onDismiss, title = title) {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp).navigationBarsPadding()) {
            SelectionContainer {
                Text(body, style = SdatType.footnote, fontFamily = FontFamily.Monospace, color = SdatTheme.colors.content)
            }
        }
    }
}

@Composable
private fun MarkerDialog(onDismiss: () -> Unit) {
    var note by remember { mutableStateOf(TextFieldValue("")) }
    val add = {
        val text = note.text.trim()
        if (text.isNotEmpty()) {
            diagMark(text)
            Chrome.flash("Marker added")
        }
        onDismiss()
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = "Add marker",
        message = "A short note in the log, such as what you are about to try.",
        buttons = listOf(
            AlertButton("Cancel", onDismiss, AlertButtonRole.Cancel),
            AlertButton("Add", add),
        ),
    ) {
        AlertFieldGroup {
            AlertTextField(note, { note = it }, placeholder = "Note")
        }
    }
}

private fun eventText(e: DiagEvent): String = buildString {
    append("${DiagFmt.clock(e.atMs)} ${e.level.label} ${e.target}\n\n${e.message}")
    e.backtrace?.let { append("\n\n$it") }
}

private fun androidVersion() = "${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT})"

private fun deviceName() = "${Build.MANUFACTURER} ${Build.MODEL}"

/** As a file: a full log can exceed what an Intent extra may carry. */
private suspend fun shareReport(context: Context) {
    val extra = "SDAT Android ${BuildConfig.VERSION_NAME}\nAndroid ${androidVersion()}\nDevice ${deviceName()}"
    val file = withContext(Dispatchers.IO) {
        File(context.cacheDir, "reports").apply { mkdirs() }.resolve("sdat-diagnostics.txt").apply { writeText(diagReport(extra)) }
    }
    val uri = FileProvider.getUriForFile(context, "${context.packageName}.files", file)
    val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_STREAM, uri)
        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    context.startActivity(Intent.createChooser(send, null))
}
