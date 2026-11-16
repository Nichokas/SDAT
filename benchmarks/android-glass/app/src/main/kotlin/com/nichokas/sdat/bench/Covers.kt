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
private const val SIZE = 320

/**
 * Album-like JPEGs on disk, decoded by Coil as SDAT decodes TIDAL's 320 px renditions, so image work stays in the
 * measurement without a network or an account.
 */
fun covers(dir: File): List<File> {
    dir.mkdirs()
    return (0 until COVERS).map { i ->
        File(dir, "cover-$i.jpg").also {
            if (!it.exists()) {
                // Written beside and renamed, so a run killed mid-write never leaves a broken cover behind.
                val tmp = File(dir, "cover-$i.jpg.tmp")
                tmp.outputStream().use { out -> cover(i).compress(Bitmap.CompressFormat.JPEG, 85, out) }
                tmp.renameTo(it)
            }
        }
    }
}

private fun cover(seed: Int): Bitmap {
    val random = Random(seed)
    fun color() = android.graphics.Color.HSVToColor(floatArrayOf(random.nextFloat() * 360, 0.4f + random.nextFloat() * 0.6f, 0.3f + random.nextFloat() * 0.7f))
    val bitmap = Bitmap.createBitmap(SIZE, SIZE, Bitmap.Config.ARGB_8888)
    val canvas = Canvas(bitmap)
    val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    paint.shader = LinearGradient(0f, 0f, SIZE.toFloat(), SIZE.toFloat(), color(), color(), Shader.TileMode.CLAMP)
    canvas.drawRect(0f, 0f, SIZE.toFloat(), SIZE.toFloat(), paint)
    paint.shader = null
    repeat(6) {
        paint.color = color()
        paint.alpha = 120 + random.nextInt(135)
        canvas.drawCircle(random.nextFloat() * SIZE, random.nextFloat() * SIZE, 20 + random.nextFloat() * 120, paint)
    }
    return bitmap
}
