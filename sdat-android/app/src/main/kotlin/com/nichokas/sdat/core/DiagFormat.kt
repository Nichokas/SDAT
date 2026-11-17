package com.nichokas.sdat.core

import java.math.BigDecimal
import java.math.RoundingMode
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import uniffi.tonearm_ffi.DiagLevel
import uniffi.tonearm_ffi.DiagNetCategory
import uniffi.tonearm_ffi.DiagNetStat

/** What the Diagnostics log shows: everything, warnings and errors, or errors only. */
enum class LogFilter(val label: String) {
    All("All"),
    Problems("Warnings and errors"),
    Errors("Errors");

    fun accepts(level: DiagLevel): Boolean = when (this) {
        All -> true
        Problems -> level == DiagLevel.ERROR || level == DiagLevel.WARN
        Errors -> level == DiagLevel.ERROR
    }
}

val DiagLevel.label: String
    get() = when (this) {
        DiagLevel.ERROR -> "Error"
        DiagLevel.WARN -> "Warning"
        DiagLevel.INFO -> "Info"
        DiagLevel.DEBUG -> "Debug"
        DiagLevel.TRACE -> "Trace"
    }

val DiagNetCategory.label: String
    get() = when (this) {
        DiagNetCategory.TIDAL -> "TIDAL"
        DiagNetCategory.IMAGES -> "Images"
        DiagNetCategory.LAST_FM -> "Last.fm"
        DiagNetCategory.LISTEN_BRAINZ -> "ListenBrainz"
        DiagNetCategory.OTHER -> "Other"
    }

object DiagFmt {
    private fun tenths(ms: Long) = BigDecimal(ms).divide(BigDecimal(1000), 1, RoundingMode.HALF_EVEN).toPlainString()

    /** "42 s", "5 min 3 s", "2 h 10 min". */
    fun duration(ms: Long): String {
        val s = ms / 1000
        return when {
            s < 60 -> "$s s"
            s < 3600 -> "${s / 60} min ${s % 60} s"
            else -> "${s / 3600} h ${s % 3600 / 60} min"
        }
    }

    /** CPU time keeps a decimal under a minute, where a few hundred ms still matter. */
    fun cpu(ms: Long): String = if (ms < 60_000) "${tenths(ms)} s" else duration(ms)

    /** "850 ms", "1.2 s". */
    fun millis(ms: Long): String = if (ms < 1000) "$ms ms" else "${tenths(ms)} s"

    fun memory(kb: Long): String = Fmt.bytes(kb * 1024)

    fun clock(atMs: Long, zone: ZoneId = ZoneId.systemDefault()): String =
        DateTimeFormatter.ofPattern("HH:mm:ss").format(Instant.ofEpochMilli(atMs).atZone(zone))

    /** The line under a category's request count; null when there were no requests. */
    fun network(stat: DiagNetStat): String? {
        if (stat.requests == 0uL) return null
        return listOfNotNull(
            "${stat.failures} failed",
            "${Fmt.bytes(stat.bytesIn.toLong())} received",
            "avg ${millis((stat.totalMs / stat.requests).toLong())}",
            "max ${millis(stat.maxMs.toLong())}",
            stat.lastStatus?.let { "last HTTP $it" },
        ).joinToString(" · ")
    }
}
