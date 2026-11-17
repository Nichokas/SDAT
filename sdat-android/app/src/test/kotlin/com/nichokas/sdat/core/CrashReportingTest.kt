package com.nichokas.sdat.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.tonearm_ffi.DiagEvent
import uniffi.tonearm_ffi.DiagLevel

class CrashReportingTest {
    private class Fake : ErrorReporter {
        val calls = mutableListOf<String>()
        override fun start(dsn: String) { calls += "start $dsn" }
        override fun stop() { calls += "stop" }
    }

    private val saved = mutableListOf<CrashReports>()
    private val reporter = Fake()

    private fun crash(dsn: String = "https://key@example.invalid/1", initial: CrashReports = CrashReports.Unset) =
        CrashReporting(dsn, initial, { saved += it }, reporter)

    @Test
    fun keysRoundTripAndUnknownKeysAreUnset() {
        CrashReports.entries.forEach { assertEquals(it, CrashReports.of(it.key)) }
        assertEquals(CrashReports.Unset, CrashReports.of("maybe"))
    }

    @Test
    fun asksOnceUntilAnswered() {
        val c = crash()
        assertTrue(c.shouldAsk)
        c.allow(false)
        assertFalse(c.shouldAsk)
        assertEquals(listOf(CrashReports.Denied), saved)
        assertEquals(emptyList<String>(), reporter.calls)
    }

    @Test
    fun startsOnlyWhenAllowedAndStopsWhenTurnedOff() {
        val c = crash()
        c.allow(true)
        c.allow(true)
        assertTrue(c.running)
        c.allow(false)
        assertFalse(c.running)
        assertEquals(listOf("start https://key@example.invalid/1", "stop"), reporter.calls)
    }

    @Test
    fun startsAtLaunchWhenAlreadyAllowed() {
        val c = crash(initial = CrashReports.Allowed)
        assertTrue(c.running)
        assertEquals(listOf("start https://key@example.invalid/1"), reporter.calls)
    }

    @Test
    fun withoutDsnNothingStartsAndNothingIsAsked() {
        val c = crash(dsn = "", initial = CrashReports.Allowed)
        assertFalse(c.available)
        assertFalse(c.running)
        assertFalse(crash(dsn = "").shouldAsk)
        assertEquals(emptyList<String>(), reporter.calls)
    }

    @Test
    fun resetStopsAndAsksAgain() {
        val c = crash(initial = CrashReports.Allowed)
        c.reset()
        assertFalse(c.running)
        assertTrue(c.shouldAsk)
        assertEquals(listOf(CrashReports.Unset), saved)
    }

    @Test
    fun errorsAndPanicsAreReportedButWarningsAndMarksAreNot() {
        fun event(level: DiagLevel, backtrace: String? = null) = DiagEvent(1uL, 0L, level, "t", "m", backtrace)
        assertTrue(event(DiagLevel.ERROR).isReportable())
        assertTrue(event(DiagLevel.WARN, "at Foo").isReportable())
        assertFalse(event(DiagLevel.WARN).isReportable())
        assertFalse(event(DiagLevel.INFO).isReportable())
    }
}
