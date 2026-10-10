package com.enn3developer.n_music.ui.components

import android.view.ViewConfiguration
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Indication
import androidx.compose.foundation.IndicationNodeFactory
import androidx.compose.foundation.interaction.FocusInteraction
import androidx.compose.foundation.interaction.HoverInteraction
import androidx.compose.foundation.interaction.InteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.isSpecified
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.drawscope.ContentDrawScope
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.node.DelegatableNode
import androidx.compose.ui.node.DrawModifierNode
import androidx.compose.ui.platform.LocalViewConfiguration
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlin.math.hypot

/** How strong a press shows: 12% of the content colour, as the design's ripples are. */
private const val PRESS_ALPHA = 0.12f

/** How strong a pointer resting over something shows. */
private const val HOVER_ALPHA = 0.08f

/** How strong keyboard focus shows. */
private const val FOCUS_ALPHA = 0.10f

/** How long a tap's ripple takes to cover what was tapped. */
private const val TAP_FILL_MS = 300

/**
 * A fill growing from the finger in step with a long press, covering what is pressed just as
 * the press counts as held. Let go sooner and it finishes as a tap's ripple does on its way out;
 * a scroll taking the finger fades it where it is.
 */
class HoldFill {
    private val grow = Animatable(0f)
    private val alpha = Animatable(0f)
    private var origin by mutableStateOf(Offset.Unspecified)

    /** A press at [at], which counts as held [hold] ms from now. */
    suspend fun press(at: Offset, hold: Long) {
        origin = at
        grow.snapTo(0f)
        coroutineScope {
            launch { alpha.animateTo(PRESS_ALPHA, NMotion.effectsFast()) }
            // On a clock, so it is full just as the hold is.
            grow.animateTo(1f, tween(hold.toInt().coerceAtLeast(1), easing = LinearEasing))
        }
    }

    /** The finger lifting: a tap's ripple finishes as it fades, a hold's fill just fades. */
    suspend fun release() {
        coroutineScope {
            val left = (TAP_FILL_MS * (1f - grow.value)).toInt()
            if (left > 0) launch { grow.animateTo(1f, tween(left, easing = FastOutSlowInEasing)) }
            if (alpha.value < PRESS_ALPHA) alpha.animateTo(PRESS_ALPHA, NMotion.effectsFast())
            alpha.animateTo(0f, NMotion.effectsDefault())
        }
    }

    /** A scroll or another gesture taking the finger: the fill fades where it is. */
    suspend fun cancel() {
        coroutineScope {
            launch { grow.stop() }
            alpha.animateTo(0f, NMotion.effectsFast())
        }
    }

    /** Draws the fill in [color] over what is drawn so far, inside its bounds. */
    fun DrawScope.drawFill(color: Color) {
        val shown = alpha.value
        val from = origin
        if (shown <= 0f || !from.isSpecified) return
        val reach = maxOf(
            maxOf(hypot(from.x, from.y), hypot(size.width - from.x, from.y)),
            maxOf(hypot(from.x, size.height - from.y), hypot(size.width - from.x, size.height - from.y)),
        )
        clipRect { drawCircle(color, reach * grow.value, from, alpha = shown) }
    }
}

/** Draws [fill] in [color] over this and what it holds, inside [shape]. */
fun Modifier.holdFill(fill: HoldFill, color: Color, shape: Shape): Modifier =
    clip(shape).drawWithContent {
        drawContent()
        with(fill) { drawFill(color) }
    }

/**
 * How a row or tile with a long press shows touches: a [HoldFill] in step with the hold, and
 * a wash while a pointer rests over it or it has keyboard focus.
 */
@Composable
fun rememberHoldIndication(): Indication {
    val color = colors.onSurface
    // In a scrolling list a press shows a tap's time after the finger lands, where the hold
    // starts counting.
    val hold = LocalViewConfiguration.current.longPressTimeoutMillis - ViewConfiguration.getTapTimeout()
    return remember(color, hold) { HoldIndication(color, hold) }
}

private class HoldIndication(private val color: Color, private val hold: Long) : IndicationNodeFactory {
    override fun create(interactionSource: InteractionSource): DelegatableNode =
        HoldIndicationNode(interactionSource, color, hold)

    override fun equals(other: Any?): Boolean = other is HoldIndication && other.color == color && other.hold == hold

    override fun hashCode(): Int = 31 * color.hashCode() + hold.hashCode()
}

private class HoldIndicationNode(
    private val interactions: InteractionSource,
    private val color: Color,
    private val hold: Long,
) : Modifier.Node(), DrawModifierNode {
    private val fill = HoldFill()
    private val wash = Animatable(0f)

    override fun onAttach() {
        coroutineScope.launch {
            val hovers = mutableListOf<HoverInteraction.Enter>()
            val focuses = mutableListOf<FocusInteraction.Focus>()
            interactions.interactions.collect { interaction ->
                when (interaction) {
                    is PressInteraction.Press -> launch { fill.press(interaction.pressPosition, hold) }
                    is PressInteraction.Release -> launch { fill.release() }
                    is PressInteraction.Cancel -> launch { fill.cancel() }
                    is HoverInteraction.Enter -> hovers += interaction
                    is HoverInteraction.Exit -> hovers -= interaction.enter
                    is FocusInteraction.Focus -> focuses += interaction
                    is FocusInteraction.Unfocus -> focuses -= interaction.focus
                }
                val target = when {
                    focuses.isNotEmpty() -> FOCUS_ALPHA
                    hovers.isNotEmpty() -> HOVER_ALPHA
                    else -> 0f
                }
                if (target != wash.targetValue) launch { wash.animateTo(target, NMotion.effectsFast()) }
            }
        }
    }

    override fun ContentDrawScope.draw() {
        drawContent()
        val washed = wash.value
        if (washed > 0f) drawRect(color, alpha = washed)
        with(fill) { drawFill(color) }
    }
}
