package com.enn3developer.n_music.ui.components

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.togetherWith
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.AnimatedVisibilityScope
import androidx.compose.animation.ContentTransform
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.LocalPageMargins
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.delayed
import com.enn3developer.n_music.ui.theme.text

/**
 * The bar standing in for a page's own while selecting: Stop selecting, how many are picked,
 * and Select all, which picks the [total] tracks of the list.
 */
@Composable
fun SelectionBar(count: Int, total: Int, onClose: () -> Unit, onSelectAll: () -> Unit, modifier: Modifier = Modifier) {
    val margins = LocalPageMargins.current
    Box(
        modifier
            .windowInsetsPadding(WindowInsets.statusBars)
            .padding(start = margins.start, end = margins.end, top = 8.dp, bottom = 8.dp)
    ) {
        SelectionBarContent(
            count,
            total,
            onClose,
            onSelectAll,
            Modifier.background(colors.secondaryContainer, RoundedCornerShape(28.dp)),
        )
    }
}

/** What the selection bar holds, on whatever pill holds it. */
@Composable
fun SelectionBarContent(count: Int, total: Int, onClose: () -> Unit, onSelectAll: () -> Unit, modifier: Modifier = Modifier) {
    Row(
        modifier
            .fillMaxWidth()
            .height(56.dp)
            .padding(horizontal = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CompositionLocalProvider(LocalContentColor provides colors.onSecondaryContainer) {
            NIconButton(NIcons.Close, stringResource(R.string.stop_selecting), onClose)
            RollingText(
                pluralStringResource(R.plurals.selected_count, count, formatCount(count)),
                count,
                text(18, FontWeight.Bold, tabular = true),
                colors.onSecondaryContainer,
                Modifier
                    .weight(1f)
                    .padding(start = 8.dp),
            )
            NIconButton(NIcons.SelectAll, stringResource(R.string.select_all, formatCount(total)), onSelectAll)
        }
    }
}

/**
 * [value] as text whose characters roll as it changes: up while [order] grows, down while it
 * shrinks. Characters that stay the same hold still.
 */
@Composable
fun RollingText(value: String, order: Int, style: TextStyle, color: Color, modifier: Modifier = Modifier) {
    // The last order seen, kept outside snapshots: reading it decides a roll's direction once.
    val last = remember { intArrayOf(order) }
    val up = order >= last[0]
    last[0] = order
    Row(modifier) {
        value.forEachIndexed { index, character ->
            AnimatedContent(
                character,
                transitionSpec = { roll(up) },
                label = "roll$index",
            ) { shown ->
                Text(shown.toString(), style = style, color = color, maxLines = 1)
            }
        }
    }
}

private fun roll(up: Boolean): ContentTransform {
    val sign = if (up) 1 else -1
    return ContentTransform(
        slideInVertically(NMotion.spatialDefault()) { sign * it } + fadeIn(NMotion.effectsDefault()),
        slideOutVertically(NMotion.spatialDefault()) { -sign * it } + fadeOut(NMotion.effectsFast()),
        sizeTransform = SizeTransform(clip = true),
    )
}

/** How long after selecting starts Play next rises, once the mini player has started fading. */
private const val ACTIONS_IN_MS = 170L

/** How long after Play next the other actions rise. */
private const val ACTIONS_FOLLOW_MS = 60L

/** How far the actions rise as they come and drop as they leave. */
private val ACTIONS_RISE = 16.dp

/**
 * What can be done with the picked tracks, in place of the mini player: Play next, Add to queue
 * and Add to playlist. They rise into place, Play next first and the others a little after. The
 * pressed one widens as the others give way, and once it is let go the others leave before it
 * does.
 */
@Composable
fun AnimatedVisibilityScope.SelectionActions(
    count: Int,
    onPlayNext: () -> Unit,
    onQueue: () -> Unit,
    onAddToPlaylist: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var pressed by remember { mutableIntStateOf(-1) }
    val density = LocalDensity.current
    val rise = with(density) { ACTIONS_RISE.roundToPx() }
    val description = pluralStringResource(R.plurals.act_on, count, formatCount(count))
    val labels = listOf(
        stringResource(R.string.play_next),
        stringResource(R.string.add_to_queue),
        stringResource(R.string.add_to_playlist),
    )
    // One size for all three: the design's, or on a phone too narrow for it, the largest that
    // fits the longest label in its third of the bar, rather than cutting labels short.
    val measurer = rememberTextMeasurer()
    var width by remember { mutableIntStateOf(0) }
    val size = remember(labels, width, density) {
        val widest = labels.maxOf { measurer.measure(it, text(ACTION_TEXT, FontWeight.Bold), maxLines = 1).size.width }
        val room = with(density) { (width - 2 * ACTIONS_GAP.toPx()) / 3 - 2 * ACTION_PADDING.toPx() }
        if (width == 0 || widest <= room) ACTION_TEXT.toFloat() else (ACTION_TEXT * room / widest).coerceAtLeast(11f)
    }
    PressGroup(
        3,
        modifier
            .onSizeChanged { width = it.width }
            .semantics { contentDescription = description },
        gap = ACTIONS_GAP,
    ) { index, interaction, weight ->
        val label = labels[index]
        val action = when (index) {
            0 -> onPlayNext
            1 -> onQueue
            else -> onAddToPlaylist
        }
        val shape = RoundedCornerShape(24.dp)
        // Play next comes first; the rest follow it in.
        val lead = if (index == 0) ACTIONS_IN_MS else ACTIONS_IN_MS + ACTIONS_FOLLOW_MS
        // The rest leave first; the pressed one follows them out.
        val lag = if (index == pressed) NMotion.STAGGER_MS else 0L
        ActionButton(
            label,
            size,
            interaction,
            onClick = {
                pressed = index
                action()
            },
            filled = index == 0,
            modifier = weight
                .animateEnterExit(
                    enter = slideInVertically(NMotion.spatialDefault<IntOffset>().delayed(lead)) { rise } +
                        fadeIn(NMotion.effectsDefault<Float>().delayed(lead)),
                    exit = slideOutVertically(NMotion.spatialDefault<IntOffset>().delayed(lag)) { rise } +
                        fadeOut(NMotion.effectsFast<Float>().delayed(lag)),
                )
                .height(48.dp)
                .floating(shape),
        )
    }
}

/** The actions' labels, in sp, where the bar is wide enough for them. */
private const val ACTION_TEXT = 14

/** Between the actions. */
private val ACTIONS_GAP = 8.dp

/** Either side of an action's label. */
private val ACTION_PADDING = 12.dp

/** One of the actions, its [label] [size] sp. */
@Composable
private fun ActionButton(
    label: String,
    size: Float,
    interaction: MutableInteractionSource,
    onClick: () -> Unit,
    filled: Boolean,
    modifier: Modifier,
) {
    ButtonSurface(
        onClick = onClick,
        shape = RoundedCornerShape(24.dp),
        container = if (filled) colors.primaryContainer else colors.secondaryContainer,
        content = if (filled) colors.onPrimaryContainer else colors.onSecondaryContainer,
        modifier = modifier,
        interactionSource = interaction,
        padding = PaddingValues(horizontal = ACTION_PADDING),
    ) {
        Text(label, style = text(size, FontWeight.Bold), maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

/** How far a bar and the selection bar drift as they swap. */
private val BAR_DRIFT = 6.dp

/** [BAR_DRIFT] in pixels, for [barSwap]. */
@Composable
fun barDrift(): Int = with(LocalDensity.current) { BAR_DRIFT.roundToPx() }

/**
 * A bar giving way to the selection bar, or back: the old one leaves first, the new one follows
 * 90 ms later, and both drift [drift] px, up as selecting starts and down as it ends.
 */
fun AnimatedContentTransitionScope<Boolean>.barSwap(selecting: Boolean, drift: Int): ContentTransform {
    val way = if (selecting) -1 else 1
    val lead = NMotion.STAGGER_MS
    return (
        slideInVertically(NMotion.spatialDefault<IntOffset>().delayed(lead + 90)) { -way * drift } +
            fadeIn(NMotion.effectsDefault<Float>().delayed(lead + 90))
        ).togetherWith(
        slideOutVertically(NMotion.spatialDefault<IntOffset>().delayed(lead)) { way * drift } +
            fadeOut(NMotion.effectsFast<Float>().delayed(lead))
    ) using SizeTransform(clip = false)
}
