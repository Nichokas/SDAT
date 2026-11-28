package com.nichokas.sdat

import android.app.Application
import androidx.annotation.OptIn
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionCommands
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], application = Application::class)
@OptIn(UnstableApi::class)
class NotificationFavouriteActionsTest {
    @Test fun lateTapTargetsTheTrackAndStateAdvertisedByTheButton() {
        val actions = NotificationFavouriteActions()
        val addA = checkNotNull(actions.update("a", fav = false))
        val removeA = checkNotNull(actions.update("a", fav = true))
        val addB = checkNotNull(actions.update("b", fav = false))
        actions.update("c", fav = false)

        assertNotEquals(addA, addB)
        val commands = actions.availableCommands(SessionCommands.EMPTY)
        for ((button, expected) in listOf(addA to ("a" to true), removeA to ("a" to false), addB to ("b" to true))) {
            val received = SessionCommand.fromBundle(checkNotNull(button.sessionCommand).toBundle())
            assertTrue(commands.contains(received))
            assertEquals(expected, actions.target(received))
        }
    }

    @Test fun expiresRetiredCommandsAfterTheGracePeriodOrWhenTheQueueClears() {
        var now = 0L
        val actions = NotificationFavouriteActions { now }
        val a = checkNotNull(actions.update("a", fav = false)?.sessionCommand)
        actions.update("b", fav = false)
        now += 5_000
        val c = checkNotNull(actions.update("c", fav = false)?.sessionCommand)

        assertFalse(actions.availableCommands(SessionCommands.EMPTY).contains(a))
        assertNull(actions.target(a))
        assertNull(actions.update(null, fav = false))
        assertFalse(actions.availableCommands(SessionCommands.EMPTY).contains(c))
        assertNull(actions.target(c))
    }
}
