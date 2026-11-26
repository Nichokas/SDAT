package com.nichokas.sdat.ui.kit

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LocalContentColor
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.dp

val capsule = RoundedCornerShape(percent = 50)

/** A solid surface floating over the page, such as a menu or a toast, with a soft shadow. */
fun Modifier.raised(shape: Shape, color: Color): Modifier =
    shadow(8.dp, shape, ambientColor = Color.Black.copy(alpha = 0.12f), spotColor = Color.Black.copy(alpha = 0.18f))
        .clip(shape)
        .background(color)

@Composable
fun Raised(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(24.dp),
    color: Color = SdatTheme.colors.elevatedBackground,
    contentColor: Color = SdatTheme.colors.content,
    content: @Composable BoxScope.() -> Unit = {},
) {
    Box(modifier.raised(shape, color)) {
        CompositionLocalProvider(LocalContentColor provides contentColor) { content() }
    }
}
