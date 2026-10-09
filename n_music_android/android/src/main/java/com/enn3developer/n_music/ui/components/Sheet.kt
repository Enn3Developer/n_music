package com.enn3developer.n_music.ui.components

import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitVerticalTouchSlopOrCancellation
import androidx.compose.foundation.gestures.verticalDrag
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.windowInsetsBottomHeight
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.layout
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.isTraversalGroup
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.NShapes
import com.enn3developer.n_music.ui.theme.colors
import kotlinx.coroutines.launch
import kotlin.math.roundToInt

/** How far a sheet hangs past the screen's bottom edge, so its overshoot never shows a gap. */
private val Overhang = 24.dp

/** Below this speed, letting go is not a flick. */
private val FlickSpeed = 400.dp

/** Where a sheet is: 0 below the screen, 1 in place, a little past 1 while it settles. */
private const val VISIBLE = 0.001f

/**
 * A bottom sheet over the whole app, on its scrim. It rises when it first shows and while
 * [open], and leaves once [open] turns off, then calls [onGone]. Dragging the handle or the
 * [header] down moves it with the finger; a flick, or a drag past a third of it, asks to close it
 * through [onDismissRequest], as the scrim and back do. A [tall] sheet reaches up to just under
 * the status bar; a short one leaves [end] under its content, above the gesture area.
 */
@Composable
fun SheetFrame(
    open: Boolean,
    title: String,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
    modifier: Modifier = Modifier,
    tall: Boolean = false,
    end: Dp = 12.dp,
    header: @Composable ColumnScope.() -> Unit,
    content: @Composable ColumnScope.() -> Unit,
) {
    val scope = rememberCoroutineScope()
    val shown = remember { Animatable(0f) }
    val scrim = remember { Animatable(0f) }
    // How far the sheet travels between its place and below the screen, in pixels.
    val travel = remember { floatArrayOf(1f) }
    var closing by remember { mutableStateOf(false) }
    val gone by rememberUpdatedState(onGone)
    val isOpen by rememberUpdatedState(open)
    val dismiss by rememberUpdatedState(onDismissRequest)

    /** Lowers the sheet from where it is, at [velocity] of its travel a second downwards. */
    fun close(velocity: Float) {
        closing = true
        scope.launch {
            launch { scrim.animateTo(0f, NMotion.effectsDefault()) }
            shown.animateTo(0f, NMotion.throwing(shown.value, velocity, VISIBLE), -velocity)
            gone()
        }
    }

    /** Puts the sheet back in place, at [velocity] of its travel a second upwards. */
    fun settle(velocity: Float) {
        scope.launch {
            launch { scrim.animateTo(1f, NMotion.effectsDefault()) }
            shown.animateTo(1f, NMotion.spatialDefault(VISIBLE), velocity)
        }
    }

    LaunchedEffect(open) {
        if (open) {
            closing = false
            settle(0f)
        } else if (!closing) {
            close(0f)
        }
    }
    BackHandler(open) { onDismissRequest() }

    Box(Modifier.fillMaxSize()) {
        val close = stringResource(R.string.dismiss_sheet)
        Box(
            Modifier
                .fillMaxSize()
                .graphicsLayer { alpha = scrim.value * shown.value.coerceIn(0f, 1f) }
                .background(colors.scrim)
                .clickable(interactionSource = null, indication = null, onClick = onDismissRequest)
                .semantics { contentDescription = close }
        )
        Column(
            modifier
                .align(Alignment.BottomCenter)
                .windowInsetsPadding(WindowInsets.statusBars)
                .padding(top = 8.dp)
                .layout { measurable, constraints ->
                    val overhang = Overhang.roundToPx()
                    val placeable = measurable.measure(
                        if (tall) {
                            constraints.copy(
                                minHeight = constraints.maxHeight + overhang,
                                maxHeight = constraints.maxHeight + overhang,
                            )
                        } else {
                            constraints.copy(minHeight = 0, maxHeight = constraints.maxHeight + overhang)
                        }
                    )
                    // The overhang lies below the space the sheet takes, past the screen's edge.
                    val height = placeable.height - overhang
                    travel[0] = height.toFloat().coerceAtLeast(1f)
                    layout(placeable.width, height) {
                        placeable.place(0, ((1f - shown.value) * height).roundToInt())
                    }
                }
                .background(colors.surfaceLow, NShapes.sheet)
                .semantics {
                    paneTitle = title
                    isTraversalGroup = true
                },
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .pointerInput(Unit) {
                        val flick = FlickSpeed.toPx()
                        awaitEachGesture {
                            val down = awaitFirstDown(requireUnconsumed = false)
                            // Touching the sheet while it moves catches it under the finger.
                            scope.launch { shown.stop() }
                            val start = awaitVerticalTouchSlopOrCancellation(down.id) { change, _ ->
                                change.consume()
                            }
                            if (start == null) {
                                if (closing) close(0f) else if (shown.value != 1f) settle(0f)
                                return@awaitEachGesture
                            }
                            val tracker = VelocityTracker()
                            var dragged = 0f
                            fun follow(change: PointerInputChange) {
                                val delta = change.positionChange().y
                                dragged += delta
                                tracker.addPosition(change.uptimeMillis, Offset(0f, dragged))
                                val value = (shown.value - delta / travel[0]).coerceAtMost(1f)
                                scope.launch { shown.snapTo(value) }
                                change.consume()
                            }
                            follow(start)
                            verticalDrag(start.id, ::follow)
                            val speed = tracker.calculateVelocity().y
                            val past = 1f - shown.value > 1f / 3f
                            if (closing || speed > flick || (past && speed > -flick)) {
                                close((speed / travel[0]).coerceAtLeast(0f))
                                if (isOpen) dismiss()
                            } else {
                                settle(-speed / travel[0])
                            }
                        }
                    },
            ) {
                Box(
                    Modifier
                        .fillMaxWidth()
                        .padding(top = if (tall) 12.dp else 16.dp, bottom = if (tall) 6.dp else 12.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    Box(
                        Modifier
                            .size(32.dp, 4.dp)
                            .background(colors.onSurfaceVariant.copy(alpha = 0.45f), RoundedCornerShape(2.dp))
                    )
                }
                header()
            }
            // A short sheet scrolls when the screen is too low for it.
            Column(
                if (tall) Modifier.weight(1f) else Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState())
            ) { content() }
            if (!tall) Spacer(Modifier.height(end))
            // The keyboard lifts the sheet's bottom, or shortens a tall sheet's content.
            Spacer(Modifier.windowInsetsBottomHeight(WindowInsets.navigationBars.union(WindowInsets.ime)))
            Spacer(Modifier.height(Overhang))
        }
    }
}
