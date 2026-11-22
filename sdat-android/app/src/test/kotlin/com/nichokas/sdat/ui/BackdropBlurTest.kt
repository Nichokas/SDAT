package com.nichokas.sdat.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class BackdropBlurTest {
    private fun argb(a: Int, r: Int, g: Int, b: Int) = (a shl 24) or (r shl 16) or (g shl 8) or b
    private fun alpha(p: Int) = p ushr 24
    private fun red(p: Int) = (p ushr 16) and 0xFF
    private fun green(p: Int) = (p ushr 8) and 0xFF
    private fun blue(p: Int) = p and 0xFF

    @Test fun uniformColorStaysUniform() {
        val color = argb(255, 42, 137, 211)
        val pixels = IntArray(16 * 16) { color }
        blurAndSaturatePixels(pixels, 16, 16, radius = 3, saturation = 1f)
        for (p in pixels) assertEquals(color, p)
    }

    @Test fun sharpEdgeBecomesSmoothGradient() {
        val w = 16
        val h = 16
        val pixels = IntArray(w * h) { i ->
            if (i % w < w / 2) argb(255, 0, 0, 0) else argb(255, 255, 255, 255)
        }
        blurAndSaturatePixels(pixels, w, h, radius = 2, saturation = 1f)
        val row = (0 until w).map { red(pixels[(h / 2) * w + it]) }
        row.zipWithNext().forEach { (a, b) -> assertTrue("non-monotonic: $row", a <= b) }
        assertTrue("left edge should stay dark: ${row.first()}", row.first() < 20)
        assertTrue("right edge should stay bright: ${row.last()}", row.last() > 235)
        assertTrue("boundary left should be softened: ${row[7]}", row[7] in 20..235)
        assertTrue("boundary right should be softened: ${row[8]}", row[8] in 20..235)
        assertTrue("gradient should rise across boundary: ${row[7]} vs ${row[8]}", row[7] < row[8])
        assertEquals(255, alpha(pixels[(h / 2) * w + 7]))
    }

    @Test fun saturationBoostsChromaAndLeavesGrayAndAlphaUnchanged() {
        val colored = intArrayOf(argb(180, 180, 100, 100))
        blurAndSaturatePixels(colored, 1, 1, radius = 0, saturation = 1.8f)
        val c = colored[0]
        assertEquals(180, alpha(c))
        val chroma = maxOf(red(c), green(c), blue(c)) - minOf(red(c), green(c), blue(c))
        assertTrue("expected chroma > 80, got $chroma", chroma > 80)

        val gray = argb(200, 128, 128, 128)
        val grays = intArrayOf(gray)
        blurAndSaturatePixels(grays, 1, 1, radius = 0, saturation = 1.8f)
        assertEquals(gray, grays[0])
    }

    @Test fun zeroAndTinyInputsDoNotCrash() {
        blurAndSaturatePixels(IntArray(0), 0, 0, radius = 5, saturation = 1.8f)
        blurAndSaturatePixels(IntArray(4), -1, 4, radius = 3, saturation = 1.5f)
        blurAndSaturatePixels(IntArray(4), 4, 4, radius = 3, saturation = 1.5f)
        blurAndSaturatePixels(IntArray(4), 100_000, 100_000, radius = 3, saturation = 1.5f)

        val single = argb(255, 90, 140, 200)
        val one = intArrayOf(single)
        blurAndSaturatePixels(one, 1, 1, radius = 10, saturation = 1f)
        assertEquals(single, one[0])

        val untouched = intArrayOf(argb(255, 10, 20, 30), argb(255, 200, 210, 220))
        val copy = untouched.copyOf()
        blurAndSaturatePixels(copy, 2, 1, radius = -2, saturation = 1f)
        assertEquals(untouched.toList(), copy.toList())
    }
}
