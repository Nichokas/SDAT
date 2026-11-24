package com.nichokas.sdat.ui.kit

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.BasicAlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.DialogProperties
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupPositionProvider
import androidx.compose.ui.window.PopupProperties
import com.nichokas.sdat.ui.SdatIcon
import kotlin.math.roundToInt

/** A full-height sheet with iOS's rounded top and grabber; [title] names it for accessibility. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BottomSheet(onDismissRequest: () -> Unit, title: String, content: @Composable ColumnScope.() -> Unit) {
    val colors = SdatTheme.colors
    ModalBottomSheet(
        onDismissRequest = onDismissRequest,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        shape = RoundedCornerShape(topStart = 38.dp, topEnd = 38.dp),
        containerColor = if (colors.isDark) colors.elevatedBackground else colors.groupedBackground,
        contentColor = colors.content,
        dragHandle = {
            Box(Modifier.padding(top = 6.dp, bottom = 10.dp).size(36.dp, 5.dp).clip(capsule).background(colors.tertiaryContent))
        },
        contentWindowInsets = { WindowInsets.statusBars.union(WindowInsets.ime) },
    ) {
        NoGlassBackdrop {
            Column(Modifier.semantics { paneTitle = title }, content = content)
        }
    }
}

enum class AlertButtonLayout { SideBySide, Stacked }

enum class AlertButtonRole { Default, Cancel, Destructive }

data class AlertButton(val label: String, val onClick: () -> Unit, val role: AlertButtonRole = AlertButtonRole.Default)

/** An iOS alert: title, message, optional [content] such as a field, then capsule buttons. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AlertDialog(
    onDismissRequest: () -> Unit,
    title: String,
    message: String? = null,
    buttonLayout: AlertButtonLayout = AlertButtonLayout.SideBySide,
    buttons: List<AlertButton> = emptyList(),
    properties: DialogProperties = DialogProperties(usePlatformDefaultWidth = false),
    content: @Composable ColumnScope.() -> Unit = {},
) {
    val colors = SdatTheme.colors
    BasicAlertDialog(onDismissRequest = onDismissRequest, properties = properties) {
        NoGlassBackdrop {
            Column(
                Modifier
                    .width(300.dp)
                    .clip(RoundedCornerShape(34.dp))
                    .background(colors.elevatedBackground)
                    .padding(start = 20.dp, end = 20.dp, top = 22.dp, bottom = 16.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Text(title, Modifier.fillMaxWidth(), style = SdatType.headline, color = colors.content, textAlign = TextAlign.Center)
                message?.let {
                    Text(it, Modifier.fillMaxWidth(), style = SdatType.footnote, color = colors.content, textAlign = TextAlign.Center)
                }
                content()
                val buttonModifier = Modifier.padding(top = 10.dp)
                if (buttonLayout == AlertButtonLayout.Stacked) {
                    Column(buttonModifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        buttons.forEach { AlertButtonView(it, Modifier.fillMaxWidth()) }
                    }
                } else {
                    Row(buttonModifier, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        buttons.forEach { AlertButtonView(it, Modifier.weight(1f)) }
                    }
                }
            }
        }
    }
}

@Composable
private fun AlertButtonView(button: AlertButton, modifier: Modifier) {
    val colors = SdatTheme.colors
    val prominent = button.role == AlertButtonRole.Default
    Button(button.onClick, modifier, emphasis = if (prominent) Emphasis.Prominent else Emphasis.Regular) {
        Text(
            button.label,
            style = SdatType.headline,
            color = when (button.role) {
                AlertButtonRole.Default -> Color.White
                AlertButtonRole.Cancel -> colors.content
                AlertButtonRole.Destructive -> colors.destructive
            },
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

/** The rounded well an alert's fields sit in. */
@Composable
fun AlertFieldGroup(content: @Composable ColumnScope.() -> Unit) {
    Column(Modifier.padding(top = 10.dp).fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(SdatTheme.colors.fill), content = content)
}

@Composable
fun AlertTextField(value: TextFieldValue, onValueChange: (TextFieldValue) -> Unit, placeholder: String) {
    val colors = SdatTheme.colors
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = Modifier.fillMaxWidth().heightIn(min = 40.dp).padding(horizontal = 12.dp),
        textStyle = SdatType.body.copy(color = colors.content),
        singleLine = true,
        cursorBrush = SolidColor(colors.accent),
        decorationBox = { field ->
            Box(Modifier.heightIn(min = 40.dp), contentAlignment = Alignment.CenterStart) {
                if (value.text.isEmpty()) Text(placeholder, style = SdatType.body, color = colors.tertiaryContent)
                field()
            }
        },
    )
}

/** One row of a menu. [systemName] is an icon name from Symbols.kt, drawn at the trailing edge as on iOS. */
data class MenuAction(
    val title: String,
    val systemName: String? = null,
    val destructive: Boolean = false,
    val separatorBefore: Boolean = false,
    val enabled: Boolean = true,
    val onClick: () -> Unit = {},
    val subtitle: String? = null,
)

/** A menu that opens from [anchor]; [anchor] gets the toggle to call on tap. */
@Composable
fun Menu(
    expanded: Boolean,
    onExpandedChange: (Boolean) -> Unit,
    actions: List<MenuAction>,
    anchor: @Composable (toggle: () -> Unit) -> Unit,
) {
    var bounds by remember { mutableStateOf<Rect?>(null) }
    Box(Modifier.onGloballyPositioned { bounds = it.boundsInRoot() }) { anchor { onExpandedChange(!expanded) } }
    if (expanded) PopupMenu(bounds, actions, title = null, onDismiss = { onExpandedChange(false) })
}

/**
 * A menu beside [anchorBounds], in root coordinates: below it if it fits, else above, and toward the screen's middle.
 * It is never taller than the roomier side, so it never covers its anchor; a longer menu scrolls.
 */
@Composable
fun PopupMenu(anchorBounds: Rect?, actions: List<MenuAction>, title: String?, onDismiss: () -> Unit) {
    val colors = SdatTheme.colors
    val position = remember(anchorBounds) { MenuPosition(anchorBounds) }
    val window = LocalWindowInfo.current.containerSize
    val maxHeight = with(LocalDensity.current) {
        val margin = menuMargin(window.width)
        val room = anchorBounds?.let { maxOf(it.top, window.height - it.bottom) } ?: window.height.toFloat()
        (room - margin * 1.5f).coerceAtLeast(0f).toDp()
    }
    Popup(popupPositionProvider = position, onDismissRequest = onDismiss, properties = PopupProperties(focusable = true)) {
        NoGlassBackdrop {
            Column(
                Modifier
                    .width(250.dp)
                    .heightIn(max = maxHeight)
                    .glass(RoundedCornerShape(26.dp), tint = colors.elevatedBackground)
                    .verticalScroll(rememberScrollState())
                    .padding(vertical = 6.dp),
            ) {
                title?.let {
                    Text(
                        it,
                        Modifier.padding(horizontal = 18.dp, vertical = 8.dp),
                        style = SdatType.footnote,
                        color = colors.secondaryContent,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Divider()
                }
                actions.forEach { action ->
                    if (action.separatorBefore) Divider()
                    MenuRow(action) { onDismiss(); action.onClick() }
                }
            }
        }
    }
}

@Composable
private fun Divider() {
    Box(Modifier.padding(vertical = 4.dp).fillMaxWidth().height(6.dp).background(SdatTheme.colors.separator.copy(alpha = 0.5f)))
}

@Composable
private fun MenuRow(action: MenuAction, onClick: () -> Unit) {
    val colors = SdatTheme.colors
    val tint = if (action.destructive) colors.destructive else colors.content
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 44.dp)
            .clickable(enabled = action.enabled, role = Role.Button, onClick = onClick)
            .padding(horizontal = 18.dp, vertical = 6.dp)
            .alpha(if (action.enabled) 1f else 0.4f),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(action.title, style = SdatType.body, color = tint, maxLines = 1, overflow = TextOverflow.Ellipsis)
            action.subtitle?.let { Text(it, style = SdatType.footnote, color = colors.secondaryContent, maxLines = 2) }
        }
        action.systemName?.let { SdatIcon(it, null, size = 20.dp, tint = tint) }
    }
}

private fun menuMargin(windowWidth: Int) = (windowWidth * 0.02f).roundToInt().coerceAtLeast(16)

private class MenuPosition(private val anchor: Rect?) : PopupPositionProvider {
    override fun calculatePosition(anchorBounds: IntRect, windowSize: IntSize, layoutDirection: LayoutDirection, popupContentSize: IntSize): IntOffset {
        val a = anchor ?: Rect(anchorBounds.left.toFloat(), anchorBounds.top.toFloat(), anchorBounds.right.toFloat(), anchorBounds.bottom.toFloat())
        val margin = menuMargin(windowSize.width)
        val x = if (a.center.x > windowSize.width / 2f) a.right - popupContentSize.width else a.left
        val below = a.bottom + margin / 2
        val above = a.top - margin / 2 - popupContentSize.height
        val y = when {
            below + popupContentSize.height <= windowSize.height - margin -> below
            above >= margin -> above
            // Without anchorBounds the height cap is the whole window, so neither side may fit.
            windowSize.height - a.bottom > a.top -> below
            else -> above
        }
        return IntOffset(
            x.roundToInt().coerceIn(margin, (windowSize.width - popupContentSize.width - margin).coerceAtLeast(margin)),
            y.roundToInt().coerceIn(margin, (windowSize.height - popupContentSize.height - margin).coerceAtLeast(margin)),
        )
    }
}
