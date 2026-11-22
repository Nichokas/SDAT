package com.nichokas.sdat.ui

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.graphics.Color
import android.graphics.ImageFormat
import android.hardware.camera2.CameraAccessException
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CameraMetadata
import android.hardware.camera2.CaptureRequest
import android.hardware.camera2.params.OutputConfiguration
import android.hardware.camera2.params.SessionConfiguration
import android.media.ImageReader
import android.os.Handler
import android.os.HandlerThread
import android.util.Size
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.widget.FrameLayout
import com.google.zxing.BinaryBitmap
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.ReaderException
import com.google.zxing.common.HybridBinarizer
import com.google.zxing.qrcode.QRCodeReader
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executor
import java.util.concurrent.TimeUnit
import kotlin.math.abs
import uniffi.tonearm_ffi.liveIsInvite

/**
 * Minimal Camera2 QR reader (iOS QRScanner); reports only SDAT invitations. Camera2 and ZXing's QR decoder keep the
 * APK small, where CameraX and ML Kit would add megabytes. The preview keeps the camera's aspect ratio, centered.
 * All camera state lives on [handler]'s thread.
 */
@SuppressLint("ViewConstructor")
class QrScannerView(context: Context, private val onCode: (String) -> Unit) : FrameLayout(context), SurfaceHolder.Callback {
    private val manager = context.getSystemService(CameraManager::class.java)
    private val preview = SurfaceView(context)
    private val thread = HandlerThread("qr-scanner").apply { start() }
    private val handler = Handler(thread.looper)
    // After release the thread is gone; a late camera callback still runs, so an opened device gets closed.
    private val executor = Executor { if (!handler.post(it)) it.run() }
    private val qr = QRCodeReader()
    // A camera that disconnects while it is queried throws; the scanner then shows no preview instead of crashing.
    private val cameraId = try {
        manager.cameraIdList.firstOrNull {
            manager.getCameraCharacteristics(it).get(CameraCharacteristics.LENS_FACING) == CameraMetadata.LENS_FACING_BACK
        } ?: manager.cameraIdList.firstOrNull()
    } catch (_: CameraAccessException) {
        null
    } catch (_: IllegalArgumentException) {
        null
    }
    private val size: Size? = cameraId?.let { id ->
        try {
            pickSize(id)
        } catch (_: CameraAccessException) {
            null
        } catch (_: IllegalArgumentException) {
            null
        }
    }
    /** The sensor is mounted sideways relative to the display: the preview's width and height swap on screen. */
    private val sideways = cameraId?.let {
        val sensor = try {
            manager.getCameraCharacteristics(it).get(CameraCharacteristics.SENSOR_ORIENTATION) ?: 0
        } catch (_: CameraAccessException) {
            0
        } catch (_: IllegalArgumentException) {
            0
        }
        val display = when (context.display.rotation) {
            Surface.ROTATION_90 -> 90
            Surface.ROTATION_180 -> 180
            Surface.ROTATION_270 -> 270
            else -> 0
        }
        (sensor - display + 360) % 180 != 0
    } ?: false
    @Volatile private var camera: CameraDevice? = null
    /** The open attempt in progress or done; cleared by [close], so callbacks of an older attempt are dropped. */
    private var target: Int? = null
    private var attempts = 0
    /** The preview surface while it exists at the camera's size. */
    private var shown: Surface? = null
    // Another app holding the camera fails the open; it is retried once the camera is free again.
    private val availability = object : CameraManager.AvailabilityCallback() {
        override fun onCameraAvailable(id: String) {
            val s = size
            val surface = shown
            if (id == cameraId && !released && target == null && s != null && surface != null) open(surface, s)
        }
    }
    private var reader: ImageReader? = null
    @Volatile private var done = false
    @Volatile private var released = false

    init {
        setBackgroundColor(Color.BLACK)
        addView(preview, LayoutParams(0, 0, Gravity.CENTER))
        size?.let { preview.holder.setFixedSize(it.width, it.height) }
        preview.holder.addCallback(this)
        manager.registerAvailabilityCallback(availability, handler)
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        super.onMeasure(widthMeasureSpec, heightMeasureSpec)
        val s = size ?: return
        val (w, h) = if (sideways) s.height to s.width else s.width to s.height
        val scale = minOf(measuredWidth.toFloat() / w, measuredHeight.toFloat() / h)
        preview.measure(MeasureSpec.makeMeasureSpec((w * scale).toInt(), MeasureSpec.EXACTLY), MeasureSpec.makeMeasureSpec((h * scale).toInt(), MeasureSpec.EXACTLY))
    }

    override fun surfaceCreated(holder: SurfaceHolder) = Unit

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        val s = size ?: return
        if (width == s.width && height == s.height) handler.post {
            shown = holder.surface
            open(holder.surface, s)
        }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        if (!thread.isAlive) return
        // SurfaceHolder requires the camera thread to stop touching the surface before this returns.
        val closed = CountDownLatch(1)
        val posted = handler.post {
            try {
                shown = null
                close()
            } finally {
                closed.countDown()
            }
        }
        try {
            if (posted) closed.await(500, TimeUnit.MILLISECONDS) else thread.join(500)
        } catch (_: InterruptedException) {
            Thread.currentThread().interrupt()
        }
    }

    /** Stops the camera and its thread; the view is gone. */
    fun release() {
        released = true
        manager.unregisterAvailabilityCallback(availability)
        handler.post(::close)
        thread.quitSafely()
    }

    private fun pickSize(id: String): Size? {
        val map = manager.getCameraCharacteristics(id).get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP) ?: return null
        val previewSizes = map.getOutputSizes(SurfaceHolder::class.java)?.toSet().orEmpty()
        // About 720p: enough detail for a phone-screen QR code, cheap to decode.
        return map.getOutputSizes(ImageFormat.YUV_420_888)
            ?.filter { it in previewSizes && it.width <= 1920 && it.height <= 1080 }
            .orEmpty()
            .minByOrNull { abs(it.width * it.height - 1280 * 720) }
    }

    private fun open(surface: Surface, s: Size) {
        val id = cameraId ?: return
        if (target != null) return
        val attempt = ++attempts
        target = attempt
        guarded { openCamera(id, surface, s, attempt) }
    }

    /** Camera2 throws when the camera is disabled, gone or taken; uncaught on this thread it would end the process. */
    private inline fun guarded(block: () -> Unit) {
        try {
            block()
        } catch (_: CameraAccessException) {
            close()
        } catch (_: IllegalStateException) {
            close()
        } catch (_: SecurityException) {
            close()
        } catch (_: IllegalArgumentException) {
            // A surface destroyed under the camera.
            close()
        }
    }

    private fun openCamera(id: String, surface: Surface, s: Size, attempt: Int) {
        if (context.checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
            target = null
            return
        }
        manager.openCamera(id, executor, object : CameraDevice.StateCallback() {
            override fun onOpened(device: CameraDevice) {
                if (released || target != attempt) return device.close()
                camera = device
                guarded { configure(device, surface, s) }
            }

            override fun onDisconnected(device: CameraDevice) {
                device.close()
                // After release this may run off the camera thread (it has quit): the scanner state is left alone.
                if (released) return
                // camera is still null when the open itself failed: its attempt is cleared, unless a newer one owns it.
                if (camera === device || camera == null && target == attempt) close()
            }

            override fun onError(device: CameraDevice, error: Int) = onDisconnected(device)
        })
    }

    private fun configure(device: CameraDevice, surface: Surface, s: Size) {
        val r = ImageReader.newInstance(s.width, s.height, ImageFormat.YUV_420_888, 2)
        r.setOnImageAvailableListener(::decode, handler)
        reader = r
        val outputs = listOf(OutputConfiguration(surface), OutputConfiguration(r.surface))
        device.createCaptureSession(SessionConfiguration(SessionConfiguration.SESSION_REGULAR, outputs, executor, object : CameraCaptureSession.StateCallback() {
            override fun onConfigured(session: CameraCaptureSession) {
                if (released || camera !== device) return
                guarded {
                    val request = device.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW).apply {
                        addTarget(surface)
                        addTarget(r.surface)
                        set(CaptureRequest.CONTROL_AF_MODE, CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE)
                    }
                    session.setRepeatingRequest(request.build(), null, handler)
                }
            }

            override fun onConfigureFailed(session: CameraCaptureSession) {
                if (released || camera !== device) return
                close()
            }
        }))
    }

    private fun close() {
        target = null
        camera?.close()
        camera = null
        reader?.close()
        reader = null
    }

    private fun decode(r: ImageReader) {
        // A frame callback queued before close() runs after it; the reader is closed by then.
        if (r !== reader) return
        val image = r.acquireLatestImage() ?: return
        try {
            if (done) return
            val plane = image.planes[0]
            val w = image.width
            val h = image.height
            val buffer = plane.buffer
            val luma = ByteArray(w * h)
            for (row in 0 until h) {
                buffer.position(row * plane.rowStride)
                buffer.get(luma, row * w, w)
            }
            val text = try {
                qr.decode(BinaryBitmap(HybridBinarizer(PlanarYUVLuminanceSource(luma, w, h, 0, 0, w, h, false)))).text
            } catch (_: ReaderException) {
                null
            } finally {
                qr.reset()
            }
            if (text != null && liveIsInvite(text)) {
                done = true
                post {
                    if (released) return@post
                    performHapticFeedback(HapticFeedbackConstants.CONFIRM)
                    onCode(text)
                }
            }
        } finally {
            image.close()
        }
    }
}
