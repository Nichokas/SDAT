package com.nichokas.sdat.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import com.nichokas.sdat.core.Preferences
import io.github.neoruaa.glasmio.glass.GlassSurface
import uniffi.tonearm_ffi.profilePictureUrl

/** iOS ProfileButton: the circular TIDAL profile photo (generic glyph as a fallback) that opens Settings. */
@Composable
fun ProfileButton(prefs: Preferences, onClick: () -> Unit, modifier: Modifier = Modifier) {
    LaunchedEffect(Unit) { runCatching { profilePictureUrl() }.onSuccess { prefs.profilePicture = it } }
    GlassSurface(modifier.size(44.dp), shape = CircleShape) {
        Box(Modifier.fillMaxSize().clickable(role = Role.Button, onClickLabel = "Settings", onClick = onClick).padding(4.dp)) {
            Avatar(prefs.profilePicture)
        }
    }
}

@Composable
private fun Avatar(url: String?) {
    val circle = Modifier.fillMaxSize().clip(CircleShape)
    if (url != null) {
        AsyncImage(url, contentDescription = "Settings", modifier = circle, contentScale = ContentScale.Crop)
        return
    }
    Canvas(circle.background(Color(0xFF1A1A1A))) {
        val inset = size.width * 0.24f
        val w = size.width - 2 * inset
        val h = size.height - 2 * inset
        fun p(x: Float, y: Float) = Offset(inset + w * x, inset + h * y)
        // Two crossing loops, as iOS AvatarGlyph.
        val path = Path().apply {
            p(0.15f, 0.95f).let { moveTo(it.x, it.y) }
            curve(p(0.45f, 0.75f), p(0.62f, 0.2f), p(0.42f, 0.15f))
            curve(p(0.25f, 0.1f), p(0.05f, 0.3f), p(0.2f, 0.45f))
            curve(p(0.4f, 0.65f), p(0.95f, 0.6f), p(0.8f, 0.4f))
            curve(p(0.7f, 0.18f), p(0.62f, 0.12f), p(0.58f, 0.15f))
            curve(p(0.5f, 0.55f), p(0.65f, 0.8f), p(0.88f, 0.95f))
        }
        drawPath(
            path,
            Brush.linearGradient(listOf(Color(0.55f, 0.7f, 1f), Color.White), start = Offset.Zero, end = Offset(size.width, size.height)),
            style = Stroke(width = maxOf(1.5.dp.toPx(), size.width * 0.06f), cap = StrokeCap.Round, join = StrokeJoin.Round),
        )
    }
}

private fun Path.curve(c1: Offset, c2: Offset, to: Offset) = cubicTo(c1.x, c1.y, c2.x, c2.y, to.x, to.y)
