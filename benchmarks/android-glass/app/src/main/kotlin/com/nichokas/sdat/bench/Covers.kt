package com.nichokas.sdat.bench

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.Shader
import java.io.File
import kotlin.random.Random

// SDAT's Discover pool target (POOL_TARGET in DiscoverView.kt), so the plane cycles as many covers as the app.
private const val COVERS = 320

// TIDAL's album renditions, as SDAT's coverUrl picks them. 1280 only for the covers the album and player pages show
// full width, so preparation stays short.
private val SIZES = intArrayOf(80, 160, 320, 640)
private const val LARGE = 1280
private const val LARGE_COVERS = 2

/** One album cover in TIDAL's renditions. */
class Cover(private val renditions: Map<Int, File>) {
    /** The smallest rendition that covers [px] pixels, or the largest, as SDAT's coverUrl does. */
    fun file(px: Int): File = renditions.entries.sortedBy { it.key }.let { all -> (all.firstOrNull { it.key >= px } ?: all.last()).value }
}

/**
 * Album-like JPEGs on disk, decoded by Coil as SDAT decodes TIDAL's renditions, so image work stays in the
 * measurement without a network or an account.
 */
fun covers(dir: File): List<Cover> {
    dir.mkdirs()
    return (0 until COVERS).map { i ->
        val sizes = if (i < LARGE_COVERS) SIZES + LARGE else SIZES
        Cover(sizes.associateWith { size -> File(dir, "cover-$i-$size.jpg").also { if (!it.exists()) write(it, cover(i, size)) } })
    }
}

/** Written to its own temporary file and renamed, so neither a run killed mid-write nor an overlapping preparation leaves a broken cover. */
private fun write(file: File, bitmap: Bitmap) {
    val tmp = File.createTempFile(file.nameWithoutExtension + "-", ".tmp", file.parentFile)
    try {
        check(tmp.outputStream().use { out -> bitmap.compress(Bitmap.CompressFormat.JPEG, 85, out) }) { "Could not write $tmp" }
        check(tmp.renameTo(file) || file.exists()) { "Could not move $tmp to $file" }
    } finally {
        tmp.delete()
        bitmap.recycle()
    }
}

/** The same picture at every [size]: the seed draws it in fractions of the side. */
private fun cover(seed: Int, size: Int): Bitmap {
    val random = Random(seed)
    val side = size.toFloat()
    fun color() = android.graphics.Color.HSVToColor(floatArrayOf(random.nextFloat() * 360, 0.4f + random.nextFloat() * 0.6f, 0.3f + random.nextFloat() * 0.7f))
    val bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888)
    val canvas = Canvas(bitmap)
    val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    paint.shader = LinearGradient(0f, 0f, side, side, color(), color(), Shader.TileMode.CLAMP)
    canvas.drawRect(0f, 0f, side, side, paint)
    paint.shader = null
    repeat(6) {
        paint.color = color()
        paint.alpha = 120 + random.nextInt(135)
        canvas.drawCircle(random.nextFloat() * side, random.nextFloat() * side, side * (20 + random.nextFloat() * 120) / 320, paint)
    }
    return bitmap
}
