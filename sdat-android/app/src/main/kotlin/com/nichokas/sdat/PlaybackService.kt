package com.nichokas.sdat

import android.app.PendingIntent
import android.content.Intent
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService

/** Keeps playback alive in the background and gives the system the notification, lock screen and headset controls
 * (iOS MPRemoteCommandCenter / MPNowPlayingInfoCenter). The player itself stays app-wide in [SdatApp]. */
class PlaybackService : MediaSessionService() {
    private var session: MediaSession? = null

    override fun onCreate() {
        super.onCreate()
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
        session = MediaSession.Builder(this, (application as SdatApp).playback.sessionPlayer).setSessionActivity(open).build()
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo) = session

    override fun onDestroy() {
        // Releases only the session; the player outlives the service.
        session?.release()
        session = null
        super.onDestroy()
    }
}
