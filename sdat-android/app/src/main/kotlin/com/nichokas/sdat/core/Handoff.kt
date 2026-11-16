package com.nichokas.sdat.core

import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.HandoffDevice
import uniffi.tonearm_ffi.HandoffElsewhere
import uniffi.tonearm_ffi.HandoffEvent
import uniffi.tonearm_ffi.HandoffLocal
import uniffi.tonearm_ffi.HandoffObserver
import uniffi.tonearm_ffi.HandoffRepeat
import uniffi.tonearm_ffi.HandoffResume
import uniffi.tonearm_ffi.Handoff as HandoffLink

/**
 * Continue playback on another device: this install is a player of the TIDAL account, and the core follows the
 * account's playback state. Runs while signed in and in the foreground or playing.
 */
object Handoff {
    private val scope = MainScope()

    /** What another device plays or last played; null hides the banner. */
    var elsewhere by mutableStateOf<HandoffElsewhere?>(null)
        private set
    /** "Send to" targets. */
    var devices by mutableStateOf(emptyList<HandoffDevice>())
        private set
    /** A Continue here is running. */
    var continuing by mutableStateOf(false)
        private set
    /** Set by MainActivity. */
    var foreground by mutableStateOf(false)

    private lateinit var player: Playback
    private lateinit var prefs: Preferences
    private lateinit var deviceName: () -> String
    private var link: HandoffLink? = null
    /** Signed in, and in the foreground or playing. */
    private var wanted = false
    /** Bumped on every start and stop; core callbacks carry the value they began with. */
    private var generation = 0
    /** Upcoming that came while a Continue here ran, applied once its result is played or dropped. */
    private var pendingUpcoming: HandoffEvent.Upcoming? = null

    fun attach(player: Playback, prefs: Preferences, session: Session, deviceName: () -> String) {
        this.player = player
        this.prefs = prefs
        this.deviceName = deviceName
        player.onChange = ::report
        scope.launch {
            snapshotFlow { session.signedIn && (foreground || player.playing) }.collect { wanted = it; if (it) start() else stop() }
        }
        // Also a pause tapped while a stream loads, which the queue saver is not told about.
        scope.launch { snapshotFlow { player.playing }.collect { report() } }
    }

    private fun start() {
        if (link != null) return
        val gen = ++generation
        scope.launch {
            while (gen == generation) {
                try {
                    val h = HandoffLink.start(prefs.handoffInstallationKey, deviceName(), Observer(gen))
                    if (gen != generation) {
                        h.stop()
                        h.close()
                    } else {
                        link = h
                        report()
                    }
                    return@launch
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    Log.i(TAG, "start failed: $e")
                    delay(RETRY_MS)
                }
            }
        }
    }

    /** Signing in over a session: the old one detaches with its own credentials, then the new one starts. */
    fun accountChanging() = stop()

    fun accountChanged() {
        if (wanted) start()
    }

    /** Backgrounded while not playing, and sign-out (before the session is gone, so the detach can still go out). */
    fun stop() {
        generation++
        link?.let {
            it.stop()
            it.close()
        }
        link = null
        elsewhere = null
        devices = emptyList()
        continuing = false
        pendingUpcoming = null
    }

    /** A Listen Together guest plays the host's queue, not its own. */
    private fun report() {
        val h = link ?: return
        if (Live.isGuest) return
        h.report(handoffLocal(player.track?.id, player.userQueue.map { it.id }, player.upNext.map { it.id }, player.livePublishedPositionMs, player.playing, player.repeat))
    }

    fun continueHere() {
        val h = link ?: return
        if (continuing || Live.isGuest) return
        continuing = true
        pendingUpcoming = null
        val gen = generation
        val entry = player.entry
        val controls = player.controls
        scope.launch {
            try {
                val r = h.continueHere()
                // Joined Listen Together meanwhile (the host drives playback now), or
                // the user started, paused or resumed something: that wins.
                if (gen == generation && !Live.isGuest && player.entry == entry && player.controls == controls) play(r)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Log.i(TAG, "continue here failed: $e")
                if (gen == generation) Chrome.flash("Couldn't continue here")
            } finally {
                if (gen == generation) {
                    continuing = false
                    // Also when this result was not played: an Arrived that superseded it may own the held tracks.
                    pendingUpcoming?.let(::append)
                    pendingUpcoming = null
                }
            }
        }
    }

    fun sendTo(device: HandoffDevice) {
        val h = link ?: return
        report()
        val sent = player.entry to player.controls
        val gen = generation
        scope.launch {
            try {
                h.sendTo(device.id)
                // Signed out or into another account meanwhile: nothing of this send applies here.
                if (gen != generation) return@launch
                // Not what the user started while the send ran.
                if ((player.entry to player.controls) == sent && !Live.isGuest) player.pause()
                Chrome.flash("Playing on ${device.label}")
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Log.i(TAG, "send to failed: $e")
                if (gen == generation) Chrome.flash("Couldn't send to ${device.label}")
            }
        }
    }

    private fun play(r: HandoffResume) = player.resume(r.current, r.positionMs, r.repeat.local)

    /** The report this queue edit sends is what the core waits for. */
    private fun append(u: HandoffEvent.Upcoming) {
        if (!Live.isGuest) player.appendUpNext(u.currentId, u.tracks)
    }

    private fun handle(e: HandoffEvent, gen: Int) {
        if (gen != generation) return
        when (e) {
            is HandoffEvent.Devices -> devices = e.devices
            is HandoffEvent.Elsewhere -> elsewhere = e.elsewhere
            // A guest's playback is the host's to drive.
            is HandoffEvent.Arrived -> if (!Live.isGuest) {
                play(e.resume)
                Chrome.flash("Continued from ${e.resume.from.label}")
            }
            // May follow a running Continue here's result, so it waits for that.
            is HandoffEvent.Upcoming -> if (continuing) pendingUpcoming = e else append(e)
            is HandoffEvent.MovedTo -> if (!Live.isGuest) {
                player.pause()
                Chrome.flash("Playing on ${e.device.label}")
            }
        }
    }

    /** Core callbacks arrive on core threads; hop to the main thread. */
    private class Observer(private val gen: Int) : HandoffObserver {
        override fun onEvent(event: HandoffEvent) {
            scope.launch { handle(event, gen) }
        }
    }

    private const val TAG = "handoff"
    private const val RETRY_MS = 30_000L
}

/** TIDAL lists installations it has no name for. */
val HandoffDevice.label get() = name.ifEmpty { "another device" }

/** The player as the core sees it: everything after [current] in play order, Play Next and Add to Queue first. */
internal fun handoffLocal(current: String?, userQueue: List<String>, upNext: List<String>, positionMs: Long, playing: Boolean, repeat: Repeat) =
    HandoffLocal(current, if (current == null) emptyList() else userQueue + upNext, positionMs, playing, repeat.handoff)

internal val Repeat.handoff get() = when (this) {
    Repeat.Off -> HandoffRepeat.OFF
    Repeat.All -> HandoffRepeat.ALL
    Repeat.One -> HandoffRepeat.ONE
}

internal val HandoffRepeat.local get() = when (this) {
    HandoffRepeat.OFF -> Repeat.Off
    HandoffRepeat.ALL -> Repeat.All
    HandoffRepeat.ONE -> Repeat.One
}

/** Upcoming tracks follow [resumedId] only while it plays and the base queue still is that track alone; otherwise the
 * user started something else, even a list at that track, or they are already there. */
internal fun upcomingApplies(resumedId: String, currentId: String?, base: List<String>) = currentId == resumedId && base == listOf(resumedId)

/** "Playing on iPhone", or "Paused on iPhone · 2 h ago". */
internal fun elsewhereLine(e: HandoffElsewhere, nowMs: Long): String {
    val on = e.device.label
    if (e.playing) return "Playing on $on"
    if (e.updatedAtMs <= 0) return "Paused on $on"
    return "Paused on $on · ${Fmt.ago(e.updatedAtMs, nowMs)}"
}
