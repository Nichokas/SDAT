package com.nichokas.sdat.core

import java.time.ZoneOffset
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.tonearm_ffi.DiagLevel
import uniffi.tonearm_ffi.DiagNetCategory
import uniffi.tonearm_ffi.DiagNetStat

class DiagFormatTest {
    @Test
    fun filtersKeepTheirLevels() {
        assertTrue(DiagLevel.entries.all(LogFilter.All::accepts))
        assertEquals(listOf(DiagLevel.ERROR, DiagLevel.WARN), DiagLevel.entries.filter(LogFilter.Problems::accepts))
        assertEquals(listOf(DiagLevel.ERROR), DiagLevel.entries.filter(LogFilter.Errors::accepts))
        assertFalse(LogFilter.Errors.accepts(DiagLevel.WARN))
    }

    @Test
    fun durationsReadAtEveryScale() {
        assertEquals("0 s", DiagFmt.duration(999))
        assertEquals("42 s", DiagFmt.duration(42_000))
        assertEquals("5 min 3 s", DiagFmt.duration(303_000))
        assertEquals("2 h 10 min", DiagFmt.duration(7_800_000))
    }

    @Test
    fun cpuTimeKeepsOneDecimalUnderAMinute() {
        assertEquals("0.4 s", DiagFmt.cpu(420))
        assertEquals("12.3 s", DiagFmt.cpu(12_345))
        assertEquals("2 min 5 s", DiagFmt.cpu(125_000))
    }

    @Test
    fun millisecondsSwitchToSeconds() {
        assertEquals("850 ms", DiagFmt.millis(850))
        assertEquals("1.2 s", DiagFmt.millis(1_200))
    }

    @Test
    fun clockUsesTheGivenZone() {
        assertEquals("01:02:03", DiagFmt.clock(3_723_000, ZoneOffset.UTC))
    }

    @Test
    fun networkLineSummarisesARequestedCategory() {
        val stat = DiagNetStat(DiagNetCategory.TIDAL, 4uL, 1uL, 2_000_000uL, 2_000uL, 1_500uL, 404.toUShort())
        assertEquals("1 failed · 2 MB received · avg 500 ms · max 1.5 s · last HTTP 404", DiagFmt.network(stat))
        assertNull(DiagFmt.network(DiagNetStat(DiagNetCategory.IMAGES, 0uL, 0uL, 0uL, 0uL, 0uL, null)))
        assertEquals("Last.fm", DiagNetCategory.LAST_FM.label)
    }
}
