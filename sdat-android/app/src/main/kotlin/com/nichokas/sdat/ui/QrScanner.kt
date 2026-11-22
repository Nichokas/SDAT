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
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import kotlin.math.abs
import uniffi.tonearm_ffi.liveIsInvite

/**
 * Coordinates surface validity and release between the UI thread and the camera thread.
 * The active token is 0 when no valid surface exists, and a unique positive id per surface lifecycle.
 */
internal class QrSurfaceGate {
    private val releasedFlag = AtomicBoolean(false)
    private val nextToken = AtomicInteger(0)
    private val activeToken = AtomicInteger(0)

    val released: Boolean get() = releasedFlag.get()
    val hasSurface: Boolean get() = activeToken.get() != 0

    fun isActive(token: Int): Boolean = token != 0 && !releasedFlag.get() && activeToken.get() == token

    fun activateSurface(): Int {
        if (releasedFlag.get()) return 0
        while (true) {
            val current = activeToken.get()
            if (current != 0) return current
            if (releasedFlag.get()) return 0
            val next = nextToken.incrementAndGet()
            if (activeToken.compareAndSet(0, next)) {
                if (releasedFlag.get()) {
                    activeToken.compareAndSet(next, 0)
                    return 0
                }
                return next
            }
        }
    }

    fun markReleased(): Boolean {
        val first = releasedFlag.compareAndSet(false, true)
        activeToken.set(0)
        return first
    }

    fun destroySurface(
        onWorkerThread: () -> Boolean,
        postFront: (Runnable) -> Boolean,
        joinWorker: () -> Unit,
        onWorker: () -> Unit,
    ) {
        activeToken.set(0)
        if (onWorkerThread()) {
            onWorker()
            return
        }
        val closed = CountDownLatch(1)
        val posted = postFront(Runnable {
            try {
                onWorker()
            } finally {
                closed.countDown()
            }
        })
        if (posted) {
            runUninterruptibly { closed.await() }
        } else {
            runUninterruptibly { joinWorker() }
            onWorker()
        }
    }
}

private inline fun runUninterruptibly(wait: () -> Unit) {
    var interrupted = false
    try {
        while (true) {
            try {
                wait()
                return
            } catch (_: InterruptedException) {
                interrupted = true
            }
        }
    } finally {
        if (interrupted) Thread.currentThread().interrupt()
    }
}

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
    private val gate = QrSurfaceGate()
    @Volatile private var camera: CameraDevice? = null
    /** The open attempt in progress or done; cleared by [close], so callbacks of an older attempt are dropped. */
    private var target: Int? = null
    private var attempts = 0
    /** The preview surface while it exists at the camera's size, and the token of its lifecycle. */
    private var shown: Surface? = null
    private var shownToken = 0
    // Another app holding the camera fails the open; it is retried once the camera is free again.
    private val availability = object : CameraManager.AvailabilityCallback() {
        override fun onCameraAvailable(id: String) {
            val s = size
            val surface = shown
            val token = shownToken
            if (id == cameraId && target == null && s != null && surface != null && gate.isActive(token)) open(surface, s, token)
        }
    }
    private var reader: ImageReader? = null
    @Volatile private var done = false

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
        if (width != s.width || height != s.height) return
        val token = gate.activateSurface()
        if (token == 0) return
        val surface = holder.surface
        handler.post {
            if (!gate.isActive(token)) return@post
            shown = surface
            shownToken = token
            open(surface, s, token)
        }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        // SurfaceHolder requires the camera thread to stop touching the surface before this returns.
        gate.destroySurface(
            onWorkerThread = { Thread.currentThread() === thread },
            postFront = handler::postAtFrontOfQueue,
            joinWorker = thread::join,
            onWorker = ::detachAndClose,
        )
    }

    /** Stops the camera and its thread; the view is gone. */
    fun release() {
        if (!gate.markReleased()) return
        manager.unregisterAvailabilityCallback(availability)
        handler.postAtFrontOfQueue(::detachAndClose)
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

    private fun open(surface: Surface, s: Size, token: Int) {
        val id = cameraId ?: return
        if (!gate.isActive(token) || target != null) return
        val attempt = ++attempts
        target = attempt
        guarded { openCamera(id, surface, s, attempt, token) }
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

    private fun openCamera(id: String, surface: Surface, s: Size, attempt: Int, token: Int) {
        if (context.checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
            target = null
            return
        }
        manager.openCamera(id, executor, object : CameraDevice.StateCallback() {
            override fun onOpened(device: CameraDevice) {
                if (!gate.isActive(token) || target != attempt) {
                    device.close()
                    if (!gate.released && target == attempt) target = null
                    return
                }
                camera = device
                guarded { configure(device, surface, s, token) }
            }

            override fun onDisconnected(device: CameraDevice) {
                device.close()
                // After release this may run off the camera thread (it has quit): the scanner state is left alone.
                if (gate.released) return
                // camera is still null when the open itself failed: its attempt is cleared, unless a newer one owns it.
                if (camera === device || camera == null && target == attempt) close()
            }

            override fun onError(device: CameraDevice, error: Int) = onDisconnected(device)
        })
    }

    private fun configure(device: CameraDevice, surface: Surface, s: Size, token: Int) {
        if (!gate.isActive(token) || camera !== device) {
            if (!gate.released && camera === device) close()
            return
        }
        val r = ImageReader.newInstance(s.width, s.height, ImageFormat.YUV_420_888, 2)
        r.setOnImageAvailableListener(::decode, handler)
        reader = r
        val outputs = listOf(OutputConfiguration(surface), OutputConfiguration(r.surface))
        device.createCaptureSession(SessionConfiguration(SessionConfiguration.SESSION_REGULAR, outputs, executor, object : CameraCaptureSession.StateCallback() {
            override fun onConfigured(session: CameraCaptureSession) {
                if (!gate.isActive(token) || camera !== device) return
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
                if (gate.released || camera !== device) return
                close()
            }
        }))
    }

    private fun detachAndClose() {
        shown = null
        shownToken = 0
        close()
    }

    private fun close() {
        target = null
        val dev = camera
        camera = null
        val rdr = reader
        reader = null
        try {
            dev?.close()
        } finally {
            rdr?.close()
        }
    }

    private fun decode(r: ImageReader) {
        // A frame callback queued before close() runs after it; the reader is closed by then.
        if (gate.released || !gate.hasSurface || r !== reader) return
        val image = try {
            r.acquireLatestImage()
        } catch (_: IllegalStateException) {
            null
        } ?: return
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
                    if (gate.released) return@post
                    performHapticFeedback(HapticFeedbackConstants.CONFIRM)
                    onCode(text)
                }
            }
        } finally {
            image.close()
        }
    }
}
