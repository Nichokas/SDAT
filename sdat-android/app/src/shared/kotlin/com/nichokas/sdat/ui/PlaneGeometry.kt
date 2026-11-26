package com.nichokas.sdat.ui

import kotlin.math.floor

// Pure Kotlin, outside src/main so benchmarks/android-glass compiles the same plane the app lays out.

data class Cell(val c: Int, val r: Int)

data class Frame(val x: Float, val y: Float, val size: Float)

/** Plane math ported from iOS PlaneScrollView, in dp. */
class PlaneGeometry(private val count: Int) {
    private val sizes = floatArrayOf(96f, 140f, 118f, 100f, 132f, 150f, 108f)
    val largestSize = sizes.max()
    private val gap = 12f
    private val period = sizes.sum() + gap * sizes.size
    private val rows = maxOf(12, kotlin.math.sqrt(count / 1.4).toInt())
    private val stride = maxOf(1, count / rows)

    fun rowSize(r: Int) = sizes[r.mod(sizes.size)]

    fun rowY(r: Int): Float {
        val k = floor(r.toDouble() / sizes.size).toInt()
        var y = k * period
        for (i in 0 until r - k * sizes.size) y += sizes[i] + gap
        return y
    }

    /** Same hash as iOS (`r &* 2_654_435_761` on a 64-bit Int), so rows stagger alike. */
    fun rowOffset(r: Int): Float {
        val h = ((r.toLong() * 2_654_435_761L).toULong() % 1000uL).toFloat() / 1000f
        return h * (rowSize(r) + gap)
    }

    fun frame(cell: Cell): Frame {
        val size = rowSize(cell.r)
        return Frame(cell.c * (size + gap) - rowOffset(cell.r), rowY(cell.r), size)
    }

    fun cells(minX: Float, minY: Float, maxX: Float, maxY: Float): List<Cell> {
        val out = mutableListOf<Cell>()
        var r = floor(minY / period).toInt() * sizes.size - 1
        while (rowY(r) <= maxY) {
            val size = rowSize(r)
            val step = size + gap
            val off = rowOffset(r)
            if (rowY(r) + size >= minY) {
                val c0 = floor((minX + off - size) / step).toInt()
                val c1 = floor((maxX + off) / step).toInt()
                for (c in c0..maxOf(c0, c1)) out += Cell(c, r)
            }
            r++
        }
        return out
    }

    /** Each row continues the pool [stride] covers on, wrapping, so every cover is on the plane (iOS's clamped
     * cols×rows block drops the remainder) and a repeat is still at least [stride] columns or [rows] rows away. */
    fun index(cell: Cell) = (cell.c + cell.r.toLong() * stride).mod(count.toLong()).toInt()
}
