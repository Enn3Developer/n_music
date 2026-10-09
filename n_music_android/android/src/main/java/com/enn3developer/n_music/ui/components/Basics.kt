package com.enn3developer.n_music.ui.components

import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Text
import androidx.compose.material3.ripple
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.shadow.Shadow
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** An icon of the design's set at [size], in the content colour unless [tint] says otherwise. */
@Composable
fun NIcon(
    icon: ImageVector,
    modifier: Modifier = Modifier,
    size: Dp = 24.dp,
    tint: Color = LocalContentColor.current,
    contentDescription: String? = null,
) {
    Icon(icon, contentDescription, modifier.size(size), tint = tint)
}

/**
 * A round touch target of [size] holding [icon]; [container] fills a circle of [containerSize]
 * behind it, as the player's toggles do when they are on.
 */
@Composable
fun NIconButton(
    icon: ImageVector,
    contentDescription: String?,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    size: Dp = 48.dp,
    iconSize: Dp = 24.dp,
    tint: Color = LocalContentColor.current,
    enabled: Boolean = true,
) {
    Box(
        modifier
            .size(size)
            .clip(CircleShape)
            .clickable(
                enabled = enabled,
                role = Role.Button,
                interactionSource = remember { MutableInteractionSource() },
                indication = ripple(),
                onClick = onClick,
            )
            .semantics { if (contentDescription != null) this.contentDescription = contentDescription },
        contentAlignment = Alignment.Center,
    ) {
        NIcon(icon, size = iconSize, tint = tint)
    }
}

/** The soft shadow under floating surfaces: `0 6px 18px` in the theme's shadow colour. */
fun Modifier.floating(shape: Shape, color: Color, radius: Dp = 18.dp, offset: Dp = 6.dp) =
    dropShadow(shape, Shadow(radius = radius, color = color, offset = DpOffset(0.dp, offset)))

/** The theme's soft shadow under floating surfaces. */
@Composable
fun Modifier.floating(shape: Shape): Modifier = floating(shape, colors.shadow)

/**
 * The background fading out the list under floating controls, from transparent at the top to
 * the page's colour at [solidFrom] of its height.
 */
@Composable
fun BottomFade(height: Dp, modifier: Modifier = Modifier, solidFrom: Float = 0.72f) {
    val background = colors.background
    Box(
        modifier
            .fillMaxWidth()
            .height(height)
            .background(
                Brush.verticalGradient(0f to background.copy(alpha = 0f), solidFrom to background)
            )
    )
}

private val BarEasing = CubicBezierEasing(0.42f, 0f, 0.58f, 1f)

/**
 * The three bars of the playing track, rising and falling out of step while [animate] is on;
 * still while paused.
 */
@Composable
fun PlayingBars(
    modifier: Modifier = Modifier,
    color: Color = LocalContentColor.current,
    height: Dp = 14.dp,
    animate: Boolean = true,
) {
    val phase by if (animate) {
        rememberInfiniteTransition(label = "bars").animateFloat(
            initialValue = 0f,
            targetValue = 1f,
            animationSpec = infiniteRepeatable(tween(1100, easing = LinearEasing), RepeatMode.Restart),
            label = "phase",
        )
    } else {
        remember { mutableFloatStateOf(0.35f) }
    }
    Canvas(modifier.size(width = 13.dp, height = height)) {
        val bar = 3.dp.toPx()
        val gap = 2.dp.toPx()
        // CSS's negative delays: the second and third bars run 0.45 s and 0.8 s ahead.
        for ((index, offset) in listOf(0f, 0.45f / 1.1f, 0.8f / 1.1f).withIndex()) {
            val t = (phase + offset) % 1f
            val scale = if (t < 0.5f) {
                0.3f + 0.7f * BarEasing.transform(t / 0.5f)
            } else {
                1f - 0.7f * BarEasing.transform((t - 0.5f) / 0.5f)
            }
            val barHeight = size.height * scale
            drawRoundRect(
                color = color,
                topLeft = Offset(index * (bar + gap), size.height - barHeight),
                size = Size(bar, barHeight),
                cornerRadius = CornerRadius(2.dp.toPx()),
            )
        }
    }
}

/** A row or tile's click, with a long press when [onLongClick] is set. */
@Composable
fun Modifier.tappable(
    onClick: () -> Unit,
    onLongClick: (() -> Unit)? = null,
    role: Role = Role.Button,
): Modifier = if (onLongClick == null) {
    clickable(role = role, onClick = onClick)
} else {
    combinedClickable(
        role = role,
        onClick = onClick,
        onLongClick = onLongClick,
    )
}

/** Takes no touches and hides from accessibility services, as dimmed controls do. */
fun Modifier.inert(): Modifier = clearAndSetSemantics {}.pointerInput(Unit) {
    awaitPointerEventScope {
        while (true) {
            awaitPointerEvent(PointerEventPass.Initial).changes.forEach { it.consume() }
        }
    }
}

/** A small fact on a [background] chip, like a track's format or plays. */
@Composable
fun InfoChip(label: String, background: Color, modifier: Modifier = Modifier) {
    Box(
        modifier
            .height(28.dp)
            .background(background, RoundedCornerShape(8.dp))
            .padding(horizontal = 10.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(label, style = text(12, FontWeight.SemiBold, tabular = true), color = colors.onSurfaceVariant, maxLines = 1)
    }
}
