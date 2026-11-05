package com.nichokas.sdat.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nichokas.sdat.core.Session
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.ui.GlasmioButton
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.DeviceLogin
import uniffi.tonearm_ffi.FfiException
import uniffi.tonearm_ffi.startDeviceLogin

/** TIDAL device linking: show the code, open link.tidal.com, wait for approval. */
@Composable
fun LoginView(session: Session) {
    val scope = rememberCoroutineScope()
    val uri = LocalUriHandler.current
    var login by remember { mutableStateOf<DeviceLogin?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var attempt by remember { mutableStateOf<Job?>(null) }
    val colors = GlasmioTheme.colors
    // Centered when it fits, scrollable in short (landscape, split-screen) windows.
    BoxWithConstraints(Modifier.fillMaxSize().safeDrawingPadding()) {
        Column(
            Modifier.verticalScroll(rememberScrollState()).fillMaxWidth().heightIn(min = maxHeight).padding(32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
        ) {
            Text("SDAT", style = IosTypography.largeTitle)
            val pending = login
            if (pending == null) {
                Text("Sign in with your TIDAL account.", style = IosTypography.body, color = colors.secondaryContent)
                GlasmioButton(onClick = {
                    if (busy) return@GlasmioButton
                    busy = true
                    error = null
                    attempt = scope.launch {
                        try {
                            startDeviceLogin().use { l ->
                                login = l
                                // No browser on this device: the code can still be approved from another one.
                                runCatching { uri.openUri(l.verificationUrl()) }
                                try {
                                    val tokens = l.wait()
                                    // Cancel may land after the native poll's last check; never store tokens it declined.
                                    ensureActive()
                                    session.signIn(tokens)
                                } catch (e: CancellationException) {
                                    // The native poll outlives the coroutine unless told to stop.
                                    l.cancel()
                                    throw e
                                }
                            }
                        } catch (e: CancellationException) {
                            throw e
                        } catch (_: FfiException.Cancelled) {
                        } catch (e: Exception) {
                            error = e.message ?: e.javaClass.simpleName
                        } finally {
                            login = null
                            busy = false
                        }
                    }
                }) { Text("Sign in with TIDAL") }
            } else {
                Text("Approve this code on link.tidal.com", style = IosTypography.body, color = colors.secondaryContent)
                Text(pending.userCode(), fontSize = 34.sp, fontWeight = FontWeight.Bold, letterSpacing = 4.sp)
                GlasmioButton(onClick = { runCatching { uri.openUri(pending.verificationUrl()) } }) { Text("Open link.tidal.com") }
                GlasmioButton(onClick = { pending.cancel(); attempt?.cancel() }) { Text("Cancel") }
            }
            error?.let { Text(it, style = IosTypography.footnote, color = colors.secondaryContent) }
        }
    }
}
