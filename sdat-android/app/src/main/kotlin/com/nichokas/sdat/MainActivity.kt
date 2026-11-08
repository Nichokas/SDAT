package com.nichokas.sdat

import android.content.ComponentName
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import com.google.common.util.concurrent.ListenableFuture
import com.nichokas.sdat.ui.RootView
import com.nichokas.sdat.ui.inter
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.glasmioTypography
import io.github.neoruaa.glasmio.icons.GlasmioIconProvider

class MainActivity : ComponentActivity() {
    private var controller: ListenableFuture<MediaController>? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        controller = MediaController.Builder(this, SessionToken(this, ComponentName(this, PlaybackService::class.java))).buildAsync()
        setContent {
            GlasmioTheme(typography = glasmioTypography(inter)) {
                GlasmioIconProvider { (application as SdatApp).let { RootView(it.session, it.playback, it.prefs, it.secrets) } }
            }
        }
    }

    // A connected controller is what creates PlaybackService, and the service goes foreground only once a stream
    // buffers. Held until destroy, not stop, so a first track still fetching when the app is backgrounded keeps its
    // service; once playing, the service keeps itself in the foreground.
    override fun onDestroy() {
        controller?.let(MediaController::releaseFuture)
        controller = null
        super.onDestroy()
    }
}
