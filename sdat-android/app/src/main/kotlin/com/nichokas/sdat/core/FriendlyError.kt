package com.nichokas.sdat.core

import java.io.IOException
import uniffi.tonearm_ffi.DiagLevel
import uniffi.tonearm_ffi.FfiException
import uniffi.tonearm_ffi.diagRecord

/**
 * Human-readable text for any error shown in the UI (never the raw FfiException dump). Also logs the error to the
 * diagnostics: a warning for the usual failures (offline, TIDAL refusing), an error, which Sentry gets, for the rest.
 */
fun friendlyMessage(error: Throwable): String {
    if (error !is FfiException.Cancelled) {
        val expected = error is FfiException.NotSignedIn || error is FfiException.Network || error is FfiException.Http ||
            error is FfiException.LastFm || error is DownloadFailure || error is IOException
        runCatching {
            diagRecord(
                if (expected) DiagLevel.WARN else DiagLevel.ERROR,
                "app",
                "${error.javaClass.simpleName}: ${error.message}",
                // A backtrace makes Sentry treat the event as a report, so only the unexpected ones carry it.
                if (expected) null else error.stackTraceToString(),
            )
        }
    }
    return message(error)
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
