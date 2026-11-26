package com.nichokas.sdat.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.slideInVertically
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nichokas.sdat.core.Session
import com.nichokas.sdat.ui.kit.Button
import com.nichokas.sdat.ui.kit.Emphasis
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import uniffi.tonearm_ffi.DeviceLogin
import uniffi.tonearm_ffi.FfiException
import uniffi.tonearm_ffi.startDeviceLogin

@Composable
fun LoginView(session: Session) {
    val scope = rememberCoroutineScope()
    val uri = LocalUriHandler.current
    var login by remember { mutableStateOf<DeviceLogin?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var attempt by remember { mutableStateOf<Job?>(null) }
    val pending = login
    val colors = SdatTheme.colors

    Column(
        Modifier.fillMaxSize().safeDrawingPadding().padding(start = 24.dp, end = 24.dp, bottom = 12.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        BoxWithConstraints(Modifier.weight(1f).fillMaxWidth()) {
            Column(
                Modifier.verticalScroll(rememberScrollState()).fillMaxWidth().heightIn(min = maxHeight).padding(vertical = 24.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(24.dp, Alignment.CenterVertically),
            ) {
                AnimatedVisibility(
                    visibleState = remember { MutableTransitionState(false).apply { targetState = true } },
                    enter = fadeIn() + slideInVertically { it / 4 },
                ) {
                    Text(
                        "SDAT",
                        style = SdatType.largeTitle,
                        fontSize = 48.sp,
                        lineHeight = 56.sp,
                        fontWeight = FontWeight.ExtraBold,
                        letterSpacing = 4.sp,
                    )
                }
                if (pending != null) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        Text(
                            "Enter this code on TIDAL",
                            style = SdatType.subheadline,
                            color = colors.secondaryContent,
                            textAlign = TextAlign.Center,
                        )
                        SelectionContainer {
                            Text(
                                pending.userCode(),
                                style = SdatType.largeTitle,
                                fontFamily = FontFamily.Monospace,
                                fontWeight = FontWeight.Bold,
                                textAlign = TextAlign.Center,
                            )
                        }
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            CircularProgressIndicator(Modifier.size(16.dp), color = colors.secondaryContent, strokeWidth = 2.dp)
                            Text("Waiting for approval…", style = SdatType.footnote, color = colors.secondaryContent)
                        }
                    }
                }
                error?.let { Text(it, style = SdatType.footnote, color = colors.destructive, textAlign = TextAlign.Center) }
            }
        }

        if (pending == null) {
            Button(
                modifier = Modifier.fillMaxWidth().height(56.dp),
                emphasis = Emphasis.Prominent,
                enabled = attempt == null,
                onClick = {
                    if (attempt != null) return@Button
                    error = null
                    val job = scope.launch(start = CoroutineStart.LAZY) {
                        try {
                            startDeviceLogin().use { deviceLogin ->
                                login = deviceLogin
                                try {
                                    val tokens = deviceLogin.wait()
                                    // Cancellation can race with the native poll's last check.
                                    ensureActive()
                                    session.signIn(tokens)
                                } catch (e: CancellationException) {
                                    // The native poll outlives the coroutine unless told to stop.
                                    deviceLogin.cancel()
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
                            attempt = null
                        }
                    }
                    attempt = job
                    job.start()
                },
            ) {
                if (attempt != null) {
                    CircularProgressIndicator(Modifier.size(18.dp), color = LocalContentColor.current, strokeWidth = 2.dp)
                    Spacer(Modifier.width(8.dp))
                }
                Text("Sign in with TIDAL")
            }
        } else {
            Button(
                modifier = Modifier.fillMaxWidth().height(56.dp),
                emphasis = Emphasis.Prominent,
                onClick = {
                    if (login !== pending) return@Button
                    error = try {
                        uri.openUri(pending.verificationUrl())
                        null
                    } catch (_: IllegalArgumentException) {
                        "Could not open TIDAL. Enter this code at link.tidal.com."
                    }
                },
            ) {
                Text("Open TIDAL")
                Spacer(Modifier.width(8.dp))
                SdatIcon("arrow.up.right", null, size = 18.dp)
            }
            TextButton(onClick = {
                if (login !== pending) return@TextButton
                error = null
                pending.cancel()
                attempt?.cancel()
            }) {
                Text("Cancel", style = SdatType.subheadline, color = colors.secondaryContent)
            }
        }
    }
}
