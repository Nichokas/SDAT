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
import androidx.lifecycle.compose.LifecycleResumeEffect
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.qrcode.QRCodeWriter
import com.google.zxing.qrcode.decoder.ErrorCorrectionLevel
import com.nichokas.sdat.core.Live
import com.nichokas.sdat.core.LiveMode
import com.nichokas.sdat.ui.kit.BottomSheet
import com.nichokas.sdat.ui.kit.GroupedFooter
import com.nichokas.sdat.ui.kit.GroupedList
import com.nichokas.sdat.ui.kit.ListRow
import com.nichokas.sdat.ui.kit.SdatTheme
import com.nichokas.sdat.ui.kit.SdatType
import com.nichokas.sdat.ui.kit.SectionTitle
import com.nichokas.sdat.ui.kit.Stepper
import com.nichokas.sdat.ui.kit.Switch

/** iOS LiveSheet: start, share the QR code, listeners, guest control, output delay, leave. */
@Composable
fun LiveSheet() {
    RequestLocalNetwork(Live.mode.active)
    BottomSheet(onDismissRequest = { Live.dismissEnded(); Live.showSheet = false }, title = "Listen Together") {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp).navigationBarsPadding()) {
            LiveContent()
        }
    }
}

@Composable
private fun LiveContent() {
    val colors = SdatTheme.colors
    when (val mode = Live.mode) {
        LiveMode.Idle -> {
            GroupedList {
                ListRow("Start a session", titleColor = colors.accent, showTopSeparator = false, showDisclosure = false, onClick = Live::startHosting)
                ListRow("Scan an invitation", titleColor = colors.accent, showDisclosure = false, onClick = { Live.showScanner = true })
            }
            GroupedFooter("Everyone listens with their own TIDAL account. The host controls playback and can let guests help.")
        }
        LiveMode.Starting, LiveMode.Joining -> GroupedList {
            ListRow(
                if (mode == LiveMode.Starting) "Starting…" else "Joining…",
                showTopSeparator = false,
                trailing = { CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp) },
            )
            ListRow("Cancel", titleColor = colors.destructive, showDisclosure = false, onClick = Live::leave)
        }
        LiveMode.Hosting -> {
            Live.invite?.let { invite ->
                GroupedList {
                    Box(Modifier.fillMaxWidth().padding(vertical = 8.dp), contentAlignment = Alignment.Center) { QrCode(invite) }
                    ShareRow(invite)
                }
                GroupedFooter("Friends scan this from My Collection.")
            }
            GroupedList {
                ListRow("Allow guests to control playback", showTopSeparator = false, trailing = { Switch(Live.guestControl, Live::allowGuestControl) })
            }
            GroupedFooter("Play, pause, skip and add songs to the queue.")
            SectionTitle("Listening (${Live.guests.size})")
            GroupedList {
                if (Live.guests.isEmpty()) ListRow("Nobody yet", titleColor = colors.secondaryContent, showTopSeparator = false)
                Live.guests.forEachIndexed { i, g -> ListRow(g.name, showTopSeparator = i > 0) }
            }
            Destructive("End session")
        }
        LiveMode.Joined -> {
            GroupedList {
                ListRow("Host", detail = Live.hostName, showTopSeparator = false)
                Text(
                    if (Live.guestControl) "The host lets you play, pause, skip and add songs." else "The host controls playback.",
                    style = SdatType.footnote,
                    color = colors.secondaryContent,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 10.dp),
                )
                if (Live.reconnecting) {
                    ListRow("Reconnecting…", trailing = { CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp) })
                }
            }
            GroupedList(Modifier.padding(top = 24.dp)) {
                // The stepper counts steps of 10 ms, as iOS's Stepper(step: 10).
                ListRow(
                    "Output delay: ${Live.extraLatencyMs} ms",
                    showTopSeparator = false,
                    trailing = { Stepper(Live.extraLatencyMs / 10, { Live.extraLatencyMs = it * 10 }, Modifier, 0..50) },
                )
            }
            GroupedFooter("Raise this if your speakers or headphones sound behind.")
            Destructive("Leave")
        }
        is LiveMode.Ended -> GroupedList {
            ListRow(mode.message, showTopSeparator = false)
            ListRow("OK", titleColor = colors.accent, showDisclosure = false, onClick = Live::dismissEnded)
        }
    }
}

@Composable
private fun Destructive(title: String) {
    GroupedList(Modifier.padding(top = 24.dp)) {
        ListRow(title, titleColor = SdatTheme.colors.destructive, showTopSeparator = false, showDisclosure = false, onClick = Live::leave)
    }
}

@Composable
private fun ShareRow(invite: String) {
    val context = LocalContext.current
    ListRow("Share link", titleColor = SdatTheme.colors.accent, showDisclosure = false, onClick = {
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
    // Access granted in Settings while the sheet stays open.
    LifecycleResumeEffect(Unit) {
        if (context.checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
            granted = true
            denied = false
        }
        onPauseOrDispose {}
    }
    BottomSheet(onDismissRequest = onDismiss, title = "Scan invitation") {
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
                SdatIcon("camera", null, size = 44.dp, tint = SdatTheme.colors.secondaryContent)
                Text("Camera access needed", style = SdatType.title3, color = SdatTheme.colors.content)
                Text("Allow camera access for SDAT in Settings to scan invitations.", style = SdatType.body, color = SdatTheme.colors.secondaryContent)
                Text(
                    "Open Settings",
                    style = SdatType.body,
                    color = SdatTheme.colors.accent,
                    modifier = Modifier.padding(top = 8.dp).clickable {
                        context.startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null)))
                    },
                )
            }
        }
    }
}
