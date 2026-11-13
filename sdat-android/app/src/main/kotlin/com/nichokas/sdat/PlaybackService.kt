package com.nichokas.sdat

import android.app.PendingIntent
import android.content.Intent
import android.os.Bundle
import androidx.annotation.OptIn
import androidx.compose.runtime.snapshotFlow
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.CommandButton
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionError
import androidx.media3.session.SessionResult
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.nichokas.sdat.core.Favs
import com.nichokas.sdat.core.Playback
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
        val session = MediaSession.Builder(this, playback.sessionPlayer).setSessionActivity(open).setCallback(Callback(playback)).build()
        this.session = session
        // iOS syncRemote: the button follows the playing track and its favourite state.
        scope.launch {
            snapshotFlow { playback.track?.id?.let { it to Favs.known(it) } }.collect { now ->
                if (now == null) return@collect session.setMediaButtonPreferences(emptyList())
                val (id, fav) = now
                session.setMediaButtonPreferences(listOf(favouriteButton(fav == true)))
                if (fav == null) launch { Favs.check(id) }
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

    private class Callback(private val playback: Playback) : MediaSession.Callback {
        override fun onConnectAsync(session: MediaSession, controller: MediaSession.ControllerInfo): ListenableFuture<MediaSession.ConnectionResult> {
            val defaults = MediaSession.ConnectionResult.AcceptedResultBuilder(session, controller).build()
            if (!session.mayFavourite(controller)) return Futures.immediateFuture(defaults)
            val commands = defaults.availableSessionCommands.buildUpon().add(favourite).build()
            return Futures.immediateFuture(MediaSession.ConnectionResult.AcceptedResultBuilder(session, controller).setAvailableSessionCommands(commands).build())
        }

        override fun onCustomCommand(
            session: MediaSession,
            controller: MediaSession.ControllerInfo,
            customCommand: SessionCommand,
            args: Bundle,
        ): ListenableFuture<SessionResult> {
            if (customCommand.customAction != favourite.customAction) return super.onCustomCommand(session, controller, customCommand, args)
            if (!session.mayFavourite(controller)) return Futures.immediateFuture(SessionResult(SessionError.ERROR_PERMISSION_DENIED))
            // Sets what the button showed, like the player sheet's star.
            playback.track?.let { Favs.set(it.id, Favs.known(it.id) != true) }
            return Futures.immediateFuture(SessionResult(SessionResult.RESULT_SUCCESS))
        }
    }
}

// The service is exported and a favourite changes the user's account, so only our notification gets the button.
// Media3 hands System UI's (notification, lock screen) commands to the callback as the notification controller.
@OptIn(UnstableApi::class)
private fun MediaSession.mayFavourite(controller: MediaSession.ControllerInfo) = isMediaNotificationController(controller)

private val favourite = SessionCommand("com.nichokas.sdat.FAVOURITE", Bundle.EMPTY)

private fun favouriteButton(fav: Boolean) = CommandButton.Builder(if (fav) CommandButton.ICON_STAR_FILLED else CommandButton.ICON_STAR_UNFILLED)
    .setDisplayName(if (fav) "Remove from Favourites" else "Add to Favourites")
    .setSessionCommand(favourite)
    .build()
