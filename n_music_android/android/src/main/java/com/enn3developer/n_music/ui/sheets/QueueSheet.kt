package com.enn3developer.n_music.ui.sheets

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animate
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.drag
import androidx.compose.foundation.gestures.scrollBy
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.shadow.Shadow
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.PointerInputScope
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.isTraversalGroup
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.PlayingFrom
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.core.QueueRow
import com.enn3developer.n_music.core.Seek
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.QueuedItem
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.CoverPlaceholder
import com.enn3developer.n_music.ui.components.MenuItem
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NMenu
import com.enn3developer.n_music.ui.components.PlayingBars
import com.enn3developer.n_music.ui.components.Segmented
import com.enn3developer.n_music.ui.components.SheetClose
import com.enn3developer.n_music.ui.components.SheetFrame
import com.enn3developer.n_music.ui.components.SwipeToRemove
import com.enn3developer.n_music.ui.components.TextAction
import com.enn3developer.n_music.ui.components.inSideSheet
import com.enn3developer.n_music.ui.components.margins
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.formatLength
import com.enn3developer.n_music.ui.player.openOrigin
import com.enn3developer.n_music.ui.player.originName
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.removeQueued
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

/** What the queue asks of playback. */
interface QueueActions {
    fun openOrigin()

    /** Takes the queued items still to play out of the queue, with Undo. */
    fun clearQueued(rows: List<QueueRow>)

    /** Takes queued [row] out of the queue, with Undo. */
    fun remove(row: QueueRow)

    fun setLoop(loop: LoopStatus)

    fun toggleShuffle()

    /** Plays [row] now. */
    fun play(row: QueueRow)

    /** Moves [row] to play right before [before], or last when `null`. */
    fun move(row: QueueRow, current: Boolean, before: ULong?)

    /** Plays [row]'s track again after the current one. */
    fun playAgain(row: QueueRow)
}

/**
 * The play session as it stands: what played, greyed, what plays now, and what plays next, which
 * a drag on its handle reorders. Over it, where it plays from, what happens at its end, and
 * shuffle.
 */
@Composable
fun QueueSheet(open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    LiveQueue(onLeave = onDismissRequest) { entries, current, playing, shuffle, loop, origin, actions ->
        QueueSheet(entries, current, playing, shuffle, loop, origin, actions, open, onDismissRequest, onGone)
    }
}

/** The play session beside a foldable's player, as the sheet shows it, but for shuffle, which the player has. */
@Composable
fun QueuePanel(modifier: Modifier = Modifier) {
    LiveQueue(onLeave = {}) { entries, current, playing, shuffle, loop, origin, actions ->
        QueuePanel(entries, current, playing, shuffle, loop, origin, actions, modifier)
    }
}

/**
 * The play session from the core, and what the queue does with it; [onLeave] runs before it
 * opens the page of what plays.
 */
@Composable
private fun LiveQueue(
    onLeave: () -> Unit,
    content: @Composable (
        entries: List<QueueRow>,
        current: ULong?,
        playing: Boolean,
        shuffle: Boolean,
        loop: LoopStatus,
        origin: String?,
        actions: QueueActions,
    ) -> Unit,
) {
    val app = LocalApp.current
    val resources = LocalResources.current
    val queue by CoreRepository.queue.collectAsStateWithLifecycle()
    val current by CoreRepository.current.collectAsStateWithLifecycle()
    val playing by CoreRepository.playing.collectAsStateWithLifecycle()
    val shuffle by CoreRepository.shuffle.collectAsStateWithLifecycle()
    val loop by CoreRepository.loopStatus.collectAsStateWithLifecycle()
    val origin by PlayingFrom.origin.collectAsStateWithLifecycle()
    val leave by rememberUpdatedState(onLeave)
    val actions = remember(app, resources) {
        object : QueueActions {
            override fun openOrigin() {
                val from = PlayingFrom.origin.value ?: return
                leave()
                app.openOrigin(from)
            }

            override fun clearQueued(rows: List<QueueRow>) {
                val count = rows.size
                app.removeQueued(
                    rows,
                    resources.getQuantityString(R.plurals.cleared_queued, quantity(count), formatCount(count)),
                    resources,
                )
            }

            override fun remove(row: QueueRow) =
                app.removeQueued(listOf(row), resources.getString(R.string.removed_from_queue, row.track.title), resources)

            override fun setLoop(loop: LoopStatus) = CoreRepository.send(Command.SetLoopStatus(loop))

            override fun toggleShuffle() = CoreRepository.send(Command.ToggleShuffle)

            override fun play(row: QueueRow) = CoreRepository.send(Command.Seek(Seek.ToItem(row.item, 0.0)))

            override fun move(row: QueueRow, current: Boolean, before: ULong?) = CoreRepository.send(
                if (current) Command.MoveCurrent(row.item, before) else Command.MoveUpcoming(row.item, before)
            )

            override fun playAgain(row: QueueRow) = CoreRepository.send(Command.Enqueue(listOf(row.track.locator), true))
        }
    }
    content(
        // Rows taken out leave at once, though the core hears of it once their snackbar goes.
        queue.filterNot { app.removals.hides(QueuedItem(it.item)) },
        current?.item,
        playing,
        shuffle,
        loop,
        originName(origin),
        actions,
    )
}

/** The sheet itself, for the session's [entries] with [current] playing. */
@Composable
fun QueueSheet(
    entries: List<QueueRow>,
    current: ULong?,
    playing: Boolean,
    shuffle: Boolean,
    loop: LoopStatus,
    origin: String?,
    actions: QueueActions,
    open: Boolean,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
) {
    val title = stringResource(R.string.queue)
    val index = entries.indexOfFirst { it.item == current }
    val upcoming = entries.drop(index + 1)
    val queued = upcoming.filter { it.queued }
    SheetFrame(
        open, title, onDismissRequest, onGone,
        tall = true,
        header = { QueueHeader(title, queued, upcoming.size, shuffle, loop, origin, actions, panel = false) },
    ) {
        Box(
            Modifier
                .fillMaxWidth()
                .height(1.dp)
                .background(colors.outlineVariant)
        )
        QueueList(entries, current, playing, actions, Modifier.weight(1f))
    }
}

/** The panel itself, for the session's [entries] with [current] playing. */
@Composable
fun QueuePanel(
    entries: List<QueueRow>,
    current: ULong?,
    playing: Boolean,
    shuffle: Boolean,
    loop: LoopStatus,
    origin: String?,
    actions: QueueActions,
    modifier: Modifier = Modifier,
) {
    val title = stringResource(R.string.queue)
    val index = entries.indexOfFirst { it.item == current }
    val upcoming = entries.drop(index + 1)
    val queued = upcoming.filter { it.queued }
    val shape = RoundedCornerShape(28.dp)
    Column(
        modifier
            .clip(shape)
            .background(colors.surfaceLow, shape)
            .semantics {
                paneTitle = title
                isTraversalGroup = true
            }
    ) {
        QueueHeader(title, queued, upcoming.size, shuffle, loop, origin, actions, panel = true)
        Box(
            Modifier
                .fillMaxWidth()
                .height(1.dp)
                .background(colors.outlineVariant)
        )
        QueueList(entries, current, playing, actions, Modifier.weight(1f), dense = true)
    }
}

/**
 * The queue's title with Clear queued, where it plays from, and what happens at its end, with
 * shuffle beside it on the sheet; a [panel] beside the player leaves shuffle to it.
 */
@Composable
private fun QueueHeader(
    title: String,
    queued: List<QueueRow>,
    left: Int,
    shuffle: Boolean,
    loop: LoopStatus,
    origin: String?,
    actions: QueueActions,
    panel: Boolean,
) {
    val side = inSideSheet
    Row(
        Modifier
            .fillMaxWidth()
            // 2 dp higher than the tall sheets' handle leaves it, as the design's is shorter.
            .then(
                when {
                    panel -> Modifier.padding(top = 12.dp)
                    side -> Modifier
                    else -> Modifier.margins(top = 2.dp)
                }
            )
            .height(if (side) 64.dp else 40.dp)
            .padding(start = 20.dp, end = if (side) 12.dp else 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text(title, style = text(20, FontWeight.ExtraBold), color = colors.onSurface, modifier = Modifier.weight(1f))
        if (queued.isNotEmpty()) {
            TextAction(stringResource(R.string.clear_queued), { actions.clearQueued(queued) })
        }
        SheetClose()
    }
    PlayingFrom(origin, shuffle, left, actions::openOrigin, minHeight = if (panel) 16.dp else 36.dp)
    Column(Modifier.padding(start = 16.dp, end = 16.dp, top = if (panel) 10.dp else 4.dp, bottom = 12.dp)) {
        Text(
            stringResource(R.string.at_the_end),
            style = text(12, FontWeight.Bold),
            color = colors.onSurfaceVariant,
            modifier = Modifier.padding(start = 4.dp, bottom = 6.dp),
        )
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Segmented(
                options = listOf(LoopStatus.OFF, LoopStatus.PLAYLIST, LoopStatus.FILE),
                selected = loop,
                label = {
                    stringResource(
                        when (it) {
                            LoopStatus.OFF -> R.string.end_stop
                            LoopStatus.PLAYLIST -> R.string.repeat_all
                            LoopStatus.FILE -> R.string.repeat_one
                        }
                    )
                },
                onSelect = actions::setLoop,
                modifier = Modifier.weight(1f),
                weights = if (panel) listOf(1f, 1.35f, 1.35f) else listOf(1f, 1.45f, 1.3f),
                textSize = if (panel) 13 else 14,
                checkSize = if (panel) 16.dp else 18.dp,
            )
            if (!panel) ShuffleToggle(shuffle, actions::toggleShuffle)
        }
    }
}

/** Playing from Late night · shuffled · 23 left, the name a link to its page. */
@Composable
private fun PlayingFrom(origin: String?, shuffle: Boolean, left: Int, onOrigin: () -> Unit, minHeight: Dp) {
    val style = text(13)
    val muted = colors.onSurfaceVariant
    val rest = buildString {
        if (shuffle) append(" · ").append(stringResource(R.string.shuffled))
        append(" · ").append(pluralStringResource(R.plurals.queue_left, quantity(left), formatCount(left)))
    }
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = minHeight)
            .padding(horizontal = 20.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (origin != null) {
            // The sentence around the name, wherever a language puts it.
            val sentence = stringResource(R.string.queue_playing_from, "\u0000").split('\u0000')
            Text(sentence[0], style = style, color = muted, maxLines = 1)
            Text(
                origin,
                style = text(13, FontWeight.Bold),
                color = colors.primary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier
                    .weight(1f, fill = false)
                    .margins(top = 14.dp, bottom = 14.dp)
                    .tappable(onOrigin, role = Role.Button)
                    .padding(vertical = 14.dp),
            )
            Text(sentence.getOrElse(1) { "" } + rest, style = style, color = muted, maxLines = 1)
        } else {
            Text(rest.removePrefix(" · "), style = style, color = muted, maxLines = 1)
        }
    }
}

/** Shuffle beside what happens at the end: filled while on. */
@Composable
private fun ShuffleToggle(on: Boolean, onClick: () -> Unit) {
    val label = stringResource(R.string.shuffle)
    val state = stringResource(if (on) R.string.toggle_on else R.string.toggle_off)
    Box(
        Modifier
            .size(40.dp)
            .clip(CircleShape)
            .background(if (on) colors.secondaryContainer else colors.surfaceHigh)
            .tappable(onClick)
            .semantics {
                contentDescription = label
                stateDescription = state
            },
        contentAlignment = Alignment.Center,
    ) {
        NIcon(NIcons.Shuffle, size = 20.dp, tint = if (on) colors.onSecondaryContainer else colors.onSurface)
    }
}

/** What a row of the queue is. */
private enum class Kind { PLAYED, CURRENT, UPCOMING }

/** A row being dragged into another place, and the order the drag gives the list meanwhile. */
@Stable
private class QueueDrag {
    /** The item dragged, until it settled in its slot. */
    var key by mutableStateOf<ULong?>(null)

    /** A finger holds it. */
    var held by mutableStateOf(false)

    /** The list as the finger rearranges it, until the core tells the new order. */
    var order by mutableStateOf<List<QueueRow>?>(null)

    /** How far the dragged row is from its slot, in pixels. */
    var offset by mutableFloatStateOf(0f)

    /** From 0 to 1 as the dragged row lifts off the list. */
    val lift = Animatable(0f)

    var settling: Job? = null
}

/**
 * The session's rows, scrolled to show the two that played last above the current one. Holding
 * a row's handle lifts it, and dragging moves it: past a neighbour's middle, the neighbour slides
 * over with a tick, and letting go settles it in its slot. What played stays put. [dense] rows
 * are a little shorter, for the panel beside a foldable's player.
 */
@Composable
private fun QueueList(
    entries: List<QueueRow>,
    current: ULong?,
    playing: Boolean,
    actions: QueueActions,
    modifier: Modifier,
    dense: Boolean = false,
) {
    val density = LocalDensity.current
    val haptics = LocalHapticFeedback.current
    val scope = rememberCoroutineScope()
    val currentIndex = entries.indexOfFirst { it.item == current }
    // Nothing moves above the current row: what played stays put.
    val firstMovable = currentIndex.coerceAtLeast(0)
    val state = rememberLazyListState(
        initialFirstVisibleItemIndex = (currentIndex - 2).coerceAtLeast(0),
        // Just over half the first row hides above the list's top.
        initialFirstVisibleItemScrollOffset = if (currentIndex >= 2) with(density) { (if (dense) 30.dp else 32.dp).roundToPx() } else 0,
    )
    val drag = remember { QueueDrag() }
    LaunchedEffect(entries) { if (!drag.held) drag.order = null }
    val rows = drag.order ?: entries
    var menuFor by remember { mutableStateOf<ULong?>(null) }

    /**
     * Moves the dragged row [by] pixels, or, as the list scrolls under it, checks it [toward] a
     * way. Once its leading edge passes a neighbour's middle, they swap places.
     */
    fun dragBy(by: Float, toward: Float = by) {
        val key = drag.key ?: return
        val order = drag.order ?: return
        var offset = drag.offset + by
        val index = order.indexOfFirst { it.item == key }
        val visible = state.layoutInfo.visibleItemsInfo
        val self = visible.firstOrNull { it.index == index }
        // The current row goes no higher than its place, the rest no higher than below it.
        val highest = if (currentIndex < 0 || key == current) firstMovable else firstMovable + 1
        if (self != null && index >= 0) {
            val top = self.offset + offset
            val bottom = top + self.size
            val below = visible.firstOrNull { it.index == index + 1 }
            val above = visible.firstOrNull { it.index == index - 1 }
            val target = when {
                toward > 0 && below != null && bottom > below.offset + below.size / 2f -> {
                    offset -= below.size
                    index + 1
                }
                toward < 0 && above != null && index - 1 >= highest && top < above.offset + above.size / 2f -> {
                    offset += above.size
                    index - 1
                }
                else -> index
            }
            if (target != index) {
                // The list keeps its first row in place, not the one that moved off it.
                val first = state.firstVisibleItemIndex
                val firstOffset = state.firstVisibleItemScrollOffset
                drag.order = order.toMutableList().apply { add(target, removeAt(index)) }
                if (first == index || first == target) state.requestScrollToItem(first, firstOffset)
                haptics.performHapticFeedback(HapticFeedbackType.SegmentTick)
            }
        }
        drag.offset = offset
    }

    fun start(row: QueueRow) {
        drag.settling?.cancel()
        drag.order = rows
        drag.key = row.item
        drag.held = true
        drag.offset = 0f
        scope.launch { drag.lift.animateTo(1f, NMotion.spatialFast()) }
    }

    fun drop() {
        val key = drag.key ?: return
        drag.held = false
        val order = drag.order ?: entries
        val moved = order.indexOfFirst { it.item == key }
        val row = order.getOrNull(moved)
        if (row != null && moved != entries.indexOfFirst { it.item == key }) {
            actions.move(row, key == current, order.getOrNull(moved + 1)?.item)
        }
        drag.settling = scope.launch {
            launch { drag.lift.animateTo(0f, NMotion.spatialDefault()) }
            animate(drag.offset, 0f, animationSpec = NMotion.spatialDefault()) { value, _ -> drag.offset = value }
            drag.key = null
        }
    }

    // Near the list's ends, a held row scrolls it, faster the further it goes.
    LaunchedEffect(drag.held) {
        if (!drag.held) return@LaunchedEffect
        val edge = with(density) { 48.dp.toPx() }
        val fastest = with(density) { 12.dp.toPx() }
        while (drag.held) {
            withFrameNanos {}
            val key = drag.key ?: break
            val info = state.layoutInfo
            val self = info.visibleItemsInfo.firstOrNull { it.key == key.toLong() } ?: continue
            val top = self.offset + drag.offset
            val bottom = top + self.size
            val speed = when {
                bottom > info.viewportEndOffset - edge -> ((bottom - info.viewportEndOffset + edge) / edge).coerceAtMost(1f) * fastest
                top < info.viewportStartOffset + edge -> -((info.viewportStartOffset + edge - top) / edge).coerceAtMost(1f) * fastest
                else -> 0f
            }
            if (speed != 0f) {
                val scrolled = state.scrollBy(speed)
                if (scrolled != 0f) {
                    drag.offset += scrolled
                    dragBy(0f, scrolled)
                }
            }
        }
    }

    LazyColumn(
        modifier.fillMaxSize(),
        state = state,
        contentPadding = PaddingValues(start = 8.dp, end = 8.dp, bottom = 24.dp),
    ) {
        itemsIndexed(rows, key = { _, row -> row.item.toLong() }) { index, row ->
            val kind = when {
                row.item == current -> Kind.CURRENT
                index < firstMovable -> Kind.PLAYED
                else -> Kind.UPCOMING
            }
            val dragged = drag.key == row.item
            val moves = moves(rows, index, firstMovable, kind, actions)
            Box(
                if (dragged) {
                    Modifier.zIndex(1f)
                } else {
                    Modifier.animateItem(fadeInSpec = null, fadeOutSpec = null, placementSpec = NMotion.spatialDefault())
                }
            ) {
                if (dragged) Slot(Modifier.matchParentSize().graphicsLayer { alpha = drag.lift.value.coerceIn(0f, 1f) })
                // A queued row a swipe takes out; every row has the wrapper, so a row turning
                // queued or current keeps its gestures.
                SwipeToRemove(
                    onRemove = { actions.remove(row) },
                    label = null,
                    shape = RoundedCornerShape(16.dp),
                    surface = colors.surfaceLow,
                    enabled = kind == Kind.UPCOMING && row.queued && drag.key == null,
                ) {
                    QueueItem(
                        row = row,
                        kind = kind,
                        playing = playing,
                        dragged = dragged,
                        lift = { if (dragged) drag.lift.value else 0f },
                        offset = { if (dragged) drag.offset else 0f },
                        onClick = { if (kind != Kind.CURRENT) actions.play(row) },
                        onLongClick = {
                            haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                            menuFor = row.item
                        },
                        handle = if (kind == Kind.PLAYED) {
                            null
                        } else {
                            {
                                awaitEachGesture {
                                    val down = awaitFirstDown()
                                    down.consume()
                                    start(row)
                                    drag(down.id) { change ->
                                        dragBy(change.positionChange().y)
                                        change.consume()
                                    }
                                    drop()
                                }
                            }
                        },
                        moves = moves,
                        dense = dense,
                    )
                }
                NMenu(menuFor == row.item, { menuFor = null }) {
                    for (move in moves) {
                        MenuItem(move.label, move.icon, {
                            menuFor = null
                            move.run()
                        }, danger = move.danger)
                    }
                }
            }
        }
    }
}

/** One of a row's long-press actions; [danger] takes something away. */
private data class RowAction(val label: String, val icon: ImageVector, val danger: Boolean = false, val run: () -> Unit)

/**
 * What a long press offers for the row at [index]: moving it a place, playing it next, or
 * taking a queued one out of the queue.
 */
@Composable
private fun moves(
    rows: List<QueueRow>,
    index: Int,
    firstMovable: Int,
    kind: Kind,
    actions: QueueActions,
): List<RowAction> {
    val row = rows[index]
    val current = kind == Kind.CURRENT
    return buildList {
        if (kind == Kind.UPCOMING && index - 1 > firstMovable) {
            add(RowAction(stringResource(R.string.move_up), NIcons.Expand) { actions.move(row, false, rows[index - 1].item) })
        }
        if (kind != Kind.PLAYED && index < rows.lastIndex) {
            add(RowAction(stringResource(R.string.move_down), NIcons.Collapse) { actions.move(row, current, rows.getOrNull(index + 2)?.item) })
        }
        when {
            kind == Kind.PLAYED ->
                add(RowAction(stringResource(R.string.play_next), NIcons.PlayNext) { actions.playAgain(row) })
            kind == Kind.UPCOMING && index - 1 > firstMovable ->
                add(RowAction(stringResource(R.string.play_next), NIcons.PlayNext) { actions.move(row, false, rows[firstMovable + 1].item) })
        }
        if (kind == Kind.UPCOMING && row.queued) {
            add(RowAction(stringResource(R.string.remove_from_queue), NIcons.Remove, danger = true) { actions.remove(row) })
        }
    }
}

/** The dashed slot a dragged row settles into. */
@Composable
private fun Slot(modifier: Modifier) {
    val color = colors.outlineVariant
    val density = LocalDensity.current
    val dashes = remember(density) {
        with(density) { PathEffect.dashPathEffect(floatArrayOf(6.dp.toPx(), 4.dp.toPx())) }
    }
    Box(
        modifier.drawBehind {
            val width = 2.dp.toPx()
            drawRoundRect(
                color,
                topLeft = Offset(width / 2, width / 2),
                size = Size(size.width - width, size.height - width),
                cornerRadius = CornerRadius(16.dp.toPx() - width / 2),
                style = Stroke(width, pathEffect = dashes),
            )
        }
    )
}

/** How tall a row of the queue is, but for the current one, which is 4 dp taller. */
private fun rowHeight(dense: Boolean) = if (dense) 58.dp else 60.dp

/**
 * A row of the queue: what played is greyed, the current one washed in the accent, and queued
 * ones say so. A row [dragged] lifts by [lift], [offset] pixels from its slot; [handle] follows
 * a finger on its handle, which played rows lack. A [dense] row has a smaller cover.
 */
@Composable
private fun QueueItem(
    row: QueueRow,
    kind: Kind,
    playing: Boolean,
    dragged: Boolean,
    lift: () -> Float,
    offset: () -> Float,
    onClick: () -> Unit,
    onLongClick: () -> Unit,
    handle: (suspend PointerInputScope.() -> Unit)?,
    moves: List<RowAction>,
    dense: Boolean,
) {
    val track = row.track
    val shape = RoundedCornerShape(16.dp)
    val played = kind == Kind.PLAYED
    val now = kind == Kind.CURRENT
    val muted = colors.onSurfaceVariant
    val quiet = colors.onSurfaceQuiet
    val accent = colors.primary
    val raised = colors.surfaceHighest
    Box(
        Modifier
            .fillMaxWidth()
            .height(rowHeight(dense) + if (now) 4.dp else 0.dp)
            .graphicsLayer {
                translationY = offset()
                val scale = 1f + 0.03f * lift()
                scaleX = scale
                scaleY = scale
            }
    ) {
        if (dragged) {
            Box(
                Modifier
                    .matchParentSize()
                    .graphicsLayer { alpha = lift().coerceIn(0f, 1f) }
                    .dropShadow(shape, Shadow(24.dp, Color.Black.copy(alpha = 0.35f), offset = DpOffset(0.dp, 10.dp)))
                    .background(raised, shape)
            )
        }
        if (now && !dragged) {
            Box(
                Modifier
                    .matchParentSize()
                    .background(colors.tint, shape)
            )
        }
        val move = moves.map { CustomAccessibilityAction(it.label) { it.run(); true } }
        Row(
            Modifier
                .matchParentSize()
                .clip(shape)
                .tappable(onClick, onLongClick)
                .semantics { customActions = move }
                .padding(start = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Box(Modifier.size(if (dense) 42.dp else 44.dp).graphicsLayer { alpha = if (played) 0.5f else 1f }) {
                Cover(
                    track.cover,
                    Modifier.fillMaxSize(),
                    shape = RoundedCornerShape(8.dp),
                    placeholder = if (track.loaded) CoverPlaceholder.ALBUM else CoverPlaceholder.UNREAD,
                )
                if (now) {
                    Box(
                        Modifier
                            .fillMaxSize()
                            .background(Color.Black.copy(alpha = 0.45f), RoundedCornerShape(8.dp)),
                        contentAlignment = Alignment.Center,
                    ) {
                        PlayingBars(color = Color.White, animate = playing)
                    }
                }
            }
            Column(Modifier.weight(1f)) {
                Text(
                    track.title,
                    style = text(15, if (now) FontWeight.Bold else if (dragged) FontWeight.SemiBold else FontWeight.Medium),
                    color = when {
                        now -> accent
                        played -> muted
                        else -> colors.onSurface
                    },
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                val artist = track.artist.ifEmpty { stringResource(R.string.unknown_artist) }
                val label = when {
                    played -> stringResource(R.string.queue_played)
                    now -> stringResource(R.string.now_playing)
                    row.queued -> stringResource(R.string.queue_queued)
                    else -> null
                }
                Text(
                    buildAnnotatedString {
                        when {
                            played -> append("$label · $artist")
                            label != null -> {
                                withStyle(SpanStyle(color = accent, fontWeight = FontWeight.Bold)) { append(label) }
                                append(" · $artist")
                            }
                            else -> append(artist)
                        }
                    },
                    style = text(13),
                    color = if (played) quiet else muted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Text(
                formatLength(track.length),
                style = text(13, tabular = true),
                color = if (played) quiet else muted,
            )
            if (handle == null) {
                Spacer(Modifier.width(44.dp))
            } else {
                // Accessibility services move a row with its actions instead of the handle.
                Box(
                    Modifier
                        .size(44.dp, 48.dp)
                        .pointerInput(row.item) { handle() }
                        .clearAndSetSemantics {},
                    contentAlignment = Alignment.Center,
                ) {
                    NIcon(NIcons.Drag, tint = lerp(muted, accent, lift().coerceIn(0f, 1f)))
                }
            }
        }
    }
}
