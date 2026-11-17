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
import uniffi.tonearm_ffi.DiagEvent
import uniffi.tonearm_ffi.DiagLevel
import uniffi.tonearm_ffi.DiagSink

/** Errors and panics are reported; warnings and marks only ride along as breadcrumbs. */
fun DiagEvent.isReportable(): Boolean = level == DiagLevel.ERROR || backtrace != null

/** Gets the core's events on any thread: every one goes to logcat, and to Sentry while it runs. It must never throw. */
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
        }
    }

    override fun stop() = Sentry.close()
}
