package com.nichokas.sdat.ui

import android.content.Context
import android.content.Intent
import android.os.Build
import android.widget.Toast
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
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
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.scaffold.GlasmioPinnedListPage
import io.github.neoruaa.glasmio.ui.GlasmioAlertButtonRole
import io.github.neoruaa.glasmio.ui.GlasmioAlertButtonSpec
import io.github.neoruaa.glasmio.ui.GlasmioAlertDialog
import io.github.neoruaa.glasmio.ui.GlasmioAlertFieldGroup
import io.github.neoruaa.glasmio.ui.GlasmioAlertTextField
import io.github.neoruaa.glasmio.ui.GlasmioBottomSheet
import io.github.neoruaa.glasmio.ui.GlasmioGroupedTableFooter
import io.github.neoruaa.glasmio.ui.GlasmioListRow
import io.github.neoruaa.glasmio.ui.GlasmioMenu
import io.github.neoruaa.glasmio.ui.GlasmioMenuItem
import io.github.neoruaa.glasmio.ui.GlasmioSwitch
import kotlinx.coroutines.delay
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
    val previous = remember { diagPreviousSession() }

    // Polls only while the page is on screen: the effect leaves with the page, and repeatOnLifecycle with the app's foreground.
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    LaunchedEffect(lifecycle) {
        lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                stats = diagStats()
                val fresh = diagEvents(events.firstOrNull()?.seq ?: 0uL)
                if (fresh.isNotEmpty()) events = (fresh.asReversed() + events).take(KEPT_EVENTS)
                delay(REFRESH_MS)
            }
        }
    }

    GlasmioPinnedListPage(title = "Diagnostics", showsLargeTitle = false, onNavigateBack = onBack) {
        item {
            Section("Overview") {
                GlasmioListRow("App version", detail = BuildConfig.VERSION_NAME, showTopSeparator = false)
                GlasmioListRow("Android", detail = androidVersion())
                GlasmioListRow("Device", detail = deviceName())
                stats?.let {
                    GlasmioListRow("Uptime", detail = DiagFmt.duration(it.uptimeMs.toLong()))
                    GlasmioListRow("CPU time, user", detail = DiagFmt.cpu(it.cpuUserMs.toLong()))
                    GlasmioListRow("CPU time, system", detail = DiagFmt.cpu(it.cpuSystemMs.toLong()))
                    GlasmioListRow("Peak memory", detail = DiagFmt.memory(it.maxRssKb.toLong()))
                }
            }
        }
        item {
            Section("Network") {
                val net = stats?.net.orEmpty()
                net.forEachIndexed { i, stat ->
                    val line = DiagFmt.network(stat)
                    GlasmioListRow(
                        stat.category.label,
                        detail = if (line == null) "No requests" else "${stat.requests} requests",
                        showTopSeparator = i > 0,
                    )
                    if (line != null) {
                        Text(
                            line,
                            style = IosTypography.footnote,
                            color = GlasmioTheme.colors.secondaryContent,
                            modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 10.dp),
                        )
                    }
                }
            }
        }
        item {
            val shown = events.filter { filter.accepts(it.level) }.take(SHOWN_EVENTS)
            Section("Log") {
                GlasmioListRow("Show", showTopSeparator = false, trailing = { FilterMenu(filter) { filter = it } })
                if (shown.isEmpty()) GlasmioListRow("No events", titleColor = GlasmioTheme.colors.secondaryContent)
                shown.forEach { e ->
                    GlasmioListRow(
                        e.message,
                        subtitle = "${DiagFmt.clock(e.atMs)} · ${e.level.label} · ${e.target}",
                        titleColor = if (e.level == DiagLevel.ERROR) GlasmioTheme.colors.destructive else Color.Unspecified,
                        onClick = { detail = Detail(e.level.label, eventText(e)) },
                    )
                }
            }
            GlasmioGroupedTableFooter("Newest first. Tap an event for the full message and stack trace.")
        }
        item {
            Section("Bug hunting") {
                GlasmioListRow("Verbose logging", showTopSeparator = false, trailing = {
                    GlasmioSwitch(verbose, { verbose = it; diagSetVerbose(it) })
                })
                GlasmioListRow("Add marker", showDisclosure = false, onClick = { askMarker = true })
                if (previous.isNotEmpty()) {
                    GlasmioListRow("Previous session", onClick = { detail = Detail("Previous session", previous) })
                }
                GlasmioListRow("Share report", showDisclosure = false, onClick = { shareReport(context) })
                if (crashReporting.running) {
                    GlasmioListRow("Send test error", showDisclosure = false, onClick = {
                        diagRecord(DiagLevel.ERROR, "diagnostics", "Test error from Diagnostics", Throwable().stackTraceToString())
                        Chrome.flash("Test error sent")
                    })
                }
            }
            GlasmioGroupedTableFooter("Markers show up in the log and in reports, to find where something went wrong.")
        }
        if (crashReporting.available) {
            item {
                Section("Crash reports") {
                    ToggleRow("Send crash reports", crashReporting.consent == CrashReports.Allowed, crashReporting::allow, first = true)
                }
                GlasmioGroupedTableFooter("Sends the error, a stack trace and the app version to the developer when SDAT crashes or hits an error. No account details.")
            }
        }
    }

    detail?.let { TextSheet(it.title, it.body) { detail = null } }
    if (askMarker) MarkerDialog(onDismiss = { askMarker = false })
}

@Composable
private fun FilterMenu(selection: LogFilter, onSelect: (LogFilter) -> Unit) {
    var open by remember { mutableStateOf(false) }
    GlasmioMenu(
        expanded = open,
        onExpandedChange = { open = it },
        itemCount = LogFilter.entries.size,
        anchor = { toggle ->
            Row(
                Modifier.clickable(role = Role.Button, onClickLabel = "Filter the log", onClick = toggle),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(selection.label, style = IosTypography.body, color = GlasmioTheme.colors.secondaryContent)
                SdatIcon("chevron.up.chevron.down", contentDescription = null, size = 12.dp, tint = GlasmioTheme.colors.secondaryContent)
            }
        },
    ) { backdrop, close ->
        LogFilter.entries.forEach { f ->
            GlasmioMenuItem(
                title = f.label,
                onClick = { onSelect(f); close() },
                systemName = if (f == selection) "checkmark" else null,
                backdrop = backdrop,
            )
        }
    }
}

/** A long text to read and copy from: a stack trace or the previous run's log. */
@Composable
private fun TextSheet(title: String, body: String, onDismiss: () -> Unit) {
    GlasmioBottomSheet(onDismissRequest = onDismiss, title = title) {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp).navigationBarsPadding()) {
            SelectionContainer {
                Text(body, style = IosTypography.footnote, fontFamily = FontFamily.Monospace, color = GlasmioTheme.colors.content)
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
    GlasmioAlertDialog(
        onDismissRequest = onDismiss,
        title = "Add marker",
        message = "A short note in the log, such as what you are about to try.",
        buttons = listOf(
            GlasmioAlertButtonSpec("Cancel", onDismiss, GlasmioAlertButtonRole.Cancel),
            GlasmioAlertButtonSpec("Add", add),
        ),
    ) {
        GlasmioAlertFieldGroup {
            GlasmioAlertTextField(note, { note = it }, placeholder = "Note")
        }
    }
}

private fun eventText(e: DiagEvent): String = buildString {
    append("${DiagFmt.clock(e.atMs)} ${e.level.label} ${e.target}\n\n${e.message}")
    e.backtrace?.let { append("\n\n$it") }
}

private fun androidVersion() = "${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT})"

private fun deviceName() = "${Build.MANUFACTURER} ${Build.MODEL}"

private fun shareReport(context: Context) {
    val extra = "SDAT Android ${BuildConfig.VERSION_NAME}\nAndroid ${androidVersion()}\nDevice ${deviceName()}"
    val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, diagReport(extra))
    // A very long log can exceed what one Intent may carry.
    try {
        context.startActivity(Intent.createChooser(send, null))
    } catch (_: RuntimeException) {
        Toast.makeText(context, "The report is too large to share", Toast.LENGTH_LONG).show()
    }
}
