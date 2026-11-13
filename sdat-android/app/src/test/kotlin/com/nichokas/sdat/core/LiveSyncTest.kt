package com.nichokas.sdat.core

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.tonearm_ffi.AlbumRef
import uniffi.tonearm_ffi.FfiException
import uniffi.tonearm_ffi.LiveCommand
import uniffi.tonearm_ffi.LiveControlObserver
import uniffi.tonearm_ffi.LiveCorrection
import uniffi.tonearm_ffi.LiveDiagnostics
import uniffi.tonearm_ffi.LiveEnd
import uniffi.tonearm_ffi.LiveGuestInfo
import uniffi.tonearm_ffi.LiveGuestInterface
import uniffi.tonearm_ffi.LiveHostInterface
import uniffi.tonearm_ffi.LivePath
import uniffi.tonearm_ffi.LivePlayNextObserver
import uniffi.tonearm_ffi.LiveQueueEntry
import uniffi.tonearm_ffi.LiveTarget
import uniffi.tonearm_ffi.LiveUpdate
import uniffi.tonearm_ffi.Track

private fun track(id: String) = Track(id, "Song $id", "", 200u, false, true, emptyList(), AlbumRef("x", "Album", null, null))

private class FakeHost : LiveHostInterface {
    val queues = mutableListOf<List<String>>()
    val anchors = mutableListOf<Anchor>()
    var failQueue = false

    data class Anchor(val itemId: String?, val trackId: String?, val positionMs: Long, val playing: Boolean)

    override fun setQueue(trackIds: List<String>): List<String> {
        if (failQueue) throw FfiException.Network("down")
        queues += trackIds
        return trackIds.map { "item-$it" }
    }

    override fun setPlayback(itemId: String?, trackId: String?, positionMs: Long, playing: Boolean) {
        anchors += Anchor(itemId, trackId, positionMs, playing)
    }

    override fun end() = Unit
    override fun guestControl() = false
    override fun guests() = emptyList<LiveGuestInfo>()
    override fun inviteUrl() = "sdat://join/x"
    override fun playNextOffset(trackIds: List<String>) = 0u
    override fun setControlObserver(observer: LiveControlObserver?) = Unit
    override fun setGuestControl(allowed: Boolean) = Unit
    override fun setPlayNextObserver(observer: LivePlayNextObserver?) = Unit
    override fun setQueueEntries(entries: List<LiveQueueEntry>) = Unit
}

private class FakeGuest : LiveGuestInterface {
    var target: LiveTarget? = null
    var answer: LiveCorrection = LiveCorrection.None
    val targetLatencies = mutableListOf<Long>()
    val measured = mutableListOf<Pair<Long, Long>>()

    override fun target(latencyMs: Long): LiveTarget? {
        targetLatencies += latencyMs
        return target
    }

    override fun correction(latencyMs: Long, actualMs: Long): LiveCorrection {
        measured += latencyMs to actualMs
        return answer
    }

    override fun diagnostics() = LiveDiagnostics(LivePath.DIRECT, null, null, null)
    override fun guestControl() = false
    override fun hostName() = "Host"
    override fun hostSupportsPlayNext() = true
    override fun hostSupportsSeek() = true
    override fun leave() = Unit
    override fun queue() = emptyList<LiveQueueEntry>()
    override fun queueTrackIds() = emptyList<String>()
    override fun sendCommand(command: LiveCommand) = true
    override fun sendPlayNext(trackIds: List<String>) = true
    override fun setControlObserver(observer: LiveControlObserver?) = Unit
    override fun setSeekOnly(on: Boolean) = Unit
}

private class FakeFollower : LiveFollower {
    override var currentId: String? = "a"
    override var isPlaying = true
    override var loading = false
    override var liveHasItem = true
    override var liveSeeking = false
    override var liveAdvancing = true
    override var systemPaused = false
    override var liveSettledAt: Long? = null
    override var livePositionMs = 0L
    val calls = mutableListOf<String>()

    override fun livePlay(track: Track, atMs: Long, playing: Boolean) { calls += "play ${track.id} $atMs $playing" }
    override fun liveResume() { calls += "resume" }
    override fun liveSeek(ms: Long) { calls += "liveSeek $ms" }
    override fun pause() { calls += "pause" }
    override fun seekTo(ms: Long) { calls += "seekTo $ms" }
}

class HostPublisherTest {
    private val host = FakeHost()
    private var clock = 0L
    private val publisher = HostPublisher(host) { clock }

    private fun publish(atMs: Long, positionMs: Long, playing: Boolean = true, queue: List<String> = listOf("a", "b")) {
        clock = atMs
        publisher.publish(queue.firstOrNull(), queue, playing, positionMs)
    }

    @Test fun firstPublishSharesTheQueueAndAnchorsTheCurrentItem() {
        publish(0, 1_000)
        assertEquals(listOf(listOf("a", "b")), host.queues)
        assertEquals(listOf(FakeHost.Anchor("item-a", "a", 1_000, true)), host.anchors)
    }

    @Test fun steadyPlaybackSendsNoAnchorUntilTheFiveSecondRefresh() {
        publish(0, 0)
        publish(1_000, 1_000)
        publish(4_900, 4_950)
        assertEquals(1, host.anchors.size)
        publish(5_000, 5_000)
        assertEquals(2, host.anchors.size)
    }

    @Test fun aPauseOrAJumpIsSentAtOnce() {
        publish(0, 0)
        publish(500, 500, playing = false)
        publish(1_000, 30_000, playing = false)
        assertEquals(listOf(false, false), host.anchors.drop(1).map { it.playing })
        assertEquals(30_000L, host.anchors.last().positionMs)
    }

    @Test fun aQueueChangeIsSharedWithAFreshAnchor() {
        publish(0, 0)
        publish(1_000, 1_000, queue = listOf("b", "c"))
        assertEquals(listOf("b", "c"), host.queues.last())
        assertEquals(FakeHost.Anchor("item-b", "b", 1_000, true), host.anchors.last())
    }

    @Test fun nothingPlayingAnchorsNoItem() {
        publish(0, 0, playing = false, queue = emptyList())
        assertEquals(FakeHost.Anchor(null, null, 0, false), host.anchors.single())
    }

    @Test fun theSharedQueueIsCapped() {
        publish(0, 0, queue = (0 until 600).map { "t$it" })
        assertEquals(HostPublisher.MAX_SHARED, host.queues.single().size)
    }

    @Test fun aRefusedQueueIsSentAgainOnTheNextTick() {
        host.failQueue = true
        publish(0, 0)
        host.failQueue = false
        publish(500, 500)
        assertEquals(1, host.queues.size)
        assertEquals("item-a", host.anchors.last().itemId)
    }
}

class GuestFollowerTest {
    private val guest = FakeGuest()
    private val player = FakeFollower()
    private var clock = 100_000L
    private var latency = 0L
    private val follower = GuestFollower(guest, player, CoroutineScope(Dispatchers.Unconfined), { track(it) }, { latency }, { clock }) {}

    private fun host(trackId: String? = "a", positionMs: Long = 60_000, playing: Boolean = true) {
        guest.target = LiveTarget("item", trackId, positionMs, playing)
    }

    @Test fun aSeekFromTheCoreIsTheOnlyCorrection() {
        host()
        player.livePositionMs = 50_000
        guest.answer = LiveCorrection.Seek(60_000)
        follower.tick()
        assertEquals(listOf("liveSeek 60000"), player.calls)
    }

    @Test fun aRateAnswerIsIgnored() {
        host()
        guest.answer = LiveCorrection.Rate(1.05)
        follower.tick()
        assertEquals(emptyList<String>(), player.calls)
    }

    @Test fun onlyTheManualDelayIsUsedAsLatency() {
        latency = 120
        host()
        player.livePositionMs = 59_000
        follower.tick()
        assertEquals(listOf(120L to 59_000L), guest.measured)
        assertTrue(guest.targetLatencies.all { it == 120L })
    }

    @Test fun noMeasurementWhileThePositionIsUntrustworthy() {
        host()
        player.liveAdvancing = false
        follower.tick()
        player.liveAdvancing = true
        player.liveSettledAt = clock - 1_000
        follower.tick()
        player.liveSettledAt = null
        player.loading = true
        follower.tick()
        player.loading = false
        player.liveSeeking = true
        follower.tick()
        assertEquals(emptyList<Pair<Long, Long>>(), guest.measured)
        player.liveSeeking = false
        follower.tick()
        assertEquals(1, guest.measured.size)
    }

    @Test fun aHostPausePausesAtTheHostPosition() {
        host(positionMs = 42_000, playing = false)
        follower.tick()
        assertEquals(listOf("pause", "seekTo 42000"), player.calls)
    }

    @Test fun aPausedGuestMovesOnlyWhenWellAwayFromTheHost() {
        player.isPlaying = false
        player.livePositionMs = 41_500
        host(positionMs = 42_000, playing = false)
        follower.tick()
        assertEquals(emptyList<String>(), player.calls)
        player.livePositionMs = 40_000
        follower.tick()
        assertEquals(listOf("seekTo 42000"), player.calls)
    }

    @Test fun aHostResumeResumesHere() {
        player.isPlaying = false
        host()
        follower.tick()
        assertEquals(listOf("resume"), player.calls)
    }

    @Test fun aSystemPauseIsNotUndoneByTheHost() {
        player.isPlaying = false
        player.systemPaused = true
        host()
        follower.tick()
        assertEquals(emptyList<String>(), player.calls)
    }

    @Test fun aNewHostTrackStartsAtTheHostPosition() {
        host(trackId = "b", positionMs = 4_000)
        follower.tick()
        assertEquals(listOf("play b 4000 true"), player.calls)
    }

    @Test fun aNewHostTrackLoadsPausedWhileSystemPaused() {
        player.systemPaused = true
        host(trackId = "b", positionMs = 4_000)
        follower.tick()
        assertEquals(listOf("play b 4000 false"), player.calls)
    }

    @Test fun theSameTrackReloadsOnceOurCopyEnded() {
        player.liveHasItem = false
        host(positionMs = 0)
        follower.tick()
        assertEquals(listOf("play a 0 true"), player.calls)
    }

    @Test fun nothingPlayingAtTheHostPausesHere() {
        host(trackId = null)
        follower.tick()
        assertEquals(listOf("pause"), player.calls)
    }

    @Test fun aChangedManualDelayRealignsOnce() {
        host()
        player.livePositionMs = 60_000
        follower.tick()
        latency = 300
        follower.tick()
        assertEquals(listOf("liveSeek 60000"), player.calls)
        follower.tick()
        assertEquals(1, player.calls.size)
    }

    @Test fun hostPositionIsOnlyForTheTrackLoadedHere() {
        host(positionMs = 7_000)
        assertEquals(HostAt(7_000, true), follower.hostPosition())
        host(trackId = "b")
        assertNull(follower.hostPosition())
    }
}

class LiveModeTest {
    @Test fun joiningBecomesJoinedOnlyOnTheHostsWelcome() {
        assertEquals(LiveMode.Joined, LiveMode.Joining.after(LiveUpdate.Joined("Host")))
        assertEquals(LiveMode.Hosting, LiveMode.Hosting.after(LiveUpdate.Joined("Host")))
        assertEquals(LiveMode.Joining, LiveMode.Joining.after(LiveUpdate.Reconnecting))
    }

    @Test fun anEndExplainsWhy() {
        assertEquals(LiveMode.Ended("The host ended the session."), LiveMode.Joined.after(LiveUpdate.Ended(LiveEnd.HOST_ENDED)))
        assertEquals(LiveMode.Ended("This session is full."), LiveMode.Joining.after(LiveUpdate.Ended(LiveEnd.FULL)))
    }

    @Test fun onlyAnOpenSessionIsActive() {
        assertEquals(
            listOf(false, true, true, true, true, false),
            listOf(LiveMode.Idle, LiveMode.Starting, LiveMode.Hosting, LiveMode.Joining, LiveMode.Joined, LiveMode.Ended("x")).map { it.active },
        )
    }
}
