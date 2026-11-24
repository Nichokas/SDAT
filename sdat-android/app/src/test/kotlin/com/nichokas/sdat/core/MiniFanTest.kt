package com.nichokas.sdat.core

import com.nichokas.sdat.core.MiniFan.Item
import com.nichokas.sdat.core.MiniFan.State
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MiniFanTest {
    @After
    fun reset() = MiniFan.close()

    @Test
    fun slidingTowardEachControlPointsAtIt() {
        assertEquals(Item.Toggle, MiniFan.hit(0f, -60f))
        assertEquals(Item.Previous, MiniFan.hit(-45f, -45f))
        assertEquals(Item.Next, MiniFan.hit(-60f, 0f))
        assertEquals(Item.Toggle, MiniFan.hit(20f, -60f))
    }

    @Test
    fun aSmallWobbleStaysOnTheCloseButton() {
        assertEquals(Item.Close, MiniFan.hit(0f, 0f))
        assertEquals(Item.Close, MiniFan.hit(-20f, -20f))
    }

    @Test
    fun slidingAwayFromTheFanPointsAtNothing() {
        assertNull(MiniFan.hit(60f, 0f))
        assertNull(MiniFan.hit(0f, 60f))
    }

    @Test
    fun controlsSitAboveAndLeftOfTheCircle() {
        assertEquals(0f, Item.Close.x, 0.01f)
        assertEquals(-MiniFan.RADIUS, Item.Toggle.y, 0.01f)
        assertEquals(-MiniFan.RADIUS, Item.Next.x, 0.01f)
        assertTrue(Item.Previous.x < 0 && Item.Previous.y < 0)
    }

    @Test
    fun liftingOnAControlRunsItAndCloses() {
        MiniFan.hold()
        assertTrue(MiniFan.move(-60f, 0f))
        assertEquals(Item.Next, MiniFan.release())
        assertEquals(State.Closed, MiniFan.state)
    }

    @Test
    fun liftingOnTheCloseButtonKeepsTheFanOpen() {
        MiniFan.hold()
        assertFalse(MiniFan.move(5f, 5f))
        assertNull(MiniFan.release())
        assertEquals(State.Open, MiniFan.state)
    }

    @Test
    fun liftingOnNothingCloses() {
        MiniFan.hold()
        assertFalse(MiniFan.move(60f, 60f))
        assertNull(MiniFan.release())
        assertEquals(State.Closed, MiniFan.state)
    }

    @Test
    fun movesAfterTheFanClosedAreIgnored() {
        assertFalse(MiniFan.move(-60f, 0f))
        assertEquals(State.Closed, MiniFan.state)
    }
}
