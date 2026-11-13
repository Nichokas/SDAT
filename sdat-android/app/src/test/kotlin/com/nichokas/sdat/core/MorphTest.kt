package com.nichokas.sdat.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.runBlocking
import org.junit.Test

class MorphTest {
    private val width = 312f
    private val ts = (0..100).map { it / 100f }

    @Test
    fun expandedShowsTheFullBarBesideTheCircle() {
        for (collapsing in listOf(true, false)) {
            val f = Morph.frame(width, 0f, collapsing)
            assertEquals(width - Morph.GAP - Morph.CIRCLE, f.tabWidth, 0.01f)
            assertEquals(Morph.CIRCLE, f.miniWidth, 0.01f)
        }
    }

    @Test
    fun collapsedShowsTheTabCircleBesideTheCapsule() {
        for (collapsing in listOf(true, false)) {
            val f = Morph.frame(width, 1f, collapsing)
            assertEquals(Morph.ROUND, f.tabWidth, 0.01f)
            assertEquals(width - Morph.GAP - Morph.ROUND, f.miniWidth, 0.01f)
        }
    }

    @Test
    fun theShapesNeverOverlap() {
        for (w in listOf(100f, 312f, 600f)) for (collapsing in listOf(true, false)) for (t in ts) {
            val f = Morph.frame(w, t, collapsing)
            assertTrue("w=$w t=$t collapsing=$collapsing", f.tabWidth + Morph.GAP + f.miniWidth <= maxOf(w, Morph.ROUND + Morph.CIRCLE + Morph.GAP) + 0.01f)
        }
    }

    @Test
    fun widthsMoveOneWayPerDirection() {
        for (collapsing in listOf(true, false)) {
            val frames = ts.map { Morph.frame(width, it, collapsing) }
            frames.zipWithNext().forEach { (a, b) ->
                assertTrue(b.tabWidth <= a.tabWidth + 0.001f)
                assertTrue(b.miniWidth >= a.miniWidth - 0.001f)
            }
        }
    }

    @Test
    fun theShrinkingShapeLeadsTheGrowingOne() {
        for (t in ts) {
            val collapse = Morph.frame(width, t, collapsing = true)
            assertTrue(collapse.miniFraction <= collapse.tabFraction)
            val expand = Morph.frame(width, t, collapsing = false)
            assertTrue(expand.tabFraction >= expand.miniFraction)
        }
    }

    @Test
    fun crossfadeNeverShowsBothContents() {
        for (f in ts) assertFalse("f=$f", Morph.outAlpha(f) > 0f && Morph.inAlpha(f) > 0f)
        assertEquals(1f, Morph.outAlpha(0f))
        assertEquals(1f, Morph.inAlpha(1f))
        assertEquals(1f, Morph.outScale(0f))
        assertEquals(1f, Morph.inScale(1f))
    }

    @Test
    fun scrollingPastTheThresholdCollapses() {
        val bar = BarCollapse()
        assertFalse(bar.scrolled(49f, nowMs = 0).collapsed)
        val collapsed = bar.scrolled(50f, nowMs = 0)
        assertTrue(collapsed.collapsed)
        assertTrue(collapsed.collapsing)
    }

    @Test
    fun scrollingBackExpandsOnceTheMorphHasRun() {
        val collapsed = BarCollapse().scrolled(80f, nowMs = 0)
        assertTrue(collapsed.scrolled(0f, nowMs = 100).collapsed)
        val expanded = collapsed.scrolled(0f, nowMs = Morph.DURATION_MS + 20L)
        assertFalse(expanded.collapsed)
        assertFalse(expanded.collapsing)
    }

    @Test
    fun anExplicitExpandAlsoLocksOutScrolling() {
        val expanded = BarCollapse().scrolled(80f, nowMs = 0).set(false, nowMs = 1000)
        assertFalse(expanded.scrolled(80f, nowMs = 1100).collapsed)
        assertTrue(expanded.scrolled(80f, nowMs = 1000 + Morph.DURATION_MS + 20L).collapsed)
    }

    @Test
    fun scrollingFurtherPastTheFirstItemAfterAnExpandCollapsesAgain() = runBlocking {
        // What the page's collector sees: snapshotFlow drops a value equal to the one before it.
        val seen = flowOf(ListScroll(3, 10), ListScroll(5, 40)).distinctUntilChanged().map { it.dp(density = 2.75f) }.toList()
        assertEquals(2, seen.size)
        val expanded = BarCollapse().scrolled(seen[0], nowMs = 0).set(false, nowMs = 1000)
        assertTrue(expanded.scrolled(seen[1], nowMs = 2000).collapsed)
    }
}
