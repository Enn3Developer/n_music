package com.enn3developer.n_music.ui.player

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ripple
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.PlaybackActions
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors

/**
 * How big the player's buttons are, with their icons, and how they share a row: [spread] across
 * it, or [gap] apart.
 */
@Immutable
data class ControlSizes(
    val toggle: Dp,
    val toggleIcon: Dp,
    val skip: Dp,
    val skipRadius: Dp,
    val skipIcon: Dp,
    val playWidth: Dp,
    val playHeight: Dp,
    val playRadius: Dp,
    val playIcon: Dp,
    val spread: Boolean = false,
    val gap: Dp = 6.dp,
) {
    companion object {
        val Phone = ControlSizes(48.dp, 24.dp, 64.dp, 20.dp, 28.dp, 96.dp, 80.dp, 28.dp, 36.dp)
        val Fold = ControlSizes(44.dp, 22.dp, 56.dp, 18.dp, 26.dp, 84.dp, 72.dp, 26.dp, 32.dp, spread = true)
        val Landscape = ControlSizes(48.dp, 24.dp, 60.dp, 20.dp, 26.dp, 92.dp, 68.dp, 26.dp, 32.dp, gap = 8.dp)
        val Pane = ControlSizes(44.dp, 20.dp, 52.dp, 16.dp, 24.dp, 76.dp, 60.dp, 22.dp, 30.dp, spread = true)
    }
}

/** The player's buttons: shuffle, previous, play or pause, next and repeat. */
@Composable
fun PlayerControls(
    playing: Boolean,
    shuffle: Boolean,
    loop: LoopStatus,
    actions: PlaybackActions,
    modifier: Modifier = Modifier,
    sizes: ControlSizes = ControlSizes.Phone,
) {
    Row(
        modifier,
        horizontalArrangement = if (sizes.spread) Arrangement.SpaceBetween else Arrangement.spacedBy(sizes.gap, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        ToggleButton(
            NIcons.Shuffle,
            stringResource(R.string.shuffle),
            stringResource(if (shuffle) R.string.toggle_on else R.string.toggle_off),
            shuffle,
            actions::toggleShuffle,
            sizes,
        )
        SkipButton(NIcons.Previous, stringResource(R.string.previous), actions::previous, sizes)
        PlayPauseButton(playing, actions::togglePause, sizes)
        SkipButton(NIcons.Next, stringResource(R.string.next), actions::next, sizes)
        ToggleButton(
            if (loop == LoopStatus.FILE) NIcons.RepeatOne else NIcons.Repeat,
            stringResource(R.string.repeat),
            stringResource(
                when (loop) {
                    LoopStatus.OFF -> R.string.toggle_off
                    LoopStatus.PLAYLIST -> R.string.repeat_state_all
                    LoopStatus.FILE -> R.string.repeat_state_one
                }
            ),
            loop != LoopStatus.OFF,
            actions::cycleRepeat,
            sizes,
        )
    }
}

/** Shuffle or repeat: a round button, filled while [on]. */
@Composable
private fun ToggleButton(icon: ImageVector, label: String, state: String, on: Boolean, onClick: () -> Unit, sizes: ControlSizes) {
    val fill by animateColorAsState(if (on) colors.secondaryContainer else Color.Transparent, NMotion.effectsDefault(), label = "fill")
    val tint by animateColorAsState(if (on) colors.onSecondaryContainer else colors.onSurfaceVariant, NMotion.effectsDefault(), label = "tint")
    Box(
        Modifier
            .size(sizes.toggle)
            .clip(CircleShape)
            .background(fill)
            .clickable(role = Role.Button, interactionSource = null, indication = ripple(), onClick = onClick)
            .semantics {
                contentDescription = label
                stateDescription = state
            },
        contentAlignment = Alignment.Center,
    ) {
        NIcon(icon, size = sizes.toggleIcon, tint = tint)
    }
}

/** Previous or next: a square-ish button whose corners square up further while pressed. */
@Composable
private fun SkipButton(icon: ImageVector, label: String, onClick: () -> Unit, sizes: ControlSizes) {
    SqueezeButton(
        width = sizes.skip,
        height = sizes.skip,
        radius = sizes.skipRadius,
        pressedRadius = sizes.skipRadius - 8.dp,
        fill = colors.surfaceHigh,
        press = colors.onSurface,
        pressAlpha = 0.1f,
        label = label,
        onClick = onClick,
    ) {
        NIcon(icon, size = sizes.skipIcon, tint = colors.onSurface)
    }
}

/**
 * Play or pause: rounded while playing, a pill while paused, and squarer while pressed. Its two
 * bars fold into a triangle as it pauses.
 */
@Composable
private fun PlayPauseButton(playing: Boolean, onClick: () -> Unit, sizes: ControlSizes) {
    // 0 shows the bars of Pause, 1 the triangle of Play.
    val morph = remember { Animatable(if (playing) 0f else 1f) }
    LaunchedEffect(playing) { morph.animateTo(if (playing) 0f else 1f, NMotion.effectsDefault()) }
    val tint = colors.onPrimaryContainer
    val path = remember { Path() }
    SqueezeButton(
        width = sizes.playWidth,
        height = sizes.playHeight,
        radius = if (playing) sizes.playRadius else sizes.playHeight / 2,
        pressedRadius = sizes.playRadius - 12.dp,
        fill = colors.primaryContainer,
        press = colors.onPrimaryContainer,
        pressAlpha = 0.12f,
        label = stringResource(if (playing) R.string.pause else R.string.play),
        onClick = onClick,
    ) {
        Canvas(Modifier.size(sizes.playIcon)) {
            val scale = size.width / 24f
            val t = morph.value
            path.reset()
            for (shape in 0 until 2) {
                for (corner in 0 until 4) {
                    val index = shape * 8 + corner * 2
                    val x = (PauseShape[index] + (PlayShape[index] - PauseShape[index]) * t) * scale
                    val y = (PauseShape[index + 1] + (PlayShape[index + 1] - PauseShape[index + 1]) * t) * scale
                    if (corner == 0) path.moveTo(x, y) else path.lineTo(x, y)
                }
                path.close()
            }
            drawPath(path, tint)
        }
    }
}

/** Pause's two bars and Play's triangle in two halves, as four corners each on the 24 grid. */
private val PauseShape = floatArrayOf(
    6.5f, 5f, 10.5f, 5f, 10.5f, 19f, 6.5f, 19f,
    13.5f, 5f, 17.5f, 5f, 17.5f, 19f, 13.5f, 19f,
)
private val PlayShape = floatArrayOf(
    8f, 5f, 13.5f, 8.5f, 13.5f, 15.5f, 8f, 19f,
    13.5f, 8.5f, 19f, 12f, 19f, 12f, 13.5f, 15.5f,
)

/**
 * A filled button whose corners go from [radius] to [pressedRadius] while pressed, under a
 * [press] wash at [pressAlpha]; letting go springs them back, or to a new [radius].
 */
@Composable
private fun SqueezeButton(
    width: Dp,
    height: Dp,
    radius: Dp,
    pressedRadius: Dp,
    fill: Color,
    press: Color,
    pressAlpha: Float,
    label: String,
    onClick: () -> Unit,
    content: @Composable () -> Unit,
) {
    val interactions = remember { MutableInteractionSource() }
    val pressed by interactions.collectIsPressedAsState()
    val corner = remember { Animatable(radius.value) }
    val wash = remember { Animatable(0f) }
    LaunchedEffect(pressed, radius) {
        if (pressed) {
            corner.animateTo(pressedRadius.value, NMotion.spatialFast())
        } else {
            corner.animateTo(radius.value, NMotion.spatialDefault())
        }
    }
    // The wash comes on as fast as the press and fades out more slowly.
    LaunchedEffect(pressed) {
        wash.animateTo(if (pressed) pressAlpha else 0f, if (pressed) NMotion.effectsFast() else NMotion.effectsDefault())
    }
    Box(
        Modifier
            .size(width, height)
            .graphicsLayer {
                shape = RoundedCornerShape(corner.value.dp)
                clip = true
            }
            .drawBehind {
                drawRect(fill)
                drawRect(press, Offset.Zero, size, wash.value)
            }
            .clickable(interactions, indication = null, role = Role.Button, onClick = onClick)
            .semantics { contentDescription = label },
        contentAlignment = Alignment.Center,
    ) {
        content()
    }
}
