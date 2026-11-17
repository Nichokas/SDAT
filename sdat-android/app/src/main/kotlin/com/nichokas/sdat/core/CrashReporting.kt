package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/** The user's answer to "Send crash reports?"; stays [Unset] until they answer. */
enum class CrashReports(val key: String?) {
    Unset(null),
    Allowed("allowed"),
    Denied("denied");

    companion object {
        fun of(key: String?): CrashReports = entries.firstOrNull { it.key == key } ?: Unset
    }
}

/** Starts and stops the error-reporting SDK. */
interface ErrorReporter {
    fun start(dsn: String)
    fun stop()
}

/**
 * Consent for sending crash reports, and whether the reporter runs because of it. [dsn] is empty in builds that cannot
 * report (dev, local, forks): the reporter never starts then and the UI hides the prompt and the toggle.
 */
class CrashReporting(
    val dsn: String,
    initial: CrashReports,
    private val save: (CrashReports) -> Unit,
    private val reporter: ErrorReporter,
) {
    var consent by mutableStateOf(initial)
        private set
    var running by mutableStateOf(false)
        private set

    val available: Boolean get() = dsn.isNotEmpty()
    val shouldAsk: Boolean get() = available && consent == CrashReports.Unset

    init { sync() }

    fun allow(on: Boolean) = set(if (on) CrashReports.Allowed else CrashReports.Denied)

    /** Forgets the answer, for erasing all app data; the prompt then shows again. */
    fun reset() = set(CrashReports.Unset)

    private fun set(next: CrashReports) {
        consent = next
        save(next)
        sync()
    }

    private fun sync() {
        val want = available && consent == CrashReports.Allowed
        if (want == running) return
        if (want) reporter.start(dsn) else reporter.stop()
        running = want
    }
}
