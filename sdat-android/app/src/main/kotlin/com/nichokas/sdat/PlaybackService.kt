package com.nichokas.sdat

import android.app.PendingIntent
import android.content.Intent
import android.os.Bundle
import android.os.SystemClock
import androidx.annotation.OptIn
import androidx.compose.runtime.snapshotFlow
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.CommandButton
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionCommands
import androidx.media3.session.SessionError
import androidx.media3.session.SessionResult
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.Playback
import java.util.UUID
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

/** Keeps playback alive in the background and gives the system the notification, lock screen and headset controls
 * (iOS MPRemoteCommandCenter / MPNowPlayingInfoCenter), with the favourite button. The player itself stays app-wide
 * in [SdatApp]. */
// Media button preferences, how a session adds its own buttons, are still marked unstable.
@OptIn(UnstableApi::class)
class PlaybackService : MediaSessionService() {
    private var session: MediaSession? = null
    private val scope = MainScope()

    override fun onCreate() {
        super.onCreate()
        val playback = (application as SdatApp).playback
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
        val favourites = NotificationFavouriteActions()
        val session = MediaSession.Builder(this, playback.sessionPlayer).setSessionActivity(open)
            .setCallback(Callback(playback, favourites)).build()
        this.session = session
        // A restored queue is idle until played; by default an idle player gets no notification, and with it no lock
        // screen controls, before its first prepare.
        setShowNotificationForIdlePlayer(SHOW_NOTIFICATION_FOR_IDLE_PLAYER_ALWAYS)
        // iOS syncRemote: the button follows the playing track and its favourite state.
        scope.launch {
            snapshotFlow { playback.track?.id?.let { it to Favs.known(it) } }.collect { now ->
                val button = favourites.update(now?.first, now?.second == true)
                session.mediaNotificationControllerInfo?.let { controller ->
                    val defaults = MediaSession.ConnectionResult.AcceptedResultBuilder(session, controller).build()
                    session.setAvailableCommands(
                        controller, favourites.availableCommands(defaults.availableSessionCommands), defaults.availablePlayerCommands,
                    )
                }
                session.setMediaButtonPreferences(listOfNotNull(button))
                if (now != null && now.second == null) launch { Favs.check(now.first) }
            }
        }
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo) = session

    override fun onDestroy() {
        scope.cancel()
        // Releases only the session; the player outlives the service.
        session?.release()
        session = null
        super.onDestroy()
    }

    private class Callback(
        private val playback: Playback,
        private val favourites: NotificationFavouriteActions,
    ) : MediaSession.Callback {
        override fun onConnectAsync(session: MediaSession, controller: MediaSession.ControllerInfo): ListenableFuture<MediaSession.ConnectionResult> {
            val defaults = MediaSession.ConnectionResult.AcceptedResultBuilder(session, controller).build()
            if (!session.mayFavourite(controller)) return Futures.immediateFuture(defaults)
            val commands = favourites.availableCommands(defaults.availableSessionCommands)
            return Futures.immediateFuture(MediaSession.ConnectionResult.AcceptedResultBuilder(session, controller).setAvailableSessionCommands(commands).build())
        }

        override fun onCustomCommand(
            session: MediaSession,
            controller: MediaSession.ControllerInfo,
            customCommand: SessionCommand,
            args: Bundle,
        ): ListenableFuture<SessionResult> {
            val (id, on) = favourites.target(customCommand)
                ?: return super.onCustomCommand(session, controller, customCommand, args)
            if (!session.mayFavourite(controller)) return Futures.immediateFuture(SessionResult(SessionError.ERROR_PERMISSION_DENIED))
            if (playback.track != null) Favs.set(id, on)
            return Futures.immediateFuture(SessionResult(SessionResult.RESULT_SUCCESS))
        }
    }
}

// The service is exported and a favourite changes the user's account, so only our notification gets the button.
// Media3 hands System UI's (notification, lock screen) commands to the callback as the notification controller.
@OptIn(UnstableApi::class)
private fun MediaSession.mayFavourite(controller: MediaSession.ControllerInfo) = isMediaNotificationController(controller)

@OptIn(UnstableApi::class)
internal class NotificationFavouriteActions(private val clock: () -> Long = SystemClock::elapsedRealtime) {
    private var current: TrackActions? = null
    private val retired = ArrayDeque<RetiredActions>()

    fun update(id: String?, fav: Boolean): CommandButton? {
        if (id == null) {
            current = null
            retired.clear()
            return null
        }
        val actions = current?.takeIf { it.id == id } ?: TrackActions(id).also {
            // System UI can send a button from before several rapid track changes.
            current?.let { retired.addLast(RetiredActions(it, clock() + FAVOURITE_GRACE_MS)) }
            current = it
        }
        return CommandButton.Builder(if (fav) CommandButton.ICON_STAR_FILLED else CommandButton.ICON_STAR_UNFILLED)
            .setDisplayName(if (fav) "Remove from Favourites" else "Add to Favourites")
            .setSessionCommand(if (fav) actions.remove else actions.add)
            .build()
    }

    fun availableCommands(defaults: SessionCommands): SessionCommands = defaults.buildUpon().apply {
        for (actions in recentActions()) add(actions.add).add(actions.remove)
    }.build()

    fun target(command: SessionCommand): Pair<String, Boolean>? {
        for (actions in recentActions()) {
            when (command) {
                actions.add -> return actions.id to true
                actions.remove -> return actions.id to false
            }
        }
        return null
    }

    private fun recentActions(): List<TrackActions> {
        val now = clock()
        while (retired.firstOrNull()?.expiresAt?.let { it <= now } == true) retired.removeFirst()
        return listOfNotNull(current) + retired.map { it.actions }
    }

    private class RetiredActions(val actions: TrackActions, val expiresAt: Long)

    private class TrackActions(val id: String) {
        // Media3 ignores extras when comparing buttons, so the action itself has to change with the track.
        private val token = UUID.randomUUID()
        val add = SessionCommand("com.nichokas.sdat.FAVOURITE_ADD.$token", Bundle.EMPTY)
        val remove = SessionCommand("com.nichokas.sdat.FAVOURITE_REMOVE.$token", Bundle.EMPTY)
    }

    private companion object {
        const val FAVOURITE_GRACE_MS = 5_000L
    }
}
