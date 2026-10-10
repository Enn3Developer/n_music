package com.enn3developer.n_music.ui.components

import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/**
 * A dialog over the whole app, on its scrim, centred in the room the keyboard leaves. It grows in
 * when it first shows and while [open], and fades out once [open] turns off, then calls
 * [onGone]. The scrim and back ask to close it through [onDismissRequest].
 */
@Composable
fun DialogFrame(
    open: Boolean,
    title: String,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
    buttons: @Composable RowScope.() -> Unit,
    content: @Composable ColumnScope.() -> Unit,
) {
    val shown = remember { Animatable(0f) }
    val gone by rememberUpdatedState(onGone)
    LaunchedEffect(open) {
        if (open) {
            shown.animateTo(1f, NMotion.spatialDefault(0.001f))
        } else {
            shown.animateTo(0f, NMotion.effectsFast(0.001f))
            gone()
        }
    }
    BackHandler(open) { onDismissRequest() }
    Box(Modifier.fillMaxSize()) {
        val close = stringResource(R.string.dismiss_dialog)
        Box(
            Modifier
                .fillMaxSize()
                .graphicsLayer { alpha = shown.value.coerceIn(0f, 1f) }
                .background(colors.scrim)
                .clickable(interactionSource = null, indication = null, onClick = onDismissRequest)
                .semantics { contentDescription = close }
        )
        Column(
            Modifier
                .align(Alignment.Center)
                .windowInsetsPadding(WindowInsets.safeDrawing.union(WindowInsets.ime))
                .padding(24.dp)
                .widthIn(max = 296.dp)
                .fillMaxWidth()
                .graphicsLayer {
                    val value = shown.value
                    alpha = value.coerceIn(0f, 1f)
                    scaleX = 0.9f + 0.1f * value
                    scaleY = 0.9f + 0.1f * value
                }
                .background(colors.surfaceHigh, RoundedCornerShape(28.dp))
                .semantics { paneTitle = title }
                .padding(24.dp),
        ) {
            Text(title, style = text(24, FontWeight.Bold, lineHeight = 32.sp), color = colors.onSurface)
            content()
            Row(
                Modifier
                    .fillMaxWidth()
                    .padding(top = 24.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
            ) { buttons() }
        }
    }
}

/** A dialog's way out: Cancel, in the primary colour. */
@Composable
fun DialogCancel(onClick: () -> Unit) {
    TextAction(stringResource(R.string.cancel), onClick, padding = PaddingValues(horizontal = 14.dp))
}

/** A dialog's confirming button, filled; [danger] for one that deletes or removes. */
@Composable
fun DialogConfirm(label: String, onClick: () -> Unit, enabled: Boolean = true, danger: Boolean = false) {
    PillButton(
        label,
        onClick,
        style = if (danger) PillStyle.ERROR else PillStyle.FILLED,
        height = 40.dp,
        textStyle = text(14, FontWeight.Bold),
        padding = PaddingValues(horizontal = 18.dp),
        enabled = enabled,
    )
}

/**
 * A text field outlined as the design draws it: its label sits on the outline, which thickens in
 * the primary colour while the field has focus, on [fill], the colour around it; [helper]
 * explains it underneath, in the error colour along with the outline while [error]. [focus]
 * takes the focus, and with it the keyboard, as it first shows. Without [onDone], the keyboard's
 * Done key closes the keyboard. A [secret] field, for a password, hides what it holds.
 */
@Composable
fun OutlinedField(
    value: String,
    onValueChange: (String) -> Unit,
    label: String,
    modifier: Modifier = Modifier,
    fill: Color = colors.surfaceHigh,
    helper: String? = null,
    error: Boolean = false,
    focus: Boolean = false,
    keyboardType: KeyboardType = KeyboardType.Text,
    imeAction: ImeAction = ImeAction.Done,
    onDone: (() -> Unit)? = null,
    secret: Boolean = false,
) {
    var focused by remember { mutableStateOf(false) }
    val requester = remember { FocusRequester() }
    if (focus) {
        LaunchedEffect(Unit) { requester.requestFocus() }
    }
    val edge = when {
        error -> colors.error
        focused -> colors.primary
        else -> colors.outline
    }
    Column(modifier.fillMaxWidth()) {
        Box(
            Modifier
                .fillMaxWidth()
                .height(56.dp)
        ) {
            Box(
                Modifier
                    .matchParentSize()
                    .border(if (focused) 2.dp else 1.dp, edge, RoundedCornerShape(12.dp))
            )
            BasicTextField(
                value,
                onValueChange,
                singleLine = true,
                textStyle = text(16).copy(color = colors.onSurface),
                cursorBrush = SolidColor(colors.primary),
                keyboardOptions = KeyboardOptions(
                    // Addresses and passwords keep their case.
                    capitalization = if (keyboardType == KeyboardType.Uri || secret) {
                        KeyboardCapitalization.None
                    } else {
                        KeyboardCapitalization.Sentences
                    },
                    autoCorrectEnabled = keyboardType != KeyboardType.Uri && !secret,
                    keyboardType = if (secret) KeyboardType.Password else keyboardType,
                    imeAction = imeAction,
                ),
                visualTransformation = if (secret) PasswordVisualTransformation() else VisualTransformation.None,
                keyboardActions = KeyboardActions(
                    onDone = { if (onDone != null) onDone() else defaultKeyboardAction(ImeAction.Done) },
                    onGo = { if (onDone != null) onDone() else defaultKeyboardAction(ImeAction.Go) },
                ),
                modifier = Modifier
                    .align(Alignment.CenterStart)
                    .fillMaxWidth()
                    .padding(horizontal = 14.dp)
                    .focusRequester(requester)
                    .onFocusChanged { focused = it.isFocused },
            )
            // Drawn after the outline, the label covers it with the colour around, as if cut out.
            Text(
                label,
                style = text(12, FontWeight.SemiBold),
                color = when {
                    error -> colors.error
                    focused -> colors.primary
                    else -> colors.onSurfaceVariant
                },
                modifier = Modifier
                    .offset(x = 10.dp, y = (-8).dp)
                    .background(fill)
                    .padding(horizontal = 4.dp),
            )
        }
        if (helper != null) {
            Text(
                helper,
                style = text(12, lineHeight = 16.sp),
                color = if (error) colors.error else colors.onSurfaceVariant,
                modifier = Modifier.padding(start = 4.dp, end = 4.dp, top = 6.dp),
            )
        }
    }
}
