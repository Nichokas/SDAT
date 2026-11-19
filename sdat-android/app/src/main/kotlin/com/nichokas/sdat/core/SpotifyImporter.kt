package com.nichokas.sdat.core

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.FfiException
import uniffi.tonearm_ffi.ImportListener
import uniffi.tonearm_ffi.ImportProgress
import uniffi.tonearm_ffi.ImportReport
import uniffi.tonearm_ffi.SpotifyImport
import uniffi.tonearm_ffi.spotifyImport

/** iOS SpotifyImporter: one Spotify playlist import at a time, which keeps running while its sheet is closed. */
object SpotifyImporter {
    sealed interface State {
        data object Idle : State
        /** [progress] is null until the core reports its first step. */
        data class Running(val progress: ImportProgress?) : State
        data class Done(val report: ImportReport) : State
        data class Failed(val message: String) : State
    }

    var state by mutableStateOf<State>(State.Idle)
        private set
    private val scope = MainScope()
    private var running: SpotifyImport? = null

    fun start(link: String) {
        if (state is State.Running) return
        val import = spotifyImport(link)
        running = import
        state = State.Running(null)
        val listener = object : ImportListener {
            override fun onProgress(progress: ImportProgress) {
                scope.launch { if (running === import) state = State.Running(progress) }
            }
        }
        scope.launch {
            val result = try {
                State.Done(import.run(listener))
            } catch (e: CancellationException) {
                throw e
            } catch (_: FfiException.Cancelled) {
                State.Idle
            } catch (e: FfiException.Other) {
                // The core words the import's own failures (a private playlist, nothing found) for the user.
                State.Failed(e.detail)
            } catch (e: Exception) {
                State.Failed(friendlyMessage(e))
            } finally {
                import.close()
            }
            // A failed import may have created its playlist too.
            if (result !is State.Idle) OwnPlaylists.load(force = true)
            // An import the account's sign-out cleared reports nothing to the next account.
            if (running === import) {
                running = null
                state = result
            }
        }
    }

    /** Sign-out: stops the import and forgets its result, which belong to the account. */
    fun clear() {
        running?.cancel()
        running = null
        state = State.Idle
    }

    fun cancel() {
        running?.cancel()
    }

    /** Forgets a finished import, so the sheet asks for a link again. */
    fun reset() {
        if (state !is State.Running) state = State.Idle
    }
}
