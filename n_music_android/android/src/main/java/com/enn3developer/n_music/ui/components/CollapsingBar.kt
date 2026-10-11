package com.enn3developer.n_music.ui.components

import androidx.compose.animation.core.animate
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.layout.layout
import androidx.compose.ui.unit.Velocity
import com.enn3developer.n_music.ui.theme.NMotion
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlin.math.roundToInt

/**
 * A bar over lists that leaves as they scroll on and comes back as soon as they scroll back, as
 * Material's top app bar does when it enters always. [connection] goes on what holds the lists,
 * and [collapsing] on the bar. Let go of halfway, the bar goes on the way it was going.
 */
@Stable
class CollapsingBar internal constructor(private val scope: CoroutineScope) {
    /** How much of the bar has left, in pixels: 0 while it shows, its height once it is gone. */
    internal var offset by mutableFloatStateOf(0f)
        private set

    /** The bar's height, as it was last laid out. */
    internal var height = 0f

    /** It stays, as the selection's bar must. */
    internal var pinned = false

    /** The scroll going on moved the bar, which settles once the scroll ends. */
    private var moved = false

    /** Its last move took it away, rather than back. */
    private var leaving = false
    private var sliding: Job? = null

    val connection: NestedScrollConnection = object : NestedScrollConnection {
        override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
            if (pinned || available.y == 0f) return Offset.Zero
            sliding?.cancel()
            // The bar moves first, so the list follows the finger as the room under it changes.
            val before = offset
            offset = (before - available.y).coerceIn(0f, height)
            if (offset != before) {
                moved = true
                leaving = offset > before
            }
            return Offset(0f, before - offset)
        }

        override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
            if (moved && !pinned && offset > 0f && offset < height) slide(if (leaving) height else 0f)
            moved = false
            return Velocity.Zero
        }
    }

    /** Brings the bar back. */
    fun show() = slide(0f)

    private fun slide(to: Float) {
        sliding?.cancel()
        if (offset == to) return
        sliding = scope.launch {
            // As Material's bar settles: no bounce, which would push the lists past their place.
            animate(offset, to, animationSpec = NMotion.effectsDefault()) { value, _ -> offset = value }
        }
    }
}

/** A [CollapsingBar] that comes back and stays while [pinned]. */
@Composable
fun rememberCollapsingBar(pinned: Boolean = false): CollapsingBar {
    val scope = rememberCoroutineScope()
    val bar = remember(scope) { CollapsingBar(scope) }
    LaunchedEffect(bar, pinned) {
        bar.pinned = pinned
        if (pinned) bar.show()
    }
    return bar
}

/**
 * Lays the bar out in the room [bar] leaves it: the bar slides up out of it as it leaves, fading,
 * and the room closes under it.
 */
fun Modifier.collapsing(bar: CollapsingBar): Modifier = clipToBounds().layout { measurable, constraints ->
    val placeable = measurable.measure(constraints.copy(minHeight = 0))
    val height = placeable.height
    bar.height = height.toFloat()
    val shown = height - bar.offset.coerceAtMost(bar.height).roundToInt()
    layout(placeable.width, shown) {
        placeable.placeWithLayer(0, shown - height) { alpha = if (height > 0) shown.toFloat() / height else 1f }
    }
}
