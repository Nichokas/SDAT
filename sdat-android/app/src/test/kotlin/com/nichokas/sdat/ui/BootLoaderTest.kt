package com.nichokas.sdat.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class BootLoaderTest {
    private fun lit(seconds: Double) = (0..2).count { squareOpacity(it, seconds, reduceMotion = false) == 1f }

    @Test fun squaresLightUpInSequenceEveryPointThreeSeconds() {
        assertEquals(listOf(0, 1, 2, 3, 0, 1), listOf(0.0, 0.31, 0.61, 0.91, 1.21, 1.51).map(::lit))
        assertEquals(listOf(1f, 1f, 0f), (0..2).map { squareOpacity(it, 0.65, reduceMotion = false) })
    }

    @Test fun reducedMotionFadesThemTogether() {
        for (t in listOf(0.0, 0.4, 1.7, 3.3)) {
            val o = (0..2).map { squareOpacity(it, t, reduceMotion = true) }
            assertEquals(1, o.toSet().size)
            assertTrue(o[0] in 0.55f..1f)
        }
    }
}
