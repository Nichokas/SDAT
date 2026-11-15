package com.nichokas.sdat

import android.Manifest
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.snapshotFlow
import androidx.lifecycle.lifecycleScope
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import com.google.common.util.concurrent.ListenableFuture
import com.nichokas.sdat.core.Chrome
import com.nichokas.sdat.core.NavState
import com.nichokas.sdat.core.Downloads
import com.nichokas.sdat.core.Handoff
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.ui.LocalTrackPlayer
import com.nichokas.sdat.ui.RootView
import com.nichokas.sdat.ui.inter
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.glasmioTypography
import io.github.neoruaa.glasmio.icons.GlasmioIconProvider
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    private var controller: ListenableFuture<MediaController>? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        // Chrome outlives a recreated Activity but not process death; restoring before the first frame brings back pages
        // like Settings that finish work when the user returns from the browser.
        savedInstanceState?.getStringArrayList(NAV)?.let { Chrome.restore(NavState.restore(it)) }
        // Asked when the first download starts, so its progress notification can show.
        val notify = registerForActivityResult(ActivityResultContracts.RequestPermission()) {}
        lifecycleScope.launch {
            snapshotFlow { Downloads.queue.isActive }.first { it }
            if (checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
                notify.launch(Manifest.permission.POST_NOTIFICATIONS)
            }
        }
        if (savedInstanceState == null) openLink(intent)
        controller = MediaController.Builder(this, SessionToken(this, ComponentName(this, PlaybackService::class.java))).buildAsync()
        setContent {
            GlasmioTheme(typography = glasmioTypography(inter)) {
                GlasmioIconProvider {
                    val app = application as SdatApp
                    CompositionLocalProvider(LocalTrackPlayer provides app.playback) { RootView(app.session, app.playback, app.prefs, app.secrets) }
                }
            }
        }
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putStringArrayList(NAV, ArrayList(Chrome.nav.saved()))
    }

    override fun onStart() {
        super.onStart()
        Handoff.foreground = true
        Downloads.resumePending()
    }

    // A rotation stops and starts the Activity; the device stays attached through it.
    override fun onStop() {
        super.onStop()
        if (!isChangingConfigurations) Handoff.foreground = false
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        openLink(intent)
    }

    private fun openLink(intent: Intent) {
        // Reopened from Recents, the task replays the link that started it, which would leave the current session.
        if (intent.flags and Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY != 0) return
        val link = intent.dataString ?: return
        if ((application as SdatApp).session.signedIn) Live.open(link)
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

private const val NAV = "nav"
