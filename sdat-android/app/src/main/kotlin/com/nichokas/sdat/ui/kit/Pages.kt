package com.nichokas.sdat.ui.kit

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nichokas.sdat.ui.SdatIcon

/** Height of the header row on the frame, under the status bar. */
val HeaderHeight = 56.dp

private val sheetShape = RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp)

/**
 * A header on the frame, then [content] on a sheet with rounded top corners. A tab root, with no [onNavigateBack],
 * names itself in a larger title; a pushed page shows a back button beside a smaller one.
 */
@Composable
fun SheetPage(
    title: String,
    onNavigateBack: (() -> Unit)?,
    modifier: Modifier = Modifier,
    actions: @Composable RowScope.() -> Unit = {},
    content: @Composable BoxScope.() -> Unit,
) {
    val colors = SdatTheme.colors
    Column(modifier.fillMaxSize().background(colors.frame).statusBarsPadding()) {
        Row(
            Modifier.fillMaxWidth().height(HeaderHeight).padding(start = if (onNavigateBack == null) 20.dp else 16.dp, end = 16.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (onNavigateBack != null) IconButton(onNavigateBack) { SdatIcon("arrow.left", "Back", size = 22.dp) }
            Text(
                title,
                Modifier.weight(1f).padding(start = if (onNavigateBack == null) 0.dp else 4.dp).semantics { heading() },
                color = colors.content,
                style = if (onNavigateBack == null) SdatType.title3.copy(fontSize = 20.sp, fontWeight = FontWeight.SemiBold) else SdatType.headline.copy(fontSize = 16.sp, fontWeight = FontWeight.Medium),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            actions()
        }
        Box(Modifier.padding(top = 8.dp).fillMaxSize().clip(sheetShape).background(colors.sheet), content = content)
    }
}
