package com.nichokas.sdat.ui.kit

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.Slider as MaterialSlider
import androidx.compose.material3.Switch as MaterialSwitch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.nichokas.sdat.ui.SdatIcon

enum class ButtonSize(val height: Dp, val padding: Dp) { Small(28.dp, 12.dp), Medium(34.dp, 14.dp), Large(50.dp, 20.dp) }

enum class Emphasis { Regular, Prominent }

/** A capsule button: accent-filled when [emphasis] is prominent, otherwise a neutral fill. */
@Composable
fun Button(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    emphasis: Emphasis = Emphasis.Regular,
    size: ButtonSize? = null,
    content: @Composable RowScope.() -> Unit,
) {
    val colors = SdatTheme.colors
    val prominent = emphasis == Emphasis.Prominent
    Row(
        modifier
            .height(size?.height ?: 48.dp)
            .clip(capsule)
            .background(if (prominent) colors.accent else colors.fill)
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .padding(horizontal = size?.padding ?: 20.dp)
            .alpha(if (enabled) 1f else 0.4f),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CompositionLocalProvider(
            LocalContentColor provides if (prominent) Color.White else colors.content,
            LocalTextStyle provides SdatType.body.copy(fontWeight = SdatType.headline.fontWeight),
        ) { content() }
    }
}

/** A 44 dp glass circle around an icon. */
@Composable
fun IconButton(onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, content: @Composable BoxScope.() -> Unit) {
    Box(
        modifier
            .defaultMinSize(44.dp, 44.dp)
            .glass(CircleShape)
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .alpha(if (enabled) 1f else 0.4f),
        contentAlignment = Alignment.Center,
    ) {
        CompositionLocalProvider(LocalContentColor provides SdatTheme.colors.content) { content() }
    }
}

private val switchGreen = Color(0xFF34C759)

@Composable
fun Switch(checked: Boolean, onCheckedChange: (Boolean) -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true) {
    val colors = SdatTheme.colors
    MaterialSwitch(
        checked = checked,
        onCheckedChange = onCheckedChange,
        modifier = modifier,
        enabled = enabled,
        thumbContent = {},
        colors = SwitchDefaults.colors(
            checkedThumbColor = Color.White,
            checkedTrackColor = switchGreen,
            checkedBorderColor = Color.Transparent,
            uncheckedThumbColor = Color.White,
            uncheckedTrackColor = colors.fill,
            uncheckedBorderColor = Color.Transparent,
        ),
    )
}

/** A thin capsule track with a small round thumb, drawn in the content color. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun Slider(
    value: Float,
    onValueChange: (Float) -> Unit,
    modifier: Modifier = Modifier,
    valueRange: ClosedFloatingPointRange<Float> = 0f..1f,
    enabled: Boolean = true,
    onValueChangeFinished: (() -> Unit)? = null,
) {
    val content = LocalContentColor.current
    MaterialSlider(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier.fillMaxWidth().height(28.dp),
        enabled = enabled,
        valueRange = valueRange,
        onValueChangeFinished = onValueChangeFinished,
        thumb = { Box(Modifier.size(18.dp).shadow(2.dp, CircleShape).background(Color.White, CircleShape)) },
        track = { state ->
            Canvas(Modifier.fillMaxWidth().height(6.dp)) {
                val span = state.valueRange.endInclusive - state.valueRange.start
                val f = if (span > 0f) ((state.value - state.valueRange.start) / span).coerceIn(0f, 1f) else 0f
                val r = CornerRadius(size.height / 2)
                drawRoundRect(content.copy(alpha = 0.25f), cornerRadius = r)
                drawRoundRect(content.copy(alpha = if (enabled) 0.9f else 0.5f), size = Size(size.width * f, size.height), cornerRadius = r)
            }
        },
    )
}

/** iOS stepper: a capsule split into minus and plus. */
@Composable
fun Stepper(value: Int, onValueChange: (Int) -> Unit, modifier: Modifier = Modifier, range: IntRange = Int.MIN_VALUE..Int.MAX_VALUE) {
    val colors = SdatTheme.colors
    // A stored value outside the range steps from its nearest end, so the first tap brings it back in.
    val current = value.coerceIn(range)
    Row(modifier.size(94.dp, 32.dp).clip(capsule).background(colors.fill), verticalAlignment = Alignment.CenterVertically) {
        StepperHalf("minus", "Decrease", current > range.first) { onValueChange(current - 1) }
        Box(Modifier.width(1.dp).height(18.dp).background(colors.separator))
        StepperHalf("plus", "Increase", current < range.last) { onValueChange(current + 1) }
    }
}

@Composable
private fun RowScope.StepperHalf(symbol: String, label: String, enabled: Boolean, onClick: () -> Unit) {
    Box(
        Modifier.weight(1f).fillMaxHeight().clickable(enabled = enabled, role = Role.Button, onClickLabel = label, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        SdatIcon(symbol, label, size = 20.dp, tint = SdatTheme.colors.content.copy(alpha = if (enabled) 1f else 0.3f))
    }
}

/** A borderless field for a grouped list row. */
@Composable
fun TextField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "",
    visualTransformation: VisualTransformation = VisualTransformation.None,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    keyboardActions: KeyboardActions = KeyboardActions.Default,
) {
    val colors = SdatTheme.colors
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier.fillMaxWidth().heightIn(min = 52.dp).padding(horizontal = 16.dp),
        textStyle = SdatType.body.copy(color = colors.content),
        singleLine = true,
        cursorBrush = SolidColor(colors.accent),
        visualTransformation = visualTransformation,
        keyboardOptions = keyboardOptions,
        keyboardActions = keyboardActions,
        decorationBox = { field ->
            Box(Modifier.heightIn(min = 52.dp), contentAlignment = Alignment.CenterStart) {
                if (value.isEmpty()) Text(placeholder, style = SdatType.body, color = colors.tertiaryContent, maxLines = 1)
                field()
            }
        },
    )
}

/** A glass capsule search field; while [active], a close button beside it ends the search. */
@Composable
fun SearchBar(
    query: TextFieldValue,
    onQueryChange: (TextFieldValue) -> Unit,
    active: Boolean,
    onActiveChange: (Boolean) -> Unit,
    onClose: () -> Unit,
    placeholder: String,
    closeContentDescription: String,
    modifier: Modifier = Modifier,
    onSearch: (String) -> Unit = {},
) {
    val colors = SdatTheme.colors
    val focus = remember { FocusRequester() }
    val focusManager = LocalFocusManager.current
    Row(modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
        Row(
            Modifier
                .weight(1f)
                .height(48.dp)
                .glass(capsule)
                .clickable(role = Role.Button, onClickLabel = placeholder) { focus.requestFocus() }
                .padding(horizontal = 14.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            SdatIcon("magnifyingglass", null, size = 22.dp, tint = colors.secondaryContent)
            BasicTextField(
                value = query,
                onValueChange = onQueryChange,
                modifier = Modifier.weight(1f).focusRequester(focus).onFocusChanged { if (it.isFocused && !active) onActiveChange(true) },
                textStyle = SdatType.body.copy(color = colors.content),
                singleLine = true,
                cursorBrush = SolidColor(colors.accent),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                keyboardActions = KeyboardActions(onSearch = { onSearch(query.text) }),
                decorationBox = { field ->
                    Box(contentAlignment = Alignment.CenterStart) {
                        if (query.text.isEmpty()) Text(placeholder, style = SdatType.body, color = colors.secondaryContent, maxLines = 1)
                        field()
                    }
                },
            )
        }
        if (active) {
            IconButton({ focusManager.clearFocus(); onClose() }, Modifier.size(48.dp)) { SdatIcon("xmark", closeContentDescription, size = 20.dp) }
        }
    }
}
