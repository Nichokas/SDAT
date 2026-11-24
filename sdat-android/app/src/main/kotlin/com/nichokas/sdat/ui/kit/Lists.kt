package com.nichokas.sdat.ui.kit

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.ui.SdatIcon

private val groupShape = RoundedCornerShape(26.dp)

/** A rounded card of rows, as in iOS grouped tables. */
@Composable
fun GroupedList(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    val colors = SdatTheme.colors
    Column(modifier.fillMaxWidth().clip(groupShape).background(colors.elevatedBackground)) {
        CompositionLocalProvider(LocalContentColor provides colors.content) { content() }
    }
}

/** A 52 dp row, or 68 dp with a [subtitle]. Rows after the first in a group draw their separator on top. */
@Composable
fun ListRow(
    title: String,
    modifier: Modifier = Modifier,
    subtitle: String? = null,
    detail: String? = null,
    onClick: (() -> Unit)? = null,
    trailing: (@Composable () -> Unit)? = null,
    showTopSeparator: Boolean = true,
    enabled: Boolean = true,
    showDisclosure: Boolean = onClick != null && trailing == null,
    titleColor: Color = Color.Unspecified,
) {
    val colors = SdatTheme.colors
    Row(
        modifier
            .fillMaxWidth()
            .heightIn(min = if (subtitle == null) 52.dp else 68.dp)
            .then(if (onClick != null) Modifier.clickable(enabled = enabled, role = Role.Button, onClick = onClick) else Modifier)
            .padding(start = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Row(
            Modifier
                .weight(1f)
                .heightIn(min = if (subtitle == null) 52.dp else 68.dp)
                .drawBehind {
                    if (showTopSeparator) drawLine(colors.separator, Offset.Zero, Offset(size.width, 0f), strokeWidth = 1.dp.toPx() / 2)
                }
                .padding(end = 16.dp)
                .alpha(if (enabled) 1f else 0.4f),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f).padding(vertical = 6.dp)) {
                Text(
                    title,
                    style = SdatType.body,
                    color = if (titleColor == Color.Unspecified) colors.content else titleColor,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                subtitle?.let {
                    Text(it, style = SdatType.subheadline, color = colors.secondaryContent, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                detail?.let { Text(it, style = SdatType.body, color = colors.secondaryContent, maxLines = 1) }
                trailing?.let { CompositionLocalProvider(LocalContentColor provides colors.content) { it() } }
                if (showDisclosure) SdatIcon("chevron.right", contentDescription = null, size = 22.dp, tint = colors.tertiaryContent)
            }
        }
    }
}

/** The label above a group, in its original casing. */
@Composable
fun SectionTitle(title: String, modifier: Modifier = Modifier) {
    Text(
        title,
        modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 17.dp, bottom = 9.dp),
        style = SdatType.body.copy(fontWeight = FontWeight.SemiBold),
        color = SdatTheme.colors.secondaryContent,
    )
}

/** Explanatory text under a group. */
@Composable
fun GroupedFooter(text: String, modifier: Modifier = Modifier) {
    Text(
        text,
        modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 6.dp),
        style = SdatType.footnote,
        color = SdatTheme.colors.secondaryContent,
    )
}
