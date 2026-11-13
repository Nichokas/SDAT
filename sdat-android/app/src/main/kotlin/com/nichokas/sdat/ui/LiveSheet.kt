package com.nichokas.sdat.ui

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.qrcode.QRCodeWriter
import com.google.zxing.qrcode.decoder.ErrorCorrectionLevel
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.core.LiveMode
import io.github.neoruaa.glasmio.core.GlasmioTheme
import io.github.neoruaa.glasmio.core.IosTypography
import io.github.neoruaa.glasmio.ui.GlasmioBottomSheet
import io.github.neoruaa.glasmio.ui.GlasmioGroupedList
import io.github.neoruaa.glasmio.ui.GlasmioGroupedTableFooter
import io.github.neoruaa.glasmio.ui.GlasmioListRow
import io.github.neoruaa.glasmio.ui.GlasmioSectionTitle
import io.github.neoruaa.glasmio.ui.GlasmioStepper
import io.github.neoruaa.glasmio.ui.GlasmioSwitch

/** iOS LiveSheet: start, share the QR code, listeners, guest control, output delay, leave. */
@Composable
fun LiveSheet() {
    RequestLocalNetwork(Live.mode.active)
    GlasmioBottomSheet(onDismissRequest = { Live.dismissEnded(); Live.showSheet = false }, title = "Listen Together") {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp).navigationBarsPadding()) {
            LiveContent()
        }
    }
}

@Composable
private fun LiveContent() {
    val colors = GlasmioTheme.colors
    when (val mode = Live.mode) {
        LiveMode.Idle -> {
            GlasmioGroupedList {
                GlasmioListRow("Start a session", titleColor = colors.accent, showTopSeparator = false, showDisclosure = false, onClick = Live::startHosting)
                GlasmioListRow("Scan an invitation", titleColor = colors.accent, showDisclosure = false, onClick = { Live.showScanner = true })
            }
            GlasmioGroupedTableFooter("Everyone listens with their own TIDAL account. The host controls playback and can let guests help.")
        }
        LiveMode.Starting, LiveMode.Joining -> GlasmioGroupedList {
            GlasmioListRow(
                if (mode == LiveMode.Starting) "Starting…" else "Joining…",
                showTopSeparator = false,
                trailing = { CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp) },
            )
            GlasmioListRow("Cancel", titleColor = colors.destructive, showDisclosure = false, onClick = Live::leave)
        }
        LiveMode.Hosting -> {
            Live.invite?.let { invite ->
                GlasmioGroupedList {
                    Box(Modifier.fillMaxWidth().padding(vertical = 8.dp), contentAlignment = Alignment.Center) { QrCode(invite) }
                    ShareRow(invite)
                }
                GlasmioGroupedTableFooter("Friends scan this from My Collection.")
            }
            GlasmioGroupedList {
                GlasmioListRow("Allow guests to control playback", showTopSeparator = false, trailing = { GlasmioSwitch(Live.guestControl, Live::allowGuestControl) })
            }
            GlasmioGroupedTableFooter("Play, pause, skip and add songs to the queue.")
            GlasmioSectionTitle("Listening (${Live.guests.size})")
            GlasmioGroupedList {
                if (Live.guests.isEmpty()) GlasmioListRow("Nobody yet", titleColor = colors.secondaryContent, showTopSeparator = false)
                Live.guests.forEachIndexed { i, g -> GlasmioListRow(g.name, showTopSeparator = i > 0) }
            }
            Destructive("End session")
        }
        LiveMode.Joined -> {
            GlasmioGroupedList {
                GlasmioListRow("Host", detail = Live.hostName, showTopSeparator = false)
                Text(
                    if (Live.guestControl) "The host lets you play, pause, skip and add songs." else "The host controls playback.",
                    style = IosTypography.footnote,
                    color = colors.secondaryContent,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 10.dp),
                )
                if (Live.reconnecting) {
                    GlasmioListRow("Reconnecting…", trailing = { CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp) })
                }
            }
            GlasmioGroupedList(Modifier.padding(top = 24.dp)) {
                // The stepper counts steps of 10 ms, as iOS's Stepper(step: 10).
                GlasmioListRow(
                    "Output delay: ${Live.extraLatencyMs} ms",
                    showTopSeparator = false,
                    trailing = { GlasmioStepper(Live.extraLatencyMs / 10, { Live.extraLatencyMs = it * 10 }, Modifier, 0..50) },
                )
            }
            GlasmioGroupedTableFooter("Raise this if your speakers or headphones sound behind.")
            Destructive("Leave")
        }
        is LiveMode.Ended -> GlasmioGroupedList {
            GlasmioListRow(mode.message, showTopSeparator = false)
            GlasmioListRow("OK", titleColor = colors.accent, showDisclosure = false, onClick = Live::dismissEnded)
        }
    }
}

@Composable
private fun Destructive(title: String) {
    GlasmioGroupedList(Modifier.padding(top = 24.dp)) {
        GlasmioListRow(title, titleColor = GlasmioTheme.colors.destructive, showTopSeparator = false, showDisclosure = false, onClick = Live::leave)
    }
}

@Composable
private fun ShareRow(invite: String) {
    val context = LocalContext.current
    GlasmioListRow("Share link", titleColor = GlasmioTheme.colors.accent, showDisclosure = false, onClick = {
        val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, invite)
        context.startActivity(Intent.createChooser(send, null))
    })
}

/** The invitation as a QR code, black on white with a quiet zone, as iOS draws it. */
@Composable
private fun QrCode(text: String) {
    val bitmap = remember(text) { qrBitmap(text) }
    Image(
        bitmap.asImageBitmap(),
        contentDescription = "Invitation QR code",
        filterQuality = FilterQuality.None,
        modifier = Modifier.size(264.dp).background(Color.White, RoundedCornerShape(12.dp)).padding(12.dp),
    )
}

private fun qrBitmap(text: String): Bitmap {
    val hints = mapOf(EncodeHintType.ERROR_CORRECTION to ErrorCorrectionLevel.M, EncodeHintType.MARGIN to 0)
    val m = QRCodeWriter().encode(text, BarcodeFormat.QR_CODE, 0, 0, hints)
    val pixels = IntArray(m.width * m.height) { i -> if (m.get(i % m.width, i / m.width)) android.graphics.Color.BLACK else android.graphics.Color.WHITE }
    return Bitmap.createBitmap(pixels, m.width, m.height, Bitmap.Config.ARGB_8888)
}

/**
 * Apps targeting Android 17 reach the local network only with this permission; without it iroh falls back to a relay
 * (slower, still in sync). Asked when a session starts or is joined, as iOS's local network prompt.
 */
@Composable
private fun RequestLocalNetwork(active: Boolean) {
    if (Build.VERSION.SDK_INT < 37) return
    val context = LocalContext.current
    val request = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {}
    LaunchedEffect(active) {
        if (active && context.checkSelfPermission(Manifest.permission.ACCESS_LOCAL_NETWORK) != PackageManager.PERMISSION_GRANTED) {
            request.launch(Manifest.permission.ACCESS_LOCAL_NETWORK)
        }
    }
}

/** iOS LiveScannerSheet: the camera reads an invitation, or explains how to allow the camera. */
@Composable
fun ScannerSheet(onDismiss: () -> Unit) {
    val context = LocalContext.current
    var granted by remember { mutableStateOf(context.checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) }
    var denied by remember { mutableStateOf(false) }
    val request = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted = it; denied = !it }
    LaunchedEffect(Unit) { if (!granted) request.launch(Manifest.permission.CAMERA) }
    GlasmioBottomSheet(onDismissRequest = onDismiss, title = "Scan invitation") {
        when {
            granted -> AndroidView(
                factory = { QrScannerView(it, Live::open) },
                modifier = Modifier.fillMaxSize(),
                onRelease = QrScannerView::release,
            )
            denied -> Column(
                Modifier.fillMaxSize().padding(32.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                SdatIcon("camera", null, size = 44.dp, tint = GlasmioTheme.colors.secondaryContent)
                Text("Camera access needed", style = IosTypography.title3, color = GlasmioTheme.colors.content)
                Text("Allow camera access for SDAT in Settings to scan invitations.", style = IosTypography.body, color = GlasmioTheme.colors.secondaryContent)
                Text(
                    "Open Settings",
                    style = IosTypography.body,
                    color = GlasmioTheme.colors.accent,
                    modifier = Modifier.padding(top = 8.dp).clickable {
                        context.startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null)))
                    },
                )
            }
        }
    }
}
