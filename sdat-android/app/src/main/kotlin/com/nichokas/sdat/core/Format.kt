package com.nichokas.sdat.core

import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import uniffi.tonearm_ffi.Track

val Track.artistLine: String get() = artists.joinToString(", ") { it.name }
val Track.coverUrl: String? get() = album?.coverUrl
val Track.coverUrlLarge: String? get() = album?.coverUrlLarge ?: album?.coverUrl
val Track.displayTitle: String get() = if (version.isEmpty()) title else "$title ($version)"

object Fmt {
    fun time(secs: Double): String {
        if (!secs.isFinite() || secs < 0) return "0:00"
        val s = secs.toInt()
        return "%d:%02d".format(s / 60, s % 60)
    }

    /** "12 songs · 48 min" */
    fun summary(count: UInt, secs: UInt): String {
        val mins = (secs / 60u).toInt()
        val dur = if (mins >= 60) "${mins / 60} h ${mins % 60} min" else "$mins min"
        return "$count ${if (count == 1u) "song" else "songs"} · $dur"
    }

    fun updated(ts: Long?, zone: ZoneId = ZoneId.systemDefault(), today: LocalDate = LocalDate.now(zone)): String? {
        if (ts == null) return null
        val date = Instant.ofEpochSecond(ts).atZone(zone).toLocalDate()
        if (date == today) return "Updated today"
        if (date == today.minusDays(1)) return "Updated yesterday"
        return "Updated " + date.format(DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM))
    }
}
