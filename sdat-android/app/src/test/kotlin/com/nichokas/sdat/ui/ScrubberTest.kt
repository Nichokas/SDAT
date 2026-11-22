package com.nichokas.sdat.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class ScrubberTest {
    @Test fun advancesLinearlyWhilePlaying() {
        assertEquals(10.125, interpolatedPosition(10.0, 0.125, 200.0, advancing = true), 1e-6)
    }

    @Test fun holdsAnchorWhenNotAdvancing() {
        assertEquals(10.0, interpolatedPosition(10.0, 0.25, 200.0, advancing = false), 1e-6)
    }

    @Test fun clampsElapsedAtHalfSecond() {
        assertEquals(10.5, interpolatedPosition(10.0, 2.0, 200.0, advancing = true), 1e-6)
    }

    @Test fun clampsNegativeElapsedToZero() {
        assertEquals(10.0, interpolatedPosition(10.0, -0.2, 200.0, advancing = true), 1e-6)
    }

    @Test fun clampsToDurationWithoutMovingBackwards() {
        assertEquals(200.0, interpolatedPosition(199.8, 0.4, 200.0, advancing = true), 1e-6)
        assertEquals(12.0, interpolatedPosition(12.0, 0.3, 0.0, advancing = true), 1e-6)
    }
}
