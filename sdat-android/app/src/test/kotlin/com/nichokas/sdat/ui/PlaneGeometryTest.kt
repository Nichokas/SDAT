package com.nichokas.sdat.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PlaneGeometryTest {
    private val plane = PlaneGeometry(320)

    @Test fun rowsRepeatEverySevenSizes() {
        assertEquals(0f, plane.rowY(0))
        assertEquals(108f, plane.rowSize(-1))
        assertEquals(96f + 12f, plane.rowY(1))
        assertEquals(plane.rowY(7) - plane.rowY(0), plane.rowY(0) - plane.rowY(-7))
    }

    @Test fun cellsCoverTheViewport() {
        val cells = plane.cells(0f, 0f, 400f, 800f)
        val frames = cells.map(plane::frame)
        assertTrue(frames.any { it.x <= 0f && it.y <= 0f && it.x + it.size >= 0f })
        // Up to the 12 dp gap between tiles.
        assertTrue(frames.any { it.x + it.size + 12f >= 400f && it.y + it.size + 12f >= 800f })
    }

    @Test fun neighbouringCellsShowDifferentCovers() {
        val cells = plane.cells(0f, 0f, 400f, 300f)
        assertEquals(cells.size, cells.map(plane::index).toSet().size)
    }

    @Test fun picksTheSmallestCoverThatFits() {
        val base = "https://resources.tidal.com/images/ab/cd/320x320.jpg"
        assertEquals("https://resources.tidal.com/images/ab/cd/640x640.jpg", coverUrl(base, 420f))
        assertEquals("https://resources.tidal.com/images/ab/cd/1280x1280.jpg", coverUrl(base, 5000f))
        assertEquals("https://example.com/a.jpg", coverUrl("https://example.com/a.jpg", 100f))
    }
}
