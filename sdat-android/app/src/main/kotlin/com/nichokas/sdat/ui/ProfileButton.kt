package com.nichokas.sdat.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import com.nichokas.sdat.core.Session
import com.nichokas.sdat.ui.kit.IconButton
import uniffi.tonearm_ffi.profilePictureUrl

/** iOS ProfileButton: the circular TIDAL profile photo (generic silhouette as a fallback) that opens Settings. */
@Composable
fun ProfileButton(session: Session, onClick: () -> Unit, modifier: Modifier = Modifier) {
    LaunchedEffect(Unit) { runCatching { profilePictureUrl() }.onSuccess { session.profilePicture = it } }
    IconButton(onClick, modifier.size(44.dp)) {
        Box(Modifier.fillMaxSize().padding(4.dp)) { Avatar(session.profilePicture) }
    }
}

@Composable
private fun Avatar(url: String?) {
    val circle = Modifier.fillMaxSize().clip(CircleShape)
    if (url != null) {
        AsyncImage(url, contentDescription = "Settings", modifier = circle, contentScale = ContentScale.Crop)
        return
    }
    // Head and shoulders, as iOS AvatarSilhouette; the circle clip cuts the shoulders off.
    Canvas(circle.background(Color(0xFF383838))) {
        val w = size.width
        val h = size.height
        val figure = Color(0xFF9E9E9E)
        drawOval(figure, topLeft = Offset(w * 0.32f, h * 0.18f), size = Size(w * 0.36f, h * 0.36f))
        drawOval(figure, topLeft = Offset(w * 0.14f, h * 0.62f), size = Size(w * 0.72f, h * 0.64f))
    }
}
