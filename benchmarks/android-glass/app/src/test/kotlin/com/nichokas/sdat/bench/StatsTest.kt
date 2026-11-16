package com.nichokas.sdat.bench

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class StatsTest {
    private fun frame(ms: Double, ui: Double = 0.0, render: Double = 0.0, gpu: Double = 0.0, deadline: Double = 16.6, interval: Double = 16.6, dropped: Int = 0) =
        Frame((ms * 1e6).toLong(), (ui * 1e6).toLong(), (render * 1e6).toLong(), (gpu * 1e6).toLong(), (deadline * 1e6).toLong(), intervalNs = (interval * 1e6).toLong(), dropped = dropped)

    @Test fun percentileIsNearestRank() {
        val sorted = (1L..10L).toList()
        assertEquals(5L, percentile(sorted, 50.0))
        assertEquals(9L, percentile(sorted, 90.0))
        assertEquals(10L, percentile(sorted, 99.0))
        assertEquals(0L, percentile(emptyList(), 90.0))
    }

    @Test fun countsSlowFramesAgainst60HzAndTheDisplayDeadline() {
        // A 120 Hz display: 10 ms misses its 8.3 ms deadline but fits a 60 Hz frame.
        val s = stats(listOf(frame(10.0, deadline = 8.3), frame(20.0, deadline = 8.3), frame(5.0, deadline = 8.3), frame(6.0, deadline = 8.3)), seconds = 1.0)
        assertEquals(4.0, s.fps, 0.0)
        assertEquals(50.0, s.jankPercent, 0.0)
        assertEquals(25.0, s.over60Percent, 0.0)
        assertEquals(20.0, s.maxMs, 0.0)
    }

    @Test fun limiterIsTheSlowestStageAtP90() {
        val s = stats(List(10) { frame(20.0, ui = 4.0, render = 6.0, gpu = 14.0) }, seconds = 1.0)
        assertEquals(Limiter.Gpu, s.limiter)
    }

    @Test fun noFramesIsZeroNotACrash() {
        val s = stats(emptyList(), seconds = 0.0)
        assertEquals(0, s.frames)
        assertEquals(0.0, s.fps, 0.0)
    }

    @Test fun meets60NeedsBothRateAndSmoothness() {
        assertTrue(meets60(fps = 59.5, over60 = 2.0))
        assertFalse(meets60(fps = 59.5, over60 = 12.0))
        assertFalse(meets60(fps = 45.0, over60 = 0.0))
        assertTrue(meets60(fps = 85.0, over60 = 1.0))
    }

    @Test fun aSmooth30HzDisplayDoesNotPass60() {
        val s = stats(List(30) { frame(10.0, deadline = 33.3) }, seconds = 1.0)
        assertFalse(meets60(s.fps, s.over60Percent))
    }

    @Test fun anUnreportedGpuIsNotAMeasuredZero() {
        val s = stats(List(10) { frame(20.0, ui = 4.0, render = 6.0, gpu = -1e-6) }, seconds = 1.0)
        assertEquals(null, s.gpuP90Ms)
        assertEquals(Limiter.RenderThread, s.limiter)
    }

    @Test fun scoreTakesTheSlowestSceneAndCapsTheOnePercentLowAtTheDisplayRate() {
        fun run(scene: Scene, frameMs: Double) = Run(scene, 1, stats(List(60) { frame(frameMs, interval = frameMs) }, seconds = 1.0), 0.0, 0, 0, 0, 0)
        val fast = Scene(Page.Collection, Glass.None)
        val slow = Scene(Page.DiscoverFling, Glass.Live, busyCores = 2)
        val score = score(listOf(run(fast, 5.0), run(slow, 25.0)), refreshHz = 60f)
        assertEquals(slow.label, score.worstLow.label)
        assertEquals(40.0, score.worstLow.low1, 1e-9)
        assertEquals(60.0, score.scenes.first { it.label == fast.label }.low1, 0.0)
        assertEquals(1, score.passing)
    }

    @Test fun fastFramesWithSkippedVsyncsStillShowALowOnePercent() {
        // 5 ms to render, but only every other 60 Hz vsync: the user sees 30 fps.
        val stats = stats(List(30) { frame(5.0, interval = 33.3) }, seconds = 1.0)
        val score = score(listOf(Run(Scene(Page.Player, Glass.Live), 1, stats, 0.0, 0, 0, 0, 0)), refreshHz = 60f)
        assertEquals(30.0, score.worstLow.low1, 0.1)
    }

    @Test fun droppedReportsStillCountAsRenderedFrames() {
        val s = stats(List(300) { i -> frame(10.0, dropped = if (i % 5 == 0) 1 else 0) }, seconds = 6.0)
        assertEquals(60, s.lostReports)
        assertEquals(60.0, s.fps, 1e-9)
    }

    @Test fun anIntervalAcrossLostReportsIsSharedByItsFrames() {
        // 99 smooth frames, then one report after a 100 ms gap that hid 4 lost reports: 5 frames of 20 ms each.
        val frames = List(99) { frame(5.0, interval = 16.7) } + frame(5.0, interval = 100.0, dropped = 4)
        assertEquals(20.0, stats(frames, seconds = 1.75).p99IntervalMs, 0.01)
    }
}
