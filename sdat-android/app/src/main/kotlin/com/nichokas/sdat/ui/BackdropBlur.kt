package com.nichokas.sdat.ui

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Rect
import coil3.request.ImageRequest
import coil3.request.allowHardware
import coil3.request.transformations
import coil3.size.Size
import coil3.transform.Transformation
import kotlin.math.roundToInt

/** Builds a Coil request for a pre-blurred 64 px backdrop so sheets and detail headers skip live GPU blur. */
internal fun backdropRequest(
    context: Context,
    url: String,
    saturation: Float = 1f,
    radius: Int = 5,
): ImageRequest = ImageRequest.Builder(context)
    .data(CoverUrl.sized(url, 160))
    .size(160)
    .allowHardware(false)
    .transformations(BackdropBlur(64, radius, saturation))
    .build()

internal class BackdropBlur(
    private val targetSize: Int = 64,
    private val radius: Int = 5,
    private val saturation: Float = 1f,
) : Transformation() {
    override val cacheKey: String = "backdrop-blur-$targetSize-$radius-$saturation"

    override suspend fun transform(input: Bitmap, size: Size): Bitmap {
        if (input.width <= 0 || input.height <= 0) return input
        val maxDim = maxOf(input.width, input.height)
        val scale = minOf(1f, targetSize.coerceAtLeast(1).toFloat() / maxDim)
        val w = (input.width * scale).roundToInt().coerceAtLeast(1)
        val h = (input.height * scale).roundToInt().coerceAtLeast(1)
        // Software Canvas cannot draw a HARDWARE bitmap directly; copy if a caller didn't disable hardware decode.
        val software = if (input.config == Bitmap.Config.HARDWARE) {
            input.copy(Bitmap.Config.ARGB_8888, false) ?: return input
        } else {
            input
        }
        val out = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        try {
            Canvas(out).drawBitmap(
                software,
                null,
                Rect(0, 0, w, h),
                Paint(Paint.FILTER_BITMAP_FLAG),
            )
        } finally {
            if (software !== input) software.recycle()
        }
        val pixels = IntArray(w * h)
        out.getPixels(pixels, 0, w, 0, 0, w, h)
        blurAndSaturatePixels(pixels, w, h, radius, saturation)
        out.setPixels(pixels, 0, w, 0, 0, w, h)
        return out
    }
}

/**
 * Three-pass separable box blur (Gaussian approximation with clamped edges) followed by optional Rec.709 saturation
 * boost on ARGB_8888 pixels in place.
 */
internal fun blurAndSaturatePixels(
    pixels: IntArray,
    width: Int,
    height: Int,
    radius: Int,
    saturation: Float = 1f,
) {
    if (width <= 0 || height <= 0 || width.toLong() * height > pixels.size) return
    val count = width * height
    if (radius > 0) {
        val tmp = IntArray(count)
        val div = 2 * radius + 1
        val half = radius
        repeat(3) {
            for (y in 0 until height) {
                val row = y * width
                val first = pixels[row]
                var sa = (first ushr 24) * (radius + 1)
                var sr = ((first ushr 16) and 0xFF) * (radius + 1)
                var sg = ((first ushr 8) and 0xFF) * (radius + 1)
                var sb = (first and 0xFF) * (radius + 1)
                for (k in 1..radius) {
                    val p = pixels[row + minOf(k, width - 1)]
                    sa += p ushr 24
                    sr += (p ushr 16) and 0xFF
                    sg += (p ushr 8) and 0xFF
                    sb += p and 0xFF
                }
                for (x in 0 until width) {
                    tmp[row + x] = (((sa + half) / div) shl 24) or
                        (((sr + half) / div) shl 16) or
                        (((sg + half) / div) shl 8) or
                        ((sb + half) / div)
                    val add = pixels[row + minOf(x + radius + 1, width - 1)]
                    val sub = pixels[row + maxOf(x - radius, 0)]
                    sa += (add ushr 24) - (sub ushr 24)
                    sr += ((add ushr 16) and 0xFF) - ((sub ushr 16) and 0xFF)
                    sg += ((add ushr 8) and 0xFF) - ((sub ushr 8) and 0xFF)
                    sb += (add and 0xFF) - (sub and 0xFF)
                }
            }
            for (x in 0 until width) {
                val first = tmp[x]
                var sa = (first ushr 24) * (radius + 1)
                var sr = ((first ushr 16) and 0xFF) * (radius + 1)
                var sg = ((first ushr 8) and 0xFF) * (radius + 1)
                var sb = (first and 0xFF) * (radius + 1)
                for (k in 1..radius) {
                    val p = tmp[minOf(k, height - 1) * width + x]
                    sa += p ushr 24
                    sr += (p ushr 16) and 0xFF
                    sg += (p ushr 8) and 0xFF
                    sb += p and 0xFF
                }
                for (y in 0 until height) {
                    pixels[y * width + x] = (((sa + half) / div) shl 24) or
                        (((sr + half) / div) shl 16) or
                        (((sg + half) / div) shl 8) or
                        ((sb + half) / div)
                    val add = tmp[minOf(y + radius + 1, height - 1) * width + x]
                    val sub = tmp[maxOf(y - radius, 0) * width + x]
                    sa += (add ushr 24) - (sub ushr 24)
                    sr += ((add ushr 16) and 0xFF) - ((sub ushr 16) and 0xFF)
                    sg += ((add ushr 8) and 0xFF) - ((sub ushr 8) and 0xFF)
                    sb += (add and 0xFF) - (sub and 0xFF)
                }
            }
        }
    }
    if (saturation != 1f) {
        for (i in 0 until count) {
            val p = pixels[i]
            val r = (p ushr 16) and 0xFF
            val g = (p ushr 8) and 0xFF
            val b = p and 0xFF
            val luma = 0.2126f * r + 0.7152f * g + 0.0722f * b
            val nr = (luma + (r - luma) * saturation).roundToInt().coerceIn(0, 255)
            val ng = (luma + (g - luma) * saturation).roundToInt().coerceIn(0, 255)
            val nb = (luma + (b - luma) * saturation).roundToInt().coerceIn(0, 255)
            pixels[i] = (p and (0xFF shl 24)) or (nr shl 16) or (ng shl 8) or nb
        }
    }
}
