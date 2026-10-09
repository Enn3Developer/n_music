package com.enn3developer.n_music.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.verticalDrag
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.sizeIn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.grid.LazyGridState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import kotlinx.coroutines.launch
import java.text.Normalizer
import kotlin.math.roundToInt

/** What a fast scroller needs of a lazy list or grid. */
interface Scrolled {
    /** How many items there are. */
    val total: Int

    /** How far down the list is, from 0 at its top to 1 at its end. */
    val progress: Float

    /** The list is longer than the room it has. */
    val overflows: Boolean

    suspend fun jumpTo(index: Int)
}

@Composable
fun rememberScrolled(state: LazyListState): Scrolled = remember(state) {
    object : Scrolled {
        override val total get() = state.layoutInfo.totalItemsCount
        override val progress get() = progress(state.firstVisibleItemIndex, state.layoutInfo.visibleItemsInfo.size, total)
        override val overflows get() = state.canScrollForward || state.canScrollBackward
        override suspend fun jumpTo(index: Int) = state.scrollToItem(index)
    }
}

@Composable
fun rememberScrolled(state: LazyGridState): Scrolled = remember(state) {
    object : Scrolled {
        override val total get() = state.layoutInfo.totalItemsCount
        override val progress get() = progress(state.firstVisibleItemIndex, state.layoutInfo.visibleItemsInfo.size, total)
        override val overflows get() = state.canScrollForward || state.canScrollBackward
        override suspend fun jumpTo(index: Int) = state.scrollToItem(index)
    }
}

private fun progress(first: Int, visible: Int, total: Int): Float {
    val last = total - visible
    return if (last <= 0) 0f else (first.toFloat() / last).coerceIn(0f, 1f)
}

/**
 * The thumb along a long list's end, and while it is dragged, the bubble naming the [section] of
 * the item it is at: a letter, a year. The thumb runs between [top] and [bottom] of the list's
 * height, clear of what floats over the list.
 */
@Composable
fun FastScroller(
    scrolled: Scrolled,
    section: (Int) -> String?,
    modifier: Modifier = Modifier,
    top: Dp = 24.dp,
    bottom: Dp = 24.dp,
) {
    val overflows by remember(scrolled) { derivedStateOf { scrolled.overflows } }
    if (!overflows) return
    val scope = rememberCoroutineScope()
    val density = LocalDensity.current
    var dragging by remember { mutableStateOf(false) }
    // Where the finger holds the thumb, from 0 to 1, while dragging.
    var held by remember { mutableFloatStateOf(0f) }
    val thumbColor by animateColorAsState(
        if (dragging) colors.primary else colors.outline,
        NMotion.effectsFast(),
        label = "thumb",
    )
    BoxWithConstraints(
        modifier
            .fillMaxSize()
            .clearAndSetSemantics {}
    ) {
        val travel = with(density) { (maxHeight - top - bottom - ThumbHeight).toPx() }.coerceAtLeast(1f)
        val start = with(density) { top.toPx() }
        // Read while placing, so scrolling moves the thumb without composing again.
        fun thumbTop() = start + (if (dragging) held else scrolled.progress) * travel
        val label = if (dragging) section(((scrolled.total - 1) * held).roundToInt().coerceAtLeast(0)) else null
        Box(
            Modifier
                .align(Alignment.TopEnd)
                .offset { IntOffset(0, thumbTop().roundToInt()) }
                .padding(end = 3.dp)
                .size(4.dp, ThumbHeight)
                .background(thumbColor, RoundedCornerShape(2.dp))
        )
        // The thumb's touch area, taller than it and a little wider, yet clear of the ⋮ beside it.
        Box(
            Modifier
                .align(Alignment.TopEnd)
                .offset { IntOffset(0, (thumbTop() - 12.dp.toPx()).roundToInt()) }
                .size(20.dp, ThumbHeight + 24.dp)
                .pointerInput(scrolled, travel) {
                    awaitEachGesture {
                        val down = awaitFirstDown()
                        down.consume()
                        held = scrolled.progress
                        dragging = true
                        verticalDrag(down.id) { change ->
                            held = (held + change.positionChange().y / travel).coerceIn(0f, 1f)
                            val index = ((scrolled.total - 1) * held).roundToInt()
                            scope.launch { scrolled.jumpTo(index) }
                            change.consume()
                        }
                        dragging = false
                    }
                }
        )
        AnimatedVisibility(
            label != null,
            Modifier
                .align(Alignment.TopEnd)
                .offset { IntOffset(-20.dp.roundToPx(), (thumbTop() - 14.dp.toPx()).roundToInt()) },
            enter = scaleIn(NMotion.spatialFast(), transformOrigin = TransformOrigin(1f, 1f)) + fadeIn(NMotion.effectsFast()),
            exit = scaleOut(NMotion.spatialFast(), transformOrigin = TransformOrigin(1f, 1f)) + fadeOut(NMotion.effectsFast()),
        ) {
            // The last label stays while the bubble shrinks away.
            val last = remember { arrayOf("") }
            if (label != null) last[0] = label
            val shown = last[0]
            Box(
                Modifier
                    .sizeIn(minWidth = 64.dp, minHeight = 64.dp)
                    .floating(BubbleShape)
                    .background(colors.primaryContainer, BubbleShape)
                    .padding(horizontal = 12.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    shown,
                    style = text(if (shown.length > 2) 22 else 30, FontWeight.ExtraBold),
                    color = colors.onPrimaryContainer,
                    maxLines = 1,
                )
            }
        }
    }
}

private val ThumbHeight = 44.dp

/** A drop pointing at the thumb: round but for its corner beside it. */
private val BubbleShape = RoundedCornerShape(topStart = 32.dp, topEnd = 32.dp, bottomEnd = 4.dp, bottomStart = 32.dp)

/** The letter a name files under: its first letter without accents, `#` for anything else. */
fun sectionLetter(name: String?): String {
    val first = name?.trim()?.firstOrNull() ?: return "#"
    val plain = Normalizer.normalize(first.toString(), Normalizer.Form.NFD).firstOrNull() ?: return "#"
    return if (plain.isLetter()) plain.uppercase() else "#"
}
