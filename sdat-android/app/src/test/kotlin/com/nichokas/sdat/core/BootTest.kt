package com.nichokas.sdat.core

import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class BootTest {
    @Test fun weightsFillTheRing() {
        val done = BootPart.entries.fold(BootProgress()) { p, part -> p.with(part, 1.0) }
        assertEquals(1.0, done.value, 1e-9)
        assertEquals(0.0, BootProgress().value, 1e-9)
    }

    @Test fun progressNeverRunsBackwards() {
        val p = BootProgress().with(BootPart.Discover, 0.6).with(BootPart.Discover, 0.2)
        assertEquals(0.3, p.value, 1e-9)
    }

    @Test fun aPartCountsAtMostItsWeight() {
        assertEquals(0.5, BootProgress().with(BootPart.Discover, 3.2).value, 1e-9)
    }

    private class Run {
        var progress = BootProgress()
        val events = mutableListOf<String>()
        val report: (BootPart, Double) -> Unit = { part, f -> progress = progress.with(part, f) }
    }

    private fun step(part: BootPart, timeoutMs: Long = 1_000, body: suspend ((Double) -> Unit) -> Unit = {}) = WarmUpStep(part, timeoutMs, body)

    @Test fun failedAndTimedOutStepsCountAsDone() = runBlocking {
        val run = Run()
        val start = System.nanoTime()
        warmUp(
            discover = step(BootPart.Discover, timeoutMs = 50) { awaitCancellation() },
            library = listOf(step(BootPart.Playlists) { error("offline") }, step(BootPart.Albums), step(BootPart.Favs)),
            covers = step(BootPart.Covers) { throw IllegalStateException() },
            report = run.report,
        )
        assertEquals(1.0, run.progress.value, 1e-9)
        assertTrue((System.nanoTime() - start) / 1_000_000 < 1_000)
    }

    @Test fun stepsReportTheirOwnFraction() = runBlocking {
        val run = Run()
        val seen = mutableListOf<Double>()
        warmUp(
            discover = step(BootPart.Discover) { report -> report(0.5); seen += run.progress.value },
            library = emptyList(),
            covers = step(BootPart.Covers),
            report = run.report,
        )
        assertEquals(listOf(0.25), seen)
    }

    @Test fun coversWaitForTheLibrary() = runBlocking {
        val run = Run()
        warmUp(
            discover = step(BootPart.Discover),
            library = listOf(
                step(BootPart.Playlists) { delay(30); run.events += "playlists" },
                step(BootPart.Albums) { delay(10); run.events += "albums" },
            ),
            covers = step(BootPart.Covers) { run.events += "covers" },
            report = run.report,
        )
        assertEquals(listOf("albums", "playlists", "covers"), run.events)
    }

    @Test fun coversStopWaitingForAStuckLibrary() = runBlocking {
        val run = Run()
        var atCovers: Double? = null
        warmUp(
            discover = step(BootPart.Discover),
            library = listOf(step(BootPart.Playlists, timeoutMs = 5_000) { awaitCancellation() }),
            covers = step(BootPart.Covers) { atCovers = run.progress.value },
            report = run.report,
            libraryWaitMs = 30,
            capMs = 200,
        )
        assertEquals("discover done, playlists still loading", 0.5, atCovers!!, 1e-9)
        assertEquals(1.0, run.progress.value, 1e-9)
    }

    @Test fun theCapEndsEveryStep() = runBlocking {
        val run = Run()
        val start = System.nanoTime()
        warmUp(
            discover = step(BootPart.Discover, timeoutMs = 5_000) { awaitCancellation() },
            library = emptyList(),
            covers = step(BootPart.Covers, timeoutMs = 5_000) { awaitCancellation() },
            report = run.report,
            capMs = 50,
        )
        assertEquals(1.0, run.progress.value, 1e-9)
        assertTrue((System.nanoTime() - start) / 1_000_000 < 1_000)
    }
}
