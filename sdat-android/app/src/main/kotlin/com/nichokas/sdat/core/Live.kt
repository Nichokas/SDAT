package com.nichokas.sdat.core

import android.os.SystemClock
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.FfiException
import uniffi.tonearm_ffi.LiveCommand
import uniffi.tonearm_ffi.LiveControlObserver
import uniffi.tonearm_ffi.LiveGuest
import uniffi.tonearm_ffi.LiveGuestInfo
import uniffi.tonearm_ffi.LiveHost
import uniffi.tonearm_ffi.LiveObserver
import uniffi.tonearm_ffi.LivePlayNextObserver
import uniffi.tonearm_ffi.LiveUpdate
import uniffi.tonearm_ffi.Track
import uniffi.tonearm_ffi.liveIsInvite
import uniffi.tonearm_ffi.liveTrack

/** A guest's upcoming entry, keyed by the shared queue item id so the same song queued twice gets two rows. */
data class LiveRow(val id: String, val track: Track)

/**
 * Listen Together (iOS Live): one host plays, guests follow on their own TIDAL accounts. Protocol, clock sync and drift
 * decisions live in the Rust core; this drives [Playback].
 */
object Live {
    private val scope = MainScope()

    var mode by mutableStateOf<LiveMode>(LiveMode.Idle)
        private set
    var invite by mutableStateOf<String?>(null)
        private set
    var guests by mutableStateOf(emptyList<LiveGuestInfo>())
        private set
    var hostName by mutableStateOf("")
        private set
    var reconnecting by mutableStateOf(false)
        private set
    /** Guest view of the host's queue. */
    var upNext by mutableStateOf(emptyList<LiveRow>())
        private set
    /** Host: whether guests may play/pause, skip and add songs (off by default). Guest: whether the host allows it. */
    var guestControl by mutableStateOf(false)
        private set
    var showSheet by mutableStateOf(false)
    var showScanner by mutableStateOf(false)

    private lateinit var player: Playback
    private lateinit var prefs: Preferences
    private lateinit var deviceName: () -> String

    /** Manual extra output delay in ms, persisted. */
    var extraLatencyMs: Int
        get() = prefs.liveExtraLatencyMs
        set(v) { prefs.liveExtraLatencyMs = v }

    private var host: LiveHost? = null
    private var guest: LiveGuest? = null
    private var publisher: HostPublisher? = null
    private var follower: GuestFollower? = null
    /** Cancelled on every teardown: ticks, guest loads and request queues of one session. */
    private var session: CoroutineScope? = null
    /** Bumped on every start, join and teardown; core callbacks carry the value they began with. */
    private var generation = 0
    private var upNextRefresh = 0
    private val cache = HashMap<String, Track>()

    /** Guest: queue requests waiting for room in the command budget. */
    private val outbox = ArrayDeque<Pair<List<String>, Boolean>>()
    private var outboxJob: Job? = null
    /** Host: accepted guest track ids, fetched one at a time in arrival order. */
    private val hostEnqueue = ArrayDeque<Pair<String, Boolean>>()
    private var hostEnqueueJob: Job? = null

    fun attach(player: Playback, prefs: Preferences, deviceName: () -> String) {
        this.player = player
        this.prefs = prefs
        this.deviceName = deviceName
    }

    val active get() = mode.active
    /** Reads [mode], so the UI follows it. */
    val isGuest get() = mode == LiveMode.Joining || mode == LiveMode.Joined
    /** A guest who may not play/pause, skip or add songs right now. */
    val playbackLocked get() = isGuest && !guestControl

    // Lifecycle

    fun startHosting() {
        if (active) return
        mode = LiveMode.Starting
        val gen = ++generation
        scope.launch {
            try {
                val h = LiveHost.start(deviceName(), null, Bridge(gen))
                // Cancelled (or replaced) while the endpoint was coming up.
                if (gen != generation || mode != LiveMode.Starting) {
                    h.end()
                    h.close()
                    return@launch
                }
                host = h
                guestControl = false
                h.setControlObserver(ControlBridge(gen))
                h.setPlayNextObserver(ControlBridge(gen))
                invite = h.inviteUrl()
                mode = LiveMode.Hosting
                publisher = HostPublisher(h, SystemClock::uptimeMillis)
                every(500) { hostTick() }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (gen == generation) mode = LiveMode.Ended(friendlyMessage(e))
            }
        }
    }

    /** A scanned QR code or an opened `sdat://join/...` link. */
    fun open(text: String) {
        if (!liveIsInvite(text)) return
        showScanner = false
        showSheet = true
        if (active) leave()
        val gen = ++generation
        try {
            val g = LiveGuest.join(text, deviceName(), null, Bridge(gen))
            guest = g
            g.setControlObserver(ControlBridge(gen))
            // The permission may have arrived before the observer was installed.
            guestControl = g.guestControl()
            mode = LiveMode.Joining
            // Only an accepted invitation clears an older system pause.
            player.clearSystemPause()
            val s = newSession()
            val f = GuestFollower(g, player, s, ::resolve, { extraLatencyMs.toLong() }, SystemClock::uptimeMillis) { Log.i(TAG, it) }
            follower = f
            player.liveFollowing = true
            player.liveTarget = f::hostPosition
            every(250) { if (mode == LiveMode.Joined) f.tick() }
        } catch (_: FfiException) {
            mode = LiveMode.Ended("This invitation isn't valid.")
        }
    }

    fun leave() {
        host?.end()
        guest?.leave()
        finish(LiveMode.Idle)
    }

    fun dismissEnded() {
        if (mode is LiveMode.Ended) mode = LiveMode.Idle
    }

    private fun finish(m: LiveMode) {
        generation++
        session?.cancel()
        session = null
        host?.setControlObserver(null)
        host?.setPlayNextObserver(null)
        guest?.setControlObserver(null)
        host?.close()
        guest?.close()
        host = null
        guest = null
        publisher = null
        follower = null
        guestControl = false
        outbox.clear()
        outboxJob = null
        hostEnqueue.clear()
        hostEnqueueJob = null
        player.liveEnded()
        invite = null
        guests = emptyList()
        upNext = emptyList()
        reconnecting = false
        mode = m
    }

    private fun newSession() = CoroutineScope(SupervisorJob() + Dispatchers.Main).also { session = it }

    private fun every(ms: Long, tick: () -> Unit) {
        (session ?: newSession()).launch {
            while (isActive) {
                tick()
                delay(ms)
            }
        }
    }

    private fun handle(u: LiveUpdate, gen: Int) {
        if (gen != generation || host == null && guest == null) return
        when (u) {
            is LiveUpdate.Guests -> guests = u.guests
            is LiveUpdate.Joined -> {
                hostName = u.hostName
                reconnecting = false
                refreshUpNext()
            }
            LiveUpdate.QueueChanged -> refreshUpNext()
            LiveUpdate.PlaybackChanged -> reconnecting = false
            LiveUpdate.Reconnecting -> reconnecting = true
            is LiveUpdate.Ended -> return finish(mode.after(u))
        }
        mode = mode.after(u)
    }

    // Guest playback control

    /** Host: lets guests play/pause, skip and add songs. */
    fun allowGuestControl(allowed: Boolean) {
        val h = host ?: return
        h.setGuestControl(allowed)
        guestControl = h.guestControl()
        if (!guestControl) dropGuestEnqueues()
    }

    private fun controlChanged(allowed: Boolean, gen: Int) {
        if (gen != generation || guest == null) return
        guestControl = allowed
        if (!allowed) outbox.clear()
    }

    /** Host: applies an accepted guest command through the normal player path; the host tick publishes the result. */
    private fun guestCommand(cmd: LiveCommand, gen: Int) {
        if (gen != generation || host?.guestControl() != true) return
        when (cmd) {
            LiveCommand.Play -> if (!player.playing) player.toggle()
            LiveCommand.Pause -> if (player.playing) player.toggle()
            LiveCommand.Next -> player.next()
            is LiveCommand.Seek -> player.seek(maxOf(cmd.positionMs, 0) / 1000.0)
            is LiveCommand.Enqueue -> {
                cmd.trackIds.forEach { hostEnqueue.addLast(it to false) }
                pumpHostEnqueue()
            }
        }
    }

    /** Host: a guest's Play Next (accepted only while control is on). */
    private fun guestPlayNext(ids: List<String>, gen: Int) {
        if (gen != generation || host?.guestControl() != true) return
        ids.forEach { hostEnqueue.addLast(it to true) }
        pumpHostEnqueue()
    }

    private fun dropGuestEnqueues() {
        hostEnqueue.clear()
        hostEnqueueJob?.cancel()
        hostEnqueueJob = null
    }

    private fun pumpHostEnqueue() {
        if (hostEnqueueJob != null) return
        val s = session ?: return
        hostEnqueueJob = s.launch {
            while (hostEnqueue.isNotEmpty()) {
                val (id, next) = hostEnqueue.removeFirst()
                val track = resolve(id) ?: continue
                val h = host?.takeIf { it.guestControl() } ?: continue
                if (next) {
                    // Publish before asking (playback may have moved on) and after inserting, so the core's Play Next
                    // run matches the queue.
                    hostTick()
                    player.insertUpNext(track, h.playNextOffset(listOf(id)).toInt())
                    hostTick()
                } else {
                    player.addToQueue(listOf(track))
                }
            }
            hostEnqueueJob = null
        }
    }

    /** True when Listen Together handled the action (sent to the host or refused); false: act locally. */
    private fun route(cmd: LiveCommand): Boolean {
        val g = guest ?: return false
        if (!guestControl) return refuse()
        if (!g.sendCommand(cmd)) Chrome.flash("Couldn't reach the host")
        return true
    }

    private fun refuse(): Boolean {
        Chrome.flash("The host controls playback")
        return true
    }

    private fun hostPlaying() = guest?.target(0)?.playing ?: false

    /** Play/pause from the UI or the media notification. */
    fun userToggle() {
        val hostPlaying = hostPlaying()
        if (hostPlaying && resumeLocally()) return
        val handled = if (hostPlaying) route(LiveCommand.Pause) else requestPlay()
        if (!handled) player.toggle()
    }

    fun userPlay() {
        if (hostPlaying() && resumeLocally()) return
        if (!requestPlay() && !player.playing) player.toggle()
    }

    fun userPause() {
        if (!route(LiveCommand.Pause) && player.playing) player.toggle()
    }

    fun userNext() {
        if (!route(LiveCommand.Next)) player.next()
    }

    /** An explicit play also lifts a local system pause, otherwise the guest would stay silent once the host resumes. */
    private fun requestPlay(): Boolean {
        if (!route(LiveCommand.Play)) return false
        if (guestControl && player.systemPaused) player.clearSystemPause()
        return true
    }

    /** Guest paused by the system while the host plays: "play" means resume here, not change the host. */
    private fun resumeLocally(): Boolean {
        if (guest == null || !player.systemPaused) return false
        player.clearSystemPause()
        return true
    }

    /** Guests with control ask the host, which moves everyone at once; false: seek locally. */
    fun userSeek(secs: Double): Boolean {
        val g = guest
        if (g != null && guestControl && !g.hostSupportsSeek()) {
            Chrome.flash("The host's app is too old to seek for everyone")
            return true
        }
        return route(LiveCommand.Seek((maxOf(secs, 0.0) * 1000).toLong()))
    }

    /** Previous track: guests always follow the host. */
    fun guestRefuses() = guest != null && refuse()

    /** "Add to Queue": guests ask the host to append the songs; false: queue locally. */
    fun userEnqueue(tracks: List<Track>) = send(tracks, next = false, "Sent to the host's queue")

    /** "Play Next": guests ask the host; the host inserts after earlier Play Next songs. False: queue locally. */
    fun userPlayNext(tracks: List<Track>): Boolean {
        val h = host
        if (h != null) {
            // The host's own Play Next joins the same run as the guests', so neither jumps ahead of the other.
            for (t in tracks) {
                hostTick()
                player.insertUpNext(t, h.playNextOffset(listOf(t.id)).toInt())
            }
            hostTick()
            Chrome.flash("Playing next")
            return true
        }
        val supported = guest?.hostSupportsPlayNext() ?: false
        val flash = if (supported) "Sent to play next on the host" else "The host's app is too old for Play Next, added to the end of the queue"
        return send(tracks, supported, flash)
    }

    private fun send(tracks: List<Track>, next: Boolean, flash: String): Boolean {
        if (guest == null) return false
        if (!guestControl) return refuse()
        tracks.map { it.id }.chunked(MAX_ENQUEUE).forEach { outbox.addLast(it to next) }
        pumpOutbox()
        Chrome.flash(flash)
        return true
    }

    /** Sends queue requests in order; when the command budget refuses one it retries for about 10 s. */
    private fun pumpOutbox() {
        if (outboxJob != null) return
        val s = session ?: return
        outboxJob = s.launch {
            var failures = 0
            while (true) {
                val (ids, next) = outbox.firstOrNull() ?: break
                val g = guest?.takeIf { guestControl } ?: break
                val accepted = if (next) g.sendPlayNext(ids) else g.sendCommand(LiveCommand.Enqueue(ids))
                if (accepted) {
                    outbox.removeFirst()
                    failures = 0
                    continue
                }
                if (++failures >= OUTBOX_RETRIES) {
                    outbox.clear()
                    Chrome.flash("Couldn't reach the host")
                    break
                }
                delay(500)
            }
            outboxJob = null
        }
    }

    // Host

    private fun hostTick() {
        val p = publisher ?: return
        val current = player.track
        val queue = listOfNotNull(current) + player.userQueue + player.upNext
        p.publish(current?.id, queue.map { it.id }, player.livePublishedPlaying, player.livePublishedPositionMs)
    }

    // Guest

    private suspend fun resolve(id: String): Track? {
        cache[id]?.let { return it }
        val t = try {
            liveTrack(id)
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            return null
        }
        cache[id] = t
        return t
    }

    private fun refreshUpNext() {
        val g = guest ?: return
        // Entries after the one playing (by item id, so repeated tracks stay distinct).
        val entries = g.queue()
        val playing = g.target(0)?.itemId
        val start = entries.indexOfFirst { it.itemId == playing }.let { if (it < 0) 1 else it + 1 }
        val next = entries.drop(start).take(30)
        val refresh = ++upNextRefresh
        session?.launch {
            val rows = next.mapNotNull { e -> resolve(e.trackId)?.let { LiveRow(e.itemId, it) } }
            if (refresh == upNextRefresh) upNext = rows
        }
    }

    /** Core callbacks arrive on core threads; hop to the main thread. */
    private class Bridge(private val gen: Int) : LiveObserver {
        override fun onUpdate(update: LiveUpdate) {
            scope.launch { handle(update, gen) }
        }
    }

    private class ControlBridge(private val gen: Int) : LiveControlObserver, LivePlayNextObserver {
        override fun onControlChanged(allowed: Boolean) {
            scope.launch { controlChanged(allowed, gen) }
        }

        override fun onGuestCommand(peer: String, name: String, command: LiveCommand) {
            scope.launch { guestCommand(command, gen) }
        }

        override fun onGuestPlayNext(peer: String, name: String, trackIds: List<String>) {
            scope.launch { guestPlayNext(trackIds, gen) }
        }
    }

    private const val TAG = "live"
    private const val MAX_ENQUEUE = 50
    private const val OUTBOX_RETRIES = 20
}
