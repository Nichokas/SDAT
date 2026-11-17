package com.nichokas.sdat.core

import android.content.Context
import android.util.Log
import com.nichokas.sdat.BuildConfig
import io.sentry.Attachment
import io.sentry.Breadcrumb
import io.sentry.Hint
import io.sentry.Sentry
import io.sentry.SentryEvent
import io.sentry.SentryLevel
import io.sentry.android.core.SentryAndroid
import io.sentry.protocol.Message
import java.io.IOException
import uniffi.tonearm_ffi.DiagEvent
import uniffi.tonearm_ffi.DiagLevel
import uniffi.tonearm_ffi.DiagSink
import uniffi.tonearm_ffi.FfiException
import uniffi.tonearm_ffi.InternalException
import uniffi.tonearm_ffi.diagRecord

/** Errors and panics are reported; warnings and marks only ride along as breadcrumbs. */
fun DiagEvent.isReportable(): Boolean = level == DiagLevel.ERROR || backtrace != null

/**
 * Gets the core's warnings, errors, marks and panics on any thread: each goes to logcat, and to Sentry while it runs.
 * It must never throw.
 */
object DiagLog : DiagSink {
    override fun onEvent(event: DiagEvent) {
        runCatching {
            logcat(event)
            if (Sentry.isEnabled()) forward(event)
        }
    }

    private fun logcat(e: DiagEvent) {
        val priority = when (e.level) {
            DiagLevel.ERROR -> Log.ERROR
            DiagLevel.WARN -> Log.WARN
            DiagLevel.INFO -> Log.INFO
            DiagLevel.DEBUG -> Log.DEBUG
            DiagLevel.TRACE -> Log.VERBOSE
        }
        Log.println(priority, "SDAT/${e.target}", e.backtrace?.let { "${e.message}\n$it" } ?: e.message)
    }

    private fun forward(e: DiagEvent) {
        if (e.isReportable()) {
            val event = SentryEvent().apply {
                message = Message().apply { message = e.message }
                level = SentryLevel.ERROR
                setTag("target", e.target)
            }
            val hint = Hint()
            e.backtrace?.let { hint.addAttachment(Attachment(it.toByteArray(), "backtrace.txt")) }
            Sentry.captureEvent(event, hint)
        } else {
            Sentry.addBreadcrumb(Breadcrumb().apply {
                message = e.message
                category = e.target
                level = if (e.level == DiagLevel.WARN) SentryLevel.WARNING else SentryLevel.INFO
            })
        }
    }
}

/** Sentry for crashes and errors. No personal data, tracing, sessions, screenshots or view hierarchy. */
class SentryReporter(private val context: Context) : ErrorReporter {
    override fun start(dsn: String) {
        SentryAndroid.init(context) { o ->
            o.dsn = dsn
            o.release = "sdat-android@${BuildConfig.VERSION_NAME}"
            o.environment = "production"
            o.isSendDefaultPii = false
            o.isEnableAutoSessionTracking = false
            o.isAttachScreenshot = false
            o.isAttachViewHierarchy = false
            // Only the core's own warnings ride along with a report, not the SDK's automatic trail.
            o.isEnableActivityLifecycleBreadcrumbs = false
            o.isEnableAppLifecycleBreadcrumbs = false
            o.isEnableSystemEventBreadcrumbs = false
            o.isEnableAppComponentBreadcrumbs = false
            o.isEnableNetworkEventBreadcrumbs = false
            o.isEnableUserInteractionBreadcrumbs = false
            // The SDK fills in a per-install id; the consent text promises nothing that identifies the user.
            o.setBeforeSend { event, _ ->
                event.apply {
                    user = null
                    contexts.device?.id = null
                }
            }
        }
    }

    override fun stop() = Sentry.close()
}

/**
 * Logs an error the UI shows to the user: a warning for failures caused by the user's situation or a third party
 * (offline, signed out, 4xx, Last.fm, downloads), an error, which Sentry gets, for the rest.
 */
fun recordShownError(error: Throwable) {
    if (error !is FfiException.Cancelled) {
        val expected = error is FfiException.NotSignedIn || error is FfiException.Network ||
            (error is FfiException.Http && error.status.toInt() < 500) || error is FfiException.LastFm ||
            error is DownloadFailure || error is IOException ||
            // A Rust panic: the core's panic hook already reported it with its own backtrace.
            error is InternalException
        runCatching {
            diagRecord(
                if (expected) DiagLevel.WARN else DiagLevel.ERROR,
                "app",
                // The UI call site, as on iOS: it tells apart, and groups in Sentry, errors whose summary is the same.
                "${diagSummary(error)} at ${callSite()}",
                // A backtrace makes Sentry treat the event as a report, so only the unexpected ones carry it.
                if (expected) null else error.stackTrace.joinToString("\n") { "\tat $it" },
            )
        }
    }
}

/** The caller of [friendlyMessage]: frames 0 to 2 are [callSite], [recordShownError] and [friendlyMessage] (R8 inlining can shift it by a frame). */
private fun callSite(): String = Throwable().stackTrace.getOrNull(3)?.let { "${it.className}.${it.methodName}(${it.fileName}:${it.lineNumber})" } ?: "unknown"

/**
 * Kind and status only: error details can hold request URLs or response bodies, and the log goes to logcat and,
 * with consent, Sentry. The backtrace above is frames only for the same reason.
 */
private fun diagSummary(error: Throwable): String = when (error) {
    is FfiException.NotSignedIn -> "Not signed in"
    is FfiException.Network -> "Network error"
    is FfiException.Http -> "HTTP ${error.status}"
    is FfiException.Decode -> "Unexpected response"
    is FfiException.LastFm -> "Last.fm error ${error.code}"
    is FfiException -> "Other error"
    is InternalException -> "Rust panic, reported by the core"
    is DownloadFailure -> "Download failed"
    else -> error.javaClass.name
}
