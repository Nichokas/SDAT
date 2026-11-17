package com.nichokas.sdat.core

import java.io.IOException
import uniffi.tonearm_ffi.DiagLevel
import uniffi.tonearm_ffi.FfiException
import uniffi.tonearm_ffi.InternalException
import uniffi.tonearm_ffi.diagRecord

/**
 * Human-readable text for any error shown in the UI (never the raw FfiException dump). Also logs the error to the
 * diagnostics: a warning for failures caused by the user's situation or a third party (offline, signed out, 4xx,
 * Last.fm, downloads), an error, which Sentry gets, for the rest.
 */
fun friendlyMessage(error: Throwable): String {
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
                diagSummary(error),
                // A backtrace makes Sentry treat the event as a report, so only the unexpected ones carry it.
                if (expected) null else error.stackTrace.joinToString("\n") { "\tat $it" },
            )
        }
    }
    return message(error)
}

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
    is DownloadFailure -> "Download failed: ${error.message}"
    else -> error.javaClass.name
}

private fun message(error: Throwable): String = when (error) {
    is FfiException.NotSignedIn -> "You're signed out. Sign in to TIDAL again."
    is FfiException.Network -> "Can't reach TIDAL. Check your connection."
    is FfiException.Http -> when (val status = error.status.toInt()) {
        401, 403 -> "TIDAL refused the request. Try signing in again."
        404 -> "Not found on TIDAL."
        429 -> "Too many requests. Wait a moment and try again."
        in 500..Int.MAX_VALUE -> "TIDAL is having trouble right now. Try again later."
        else -> "TIDAL couldn't handle this request ($status)."
    }
    is FfiException.Decode -> "TIDAL sent something unexpected. Try again."
    is FfiException.LastFm -> "Last.fm: ${error.detail}"
    is FfiException.Cancelled -> "Cancelled."
    is DownloadFailure -> error.message.orEmpty()
    is IOException -> "Can't reach TIDAL. Check your connection."
    else -> "Something went wrong. Please try again."
}
