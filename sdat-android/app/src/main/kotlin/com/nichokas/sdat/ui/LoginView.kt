package com.nichokas.sdat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
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
    val colors = GlasmioTheme.colors
    Column(
        Modifier.fillMaxSize().background(colors.groupedBackground).safeDrawingPadding().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
    ) {
        Text("SDAT", style = IosTypography.largeTitle)
        val pending = login
        if (pending == null) {
            Text("Sign in with your TIDAL account.", style = IosTypography.body, color = colors.secondaryContent)
            GlasmioButton(onClick = {
                error = null
                scope.launch {
                    try {
                        startDeviceLogin().use { l ->
                            login = l
                            uri.openUri(l.verificationUrl())
                            session.signIn(l.wait())
                        }
                    } catch (e: CancellationException) {
                        throw e
                    } catch (_: FfiException.Cancelled) {
                    } catch (e: Exception) {
                        error = e.message ?: e.javaClass.simpleName
                    } finally {
                        login = null
                    }
                }
            }) { Text("Sign in with TIDAL") }
        } else {
            Text("Approve this code on link.tidal.com", style = IosTypography.body, color = colors.secondaryContent)
            Text(pending.userCode(), fontSize = 34.sp, fontWeight = FontWeight.Bold, letterSpacing = 4.sp)
            GlasmioButton(onClick = { uri.openUri(pending.verificationUrl()) }) { Text("Open link.tidal.com") }
            GlasmioButton(onClick = { pending.cancel() }) { Text("Cancel") }
        }
        error?.let { Text(it, style = IosTypography.footnote, color = colors.secondaryContent) }
    }
}
