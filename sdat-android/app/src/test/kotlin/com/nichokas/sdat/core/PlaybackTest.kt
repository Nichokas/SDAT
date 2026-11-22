package com.nichokas.sdat.core

import androidx.media3.common.Player
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancel
import kotlinx.coroutines.job
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.tonearm_ffi.Quality
import uniffi.tonearm_ffi.Stream

class PlaybackTest {
    @Test fun retriesAfterFailureBeforeAnythingElse() {
        assertEquals(Tap.Retry, tapAction(unprepared = true, ended = true, playWhenReady = true))
    }

    @Test fun replaysFromTheStartOnceEnded() {
        assertEquals(Tap.Replay, tapAction(unprepared = false, ended = true, playWhenReady = false))
    }

    @Test fun pausesWhileBufferingAndResumesWhenPaused() {
        assertEquals(Tap.Pause, tapAction(unprepared = false, ended = false, playWhenReady = true))
        assertEquals(Tap.Resume, tapAction(unprepared = false, ended = false, playWhenReady = false))
    }

    @Test fun refetchesTheFirstFailureOfEachTrack() {
        assertTrue(refetchDue(null, "a", now = 5_000))
        assertTrue(refetchDue("b" to 5_000L, "a", now = 5_000))
    }

    @Test fun showsAFailureRightAfterARefetchOfTheSameTrack() {
        assertFalse(refetchDue("a" to 5_000L, "a", now = 5_000 + RETRY_WINDOW_MS - 1))
    }

    @Test fun refetchesTheSameTrackAgainOnceTheWindowPassed() {
        assertTrue(refetchDue("a" to 5_000L, "a", now = 5_000 + RETRY_WINDOW_MS))
    }

    @Test fun progressIsPositionOverDurationCappedAtOne() {
        assertEquals(0.25, progress(30.0, 120.0), 1e-9)
        assertEquals(1.0, progress(130.0, 120.0), 1e-9)
        assertEquals(0.0, progress(30.0, 0.0), 1e-9)
    }

    @Test fun seekStaysInsideTheTrack() {
        assertEquals(0.0, clampSeek(-4.0, 120.0), 1e-9)
        assertEquals(120.0, clampSeek(130.0, 120.0), 1e-9)
        assertEquals(42.0, clampSeek(42.0, 120.0), 1e-9)
        assertEquals(42.0, clampSeek(42.0, 0.0), 1e-9)
    }

    @Test fun controllerSeeksToAnotherItemGoThroughTheQueue() {
        assertEquals(Seek.Next, seekAction(Player.COMMAND_SEEK_TO_NEXT, 0, 0))
        assertEquals(Seek.Previous, seekAction(Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM, 0, 0))
        assertEquals(Seek.InCurrent, seekAction(Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM, 0, 0))
        assertEquals(Seek.InCurrent, seekAction(Player.COMMAND_SEEK_TO_DEFAULT_POSITION, 0, 0))
        assertEquals(Seek.Next, seekAction(Player.COMMAND_SEEK_TO_MEDIA_ITEM, 1, 0))
        assertEquals(Seek.Ignore, seekAction(Player.COMMAND_SEEK_TO_MEDIA_ITEM, 3, 0))
    }

    @Test fun badgeShowsTheDeliveredQuality() {
        fun stream(asked: Quality, vararg formats: String) = Stream("u", asked, formats.toList(), null, null)
        assertEquals(Quality.HI_FI, stream(Quality.CD, "FLAC", "FLAC_HIRES").actualQuality)
        assertEquals(Quality.CD, stream(Quality.HI_FI, "FLAC").actualQuality)
        assertEquals(Quality.AAC, stream(Quality.HI_FI, "AACLC").actualQuality)
        assertEquals(Quality.CD, stream(Quality.CD).actualQuality)
        assertEquals("Hi-Res Lossless", Quality.HI_FI.badgeText)
    }

    @Test fun sessionCommandsStripUnguardedExoPlayerMutations() {
        val blocked = setOf(
            Player.COMMAND_SET_REPEAT_MODE,
            Player.COMMAND_SET_SHUFFLE_MODE,
            Player.COMMAND_SET_SPEED_AND_PITCH,
            Player.COMMAND_SET_MEDIA_ITEM,
            Player.COMMAND_CHANGE_MEDIA_ITEMS,
            Player.COMMAND_RELEASE,
        )
        val readOnly = setOf(
            Player.COMMAND_GET_CURRENT_MEDIA_ITEM,
            Player.COMMAND_GET_TIMELINE,
            Player.COMMAND_GET_METADATA,
            Player.COMMAND_GET_AUDIO_ATTRIBUTES,
            Player.COMMAND_GET_VOLUME,
            Player.COMMAND_GET_TEXT,
            Player.COMMAND_GET_TRACKS,
        )
        val playback = setOf(
            Player.COMMAND_PLAY_PAUSE,
            Player.COMMAND_PREPARE,
            Player.COMMAND_STOP,
        )
        val keptWithTrack = readOnly + playback
        val base = keptWithTrack + blocked

        val loaded = sessionCommands(base, hasTrack = true, emptyTimeline = false)
        assertTrue(loaded.containsAll(keptWithTrack))
        assertTrue(Player.COMMAND_SEEK_TO_NEXT in loaded)
        assertTrue(Player.COMMAND_SEEK_TO_PREVIOUS in loaded)
        assertFalse(Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM in loaded)
        blocked.forEach { assertFalse(it in loaded) }

        val restored = sessionCommands(base, hasTrack = true, emptyTimeline = true)
        assertTrue(restored.containsAll(keptWithTrack))
        assertTrue(Player.COMMAND_SEEK_TO_NEXT in restored)
        assertTrue(Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM in restored)
        blocked.forEach { assertFalse(it in restored) }

        val emptyBase = base + setOf(
            Player.COMMAND_SEEK_TO_DEFAULT_POSITION,
            Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM,
            Player.COMMAND_SEEK_TO_NEXT,
            Player.COMMAND_SEEK_TO_PREVIOUS,
            Player.COMMAND_SEEK_BACK,
            Player.COMMAND_SEEK_FORWARD,
            Player.COMMAND_SET_VOLUME,
            Player.COMMAND_SET_PLAYLIST_METADATA,
            Player.COMMAND_SET_AUDIO_ATTRIBUTES,
            Player.COMMAND_SET_TRACK_SELECTION_PARAMETERS,
            Player.COMMAND_SET_VIDEO_SURFACE,
        )
        assertEquals(readOnly, sessionCommands(emptyBase, hasTrack = false, emptyTimeline = true))
        assertEquals(readOnly, sessionCommands(emptyBase, hasTrack = false, emptyTimeline = false))
    }

    @Test fun nonDiscoverHandoffWaitsForUpcomingUnderRepeatOffAndRepeatAll() {
        assertEquals(
            Advance.Wait,
            advanceAction(auto = true, repeat = Repeat.Off, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = false, repeat = Repeat.Off, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = true, repeat = Repeat.All, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = false, repeat = Repeat.All, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Queue,
            advanceAction(auto = true, repeat = Repeat.One, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = false, repeat = Repeat.One, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
        assertEquals(
            Advance.Queue,
            advanceAction(auto = true, repeat = Repeat.All, hasNext = true, hasInfinite = false, awaitingMixes = false, awaitingUpNext = true),
        )
    }

    @Test fun advanceStopsWrapsOrRefillsOnceUpcomingIsSettled() {
        assertEquals(
            Advance.Stop,
            advanceAction(auto = true, repeat = Repeat.Off, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = false),
        )
        assertEquals(
            Advance.Stop,
            advanceAction(auto = false, repeat = Repeat.Off, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = false),
        )
        assertEquals(
            Advance.Queue,
            advanceAction(auto = true, repeat = Repeat.All, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = false),
        )
        assertEquals(
            Advance.Queue,
            advanceAction(auto = false, repeat = Repeat.All, hasNext = false, hasInfinite = false, awaitingMixes = false, awaitingUpNext = false),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = true, repeat = Repeat.Off, hasNext = false, hasInfinite = true, awaitingMixes = false, awaitingUpNext = false),
        )
        assertEquals(
            Advance.Wait,
            advanceAction(auto = false, repeat = Repeat.Off, hasNext = false, hasInfinite = false, awaitingMixes = true, awaitingUpNext = false),
        )
    }

    @Test fun postWaitRestartsPausedOnEmptyManualSkipAndAdvancesWhenQueuedOrRepeating() {
        assertEquals(PostWait.RestartPaused, postWaitAction(auto = false, hasNext = false, repeat = Repeat.Off, timedOut = false))
        assertEquals(PostWait.RestartPaused, postWaitAction(auto = false, hasNext = false, repeat = Repeat.One, timedOut = false))
        assertEquals(PostWait.Advance, postWaitAction(auto = false, hasNext = false, repeat = Repeat.All, timedOut = false))
        assertEquals(PostWait.Advance, postWaitAction(auto = false, hasNext = true, repeat = Repeat.Off, timedOut = false))
        assertEquals(PostWait.Advance, postWaitAction(auto = false, hasNext = true, repeat = Repeat.One, timedOut = false))
        assertEquals(PostWait.StopAtEnd, postWaitAction(auto = true, hasNext = false, repeat = Repeat.Off, timedOut = false))
        assertEquals(PostWait.Advance, postWaitAction(auto = true, hasNext = false, repeat = Repeat.All, timedOut = false))
        assertEquals(PostWait.Advance, postWaitAction(auto = true, hasNext = false, repeat = Repeat.One, timedOut = false))
        assertEquals(PostWait.Advance, postWaitAction(auto = true, hasNext = true, repeat = Repeat.Off, timedOut = false))
    }

    @Test fun postWaitDoesNotLoopRepeatAllAfterHandoffTimeout() {
        assertEquals(PostWait.StopAtEnd, postWaitAction(auto = true, hasNext = false, repeat = Repeat.All, timedOut = true))
        assertEquals(PostWait.RestartPaused, postWaitAction(auto = false, hasNext = false, repeat = Repeat.All, timedOut = true))
        assertEquals(PostWait.Advance, postWaitAction(auto = true, hasNext = true, repeat = Repeat.All, timedOut = true))
        assertEquals(PostWait.Advance, postWaitAction(auto = false, hasNext = true, repeat = Repeat.All, timedOut = true))
        assertEquals(PostWait.Advance, postWaitAction(auto = true, hasNext = false, repeat = Repeat.One, timedOut = true))
        assertEquals(PostWait.RestartPaused, postWaitAction(auto = false, hasNext = false, repeat = Repeat.One, timedOut = true))
    }

    @Test fun followUpSkipKeepsActiveWaitAndUpgradesAutoToManual() = runBlocking {
        val wait = AdvanceWait()
        val gate = CompletableDeferred<Unit>()
        var launches = 0
        var cancelled = false
        var observedAuto: Boolean? = null

        wait.launchOrJoin(this, auto = true) {
            launches++
            try {
                gate.await()
                observedAuto = wait.auto
                wait.clear(coroutineContext.job)
            } catch (e: kotlinx.coroutines.CancellationException) {
                cancelled = true
                throw e
            }
        }
        yield()
        assertTrue(wait.active)
        assertTrue(wait.auto)

        wait.launchOrJoin(this, auto = false) {
            launches++
        }
        yield()
        assertEquals(1, launches)
        assertFalse(cancelled)
        assertTrue(wait.active)
        assertFalse(wait.auto)

        gate.complete(Unit)
        yield()
        assertFalse(wait.active)
        assertEquals(false, observedAuto)

        wait.launchOrJoin(this, auto = true) {
            launches++
        }
        yield()
        assertEquals(2, launches)
        assertTrue(wait.auto)
        assertFalse(wait.active)
    }

    @Test fun cancelledWaitDoesNotClearAReplacementWait() = runBlocking {
        val wait = AdvanceWait()
        val firstGate = CompletableDeferred<Unit>()
        val secondGate = CompletableDeferred<Unit>()

        wait.launchOrJoin(this, auto = false) {
            firstGate.await()
        }
        yield()
        assertTrue(wait.active)

        wait.cancel()
        assertFalse(wait.active)

        wait.launchOrJoin(this, auto = true) {
            secondGate.await()
        }
        yield()
        assertTrue(wait.active)
        assertTrue(wait.auto)

        secondGate.complete(Unit)
        yield()
        assertFalse(wait.active)
    }

    @Test fun abandonedWaitClearsActiveStateWithoutManualClear() = runBlocking {
        val wait = AdvanceWait()
        val gate = CompletableDeferred<Unit>()
        var abandoned = false

        wait.launchOrJoin(this, auto = true) {
            gate.await()
            abandoned = true
            // Early return without calling wait.clear (e.g. track/entry changed or playback no longer STATE_ENDED).
            return@launchOrJoin
        }
        assertTrue(wait.active)
        yield()
        assertTrue(wait.active)

        gate.complete(Unit)
        yield()
        assertTrue(abandoned)
        assertFalse(wait.active)
    }

    @Test fun reentrantLaunchAfterClearKeepsReplacementWaitActive() = runBlocking {
        val wait = AdvanceWait()
        val firstGate = CompletableDeferred<Unit>()
        val secondGate = CompletableDeferred<Unit>()

        wait.launchOrJoin(this, auto = true) {
            firstGate.await()
            wait.clear(coroutineContext.job)
            assertFalse(wait.active)
            wait.launchOrJoin(this@runBlocking, auto = false) {
                secondGate.await()
            }
            assertTrue(wait.active)
        }
        yield()
        assertTrue(wait.active)

        firstGate.complete(Unit)
        yield()
        assertTrue(wait.active)
        assertFalse(wait.auto)

        secondGate.complete(Unit)
        yield()
        assertFalse(wait.active)
    }

    @Test fun launchOnCancelledScopeLeavesWaitInactive() {
        val wait = AdvanceWait()
        val scope = CoroutineScope(Job())
        scope.cancel()

        wait.launchOrJoin(scope, auto = true) {}
        assertFalse(wait.active)
    }
}
