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
import java.util.concurrent.Executor
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
    }
    private val size: Size? = cameraId?.let { id ->
        try {
            pickSize(id)
        } catch (_: CameraAccessException) {
            null
        }
    }
    /** The sensor is mounted sideways relative to the display: the preview's width and height swap on screen. */
    private val sideways = cameraId?.let {
        val sensor = try {
            manager.getCameraCharacteristics(it).get(CameraCharacteristics.SENSOR_ORIENTATION) ?: 0
        } catch (_: CameraAccessException) {
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
    private var camera: CameraDevice? = null
    /** The surface an open is for; cleared by [close], so an open that completes after it is dropped. */
    private var target: Surface? = null
    private var reader: ImageReader? = null
    @Volatile private var done = false
    @Volatile private var released = false

    init {
        setBackgroundColor(Color.BLACK)
        addView(preview, LayoutParams(0, 0, Gravity.CENTER))
        size?.let { preview.holder.setFixedSize(it.width, it.height) }
        preview.holder.addCallback(this)
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
        if (width == s.width && height == s.height) handler.post { open(holder.surface, s) }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        handler.post(::close)
    }

    /** Stops the camera and its thread; the view is gone. */
    fun release() {
        released = true
        handler.post(::close)
        thread.quitSafely()
    }

    private fun pickSize(id: String): Size? {
        val map = manager.getCameraCharacteristics(id).get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP) ?: return null
        val previewSizes = map.getOutputSizes(SurfaceHolder::class.java).toSet()
        // About 720p: enough detail for a phone-screen QR code, cheap to decode.
        return map.getOutputSizes(ImageFormat.YUV_420_888)
            .filter { it in previewSizes && it.width <= 1920 && it.height <= 1080 }
            .minByOrNull { abs(it.width * it.height - 1280 * 720) }
    }

    private fun open(surface: Surface, s: Size) {
        val id = cameraId ?: return
        if (target != null) return
        target = surface
        guarded { openCamera(id, surface, s) }
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

    private fun openCamera(id: String, surface: Surface, s: Size) {
        if (context.checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) return
        manager.openCamera(id, executor, object : CameraDevice.StateCallback() {
            override fun onOpened(device: CameraDevice) {
                if (released || target !== surface) return device.close()
                camera = device
                guarded { configure(device, surface, s) }
            }

            override fun onDisconnected(device: CameraDevice) {
                device.close()
                if (camera === device) close()
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
                if (camera !== device) return
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
                if (camera === device) close()
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
