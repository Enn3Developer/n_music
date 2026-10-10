package com.enn3developer.n_music.ui.components

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibilityScope
import androidx.compose.animation.BoundsTransform
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.SharedTransitionLayout
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.VisibilityThreshold
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import com.enn3developer.n_music.ViewMode
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.delayed
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch

/** How long after the first cover the lowest one leaves, so they go top first. */
private const val COVER_STAGGER_MS = 40L

/** When the grid's captions fade in: as their covers are about 80% of the way there. */
private const val CAPTIONS_IN_MS = 125L

/** When the list's text comes back: as its covers are within about 10 dp of their places. */
private const val ROWS_IN_MS = 140L

/**
 * Where covers travel as a list and a grid swap: the shared transition they sit in, the view a
 * cover shows in, and how tall the views are, which sets when each cover leaves.
 */
class CoverTravel internal constructor(
    internal val transition: SharedTransitionScope,
    internal val view: AnimatedVisibilityScope,
    private val height: () -> Float,
) {
    internal val bounds = BoundsTransform { initial, _: Rect ->
        val depth = if (height() > 0f) (initial.top / height()).coerceIn(0f, 1f) else 0f
        NMotion.spatialDefault(Rect.VisibilityThreshold).delayed((depth * COVER_STAGGER_MS).toLong())
    }
}

/** The covers' travel while the list and the grid swap; none elsewhere, where covers stay put. */
val LocalCoverTravel = compositionLocalOf<CoverTravel?> { null }

/** A cover's way to the one of the same key in the other view. */
class CoverWay internal constructor(
    internal val travel: CoverTravel,
    internal val state: SharedTransitionScope.SharedContentState,
)

/** The way of the cover with [key] while the list and the grid swap, or `null` where they don't. */
@Composable
fun rememberCoverWay(key: Any): CoverWay? {
    val travel = LocalCoverTravel.current ?: return null
    val state = with(travel.transition) { rememberSharedContentState(key) }
    return remember(travel, state) { CoverWay(travel, state) }
}

/** Lets this cover travel along [way], when there is one. Put it before the cover's size. */
fun Modifier.coverWay(way: CoverWay?): Modifier = if (way == null) {
    this
} else {
    with(way.travel.transition) { sharedElement(way.state, way.travel.view, boundsTransform = way.travel.bounds) }
}

/** The item a list and a grid start from as they swap: the first one the other view showed. */
class FirstItem {
    var index = 0
}

/** Keeps [first] at [index], the first item the view showing has scrolled to. */
@Composable
fun FollowFirst(first: FirstItem, index: () -> Int) {
    val latest by rememberUpdatedState(index)
    LaunchedEffect(first) { snapshotFlow { latest() }.collect { first.index = it } }
}

/**
 * How much of each view's text shows: the leaving view's fades out at once, the coming view's
 * fades in once its covers are nearly there.
 */
private class TextFade(view: ViewMode) {
    private val alphas = ViewMode.entries.associateWith { Animatable(if (it == view) 1f else 0f) }
    private var swappedAt = 0L

    fun alpha(mode: ViewMode): Float = alphas.getValue(mode).value

    suspend fun swapTo(view: ViewMode) {
        coroutineScope {
            for ((mode, alpha) in alphas) {
                if (mode != view) launch { alpha.animateTo(0f, NMotion.effectsFast()) }
            }
            val now = withFrameNanos { it }
            val since = if (swappedAt == 0L) Long.MAX_VALUE else (now - swappedAt) / 1_000_000
            swappedAt = now
            // A swap back mid-change sends each cover back from where it is, so its text needn't
            // wait for a whole trip.
            val textIn = minOf(if (view == ViewMode.GRID) CAPTIONS_IN_MS else ROWS_IN_MS, since)
            alphas.getValue(view).animateTo(1f, NMotion.effectsDefault<Float>().delayed(textIn))
        }
    }
}

/**
 * A list and a grid of the same items, swapping as [view] changes: each cover travels to its
 * place in the other view, the leaving view's text fades out first and the new one's fades in
 * once its covers are nearly there. [content] shows [view] starting from the item [FirstItem]
 * holds, and keeps it up to date as it scrolls.
 */
@Composable
fun ViewSwap(view: ViewMode, modifier: Modifier = Modifier, content: @Composable (ViewMode, FirstItem) -> Unit) {
    val first = remember { FirstItem() }
    var height by remember { mutableFloatStateOf(0f) }
    val fade = remember { TextFade(view) }
    LaunchedEffect(view) { fade.swapTo(view) }
    SharedTransitionLayout(modifier.onSizeChanged { height = it.height.toFloat() }) {
        AnimatedContent(
            view,
            // The text fades on its own springs, which a swap back mid-change doesn't skip.
            transitionSpec = { EnterTransition.None togetherWith ExitTransition.KeepUntilTransitionsFinished },
            label = "view",
        ) { mode ->
            val travel = remember(this) { CoverTravel(this@SharedTransitionLayout, this) { height } }
            CompositionLocalProvider(LocalCoverTravel provides travel) {
                Box(Modifier.fillMaxSize().graphicsLayer { alpha = fade.alpha(mode) }) { content(mode, first) }
            }
        }
    }
}
