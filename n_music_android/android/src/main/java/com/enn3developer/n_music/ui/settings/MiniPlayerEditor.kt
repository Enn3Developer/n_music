package com.enn3developer.n_music.ui.settings

import androidx.compose.animation.BoundsTransform
import androidx.compose.animation.animateBounds
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.VisibilityThreshold
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.drag
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.geometry.lerp
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.AwaitPointerEventScope
import androidx.compose.ui.input.pointer.PointerId
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.LookaheadScope
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.util.lerp
import com.enn3developer.n_music.MiniButton
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.CoverPlaceholder
import com.enn3developer.n_music.ui.components.HoldFill
import com.enn3developer.n_music.ui.components.MenuItem
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NMenu
import com.enn3developer.n_music.ui.components.PlaybackActions
import com.enn3developer.n_music.ui.components.PlaybackUi
import com.enn3developer.n_music.ui.components.holdFill
import com.enn3developer.n_music.ui.components.miniButton
import com.enn3developer.n_music.ui.components.miniProgress
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.delayed
import com.enn3developer.n_music.ui.theme.text
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import kotlin.math.roundToInt

/** How many buttons a phone's bar holds, leaving room for the title. */
const val BAR_MAX = 5

/** The spare buttons in the order the tray shows them. */
private val TRAY_ORDER = listOf(
    MiniButton.REPEAT, MiniButton.SHUFFLE, MiniButton.OUTPUT, MiniButton.SLEEP_TIMER,
    MiniButton.PREVIOUS, MiniButton.PLAY_PAUSE, MiniButton.NEXT,
)

/** How far above the finger a lifted button floats, so the finger doesn't hide it. */
private val LIFT_ABOVE = 22.dp

/** How long after a tray button lifts the others close its gap, once it has lifted. */
private const val TRAY_CLOSE_MS = 230L

/** How long after a button lands back in the tray its name comes back. */
private const val NAME_BACK_MS = 100L

/** A tray button's corners, and a lifted one's on the bigger chip. */
private val TRAY_CORNER = 16.dp
private val CHIP_TRAY_CORNER = TRAY_CORNER * 56f / 48f

/** Nothing in the preview plays: its buttons only show. */
private object Preview : PlaybackActions {
    override fun togglePause() {}
    override fun previous() {}
    override fun next() {}
    override fun toggleShuffle() {}
    override fun cycleRepeat() {}
    override fun openOutput() {}
    override fun openSleepTimer() {}
}

/** A button held up off the bar or the tray, and where it would land. */
@Stable
private class EditorDrag {
    var button by mutableStateOf<MiniButton?>(null)

    /** It came off the bar, rather than the tray. */
    var fromBar by mutableStateOf(false)

    /** The finger, in the editor's coordinates. */
    var position by mutableStateOf(Offset.Zero)

    /** Where in the bar it would land; `null` while out of the bar. */
    var target by mutableStateOf<Int?>(null)

    /** From 0 to 1 as it lifts. */
    val lift = Animatable(0f)

    /** The button let go, while it lands in its new place. */
    var landing by mutableStateOf<MiniButton?>(null)

    /** It lands in the tray, where it turns back into a rounded square. */
    var toTray by mutableStateOf(false)

    /** The chip's centre when it was let go, which it lands from. */
    var from by mutableStateOf(Offset.Zero)

    /** From 0 to 1 as it lands. */
    val land = Animatable(0f)
}

/**
 * The mini player's buttons as a preview bar and a tray of the rest. Holding or dragging a button
 * lifts it, and it goes where it is let go: along the bar, into it or out of it; a tap gives the
 * same moves as a menu, and accessibility services as actions. A full bar dims the tray.
 */
@Composable
fun MiniPlayerEditor(
    buttons: List<MiniButton>,
    preview: PlaybackUi,
    progress: () -> Float,
    onChange: (List<MiniButton>) -> Unit,
) {
    val density = LocalDensity.current
    val haptics = LocalHapticFeedback.current
    val scope = rememberCoroutineScope()
    val drag = remember { EditorDrag() }
    // The fill growing under the finger in step with the hold, on the button pressed.
    val fill = remember { HoldFill() }
    var pressed by remember { mutableStateOf<MiniButton?>(null) }
    val fillColor = colors.onSurface
    var root by remember { mutableStateOf<LayoutCoordinates?>(null) }
    var bar by remember { mutableStateOf(Rect.Zero) }
    // Where each button shows, bar and tray alike, for a finger to find it.
    val bounds = remember { mutableStateMapOf<MiniButton, Rect>() }
    var menuFor by remember { mutableStateOf<MiniButton?>(null) }
    val full = buttons.size >= BAR_MAX
    val dragged = drag.button
    val rest = if (dragged != null && drag.fromBar) buttons - dragged else buttons
    // The bar as it shows: the held button's slot goes where it would land.
    val entries: List<MiniButton?> = drag.target?.let { target ->
        rest.toMutableList<MiniButton?>().apply { add(target.coerceIn(0, size), null) }
    } ?: rest
    // The tray as it shows: one lifted out of it leaves a gap that closes, and one held off the
    // bar has its place open while it is out of the bar's reach.
    val tray = TRAY_ORDER.filter { button ->
        if (button == dragged) drag.fromBar && drag.target == null else button !in buttons
    }
    val trayMove = remember(drag) {
        BoundsTransform { _, _ ->
            val delay = if (drag.button != null && !drag.fromBar) TRAY_CLOSE_MS else 0L
            NMotion.spatialDefault(Rect.VisibilityThreshold).delayed(delay)
        }
    }

    fun change(next: List<MiniButton>) {
        if (next != buttons) onChange(next)
    }

    fun move(button: MiniButton, by: Int) {
        val index = buttons.indexOf(button)
        val to = (index + by).coerceIn(0, buttons.lastIndex)
        if (index < 0 || to == index) return
        change(buttons.toMutableList().apply { add(to, removeAt(index)) })
    }

    /** Where in the bar a button at [position] lands; `null` when it leaves the bar. */
    fun targetAt(position: Offset): Int? = with(density) {
        val reach = 24.dp.toPx()
        if (position.y < bar.top - reach || position.y > bar.bottom + reach) return null
        val others = if (drag.fromBar) buttons.size - 1 else buttons.size
        if (others >= BAR_MAX) return null
        val slots = others + 1
        val slot = 48.dp.toPx()
        val left = bar.right - 4.dp.toPx() - slots * slot
        ((position.x - left) / slot).toInt().coerceIn(0, slots - 1)
    }

    fun lift(button: MiniButton, at: Offset, held: Boolean) {
        drag.landing = null
        drag.fromBar = button in buttons
        drag.button = button
        drag.position = at
        drag.target = targetAt(at)
        haptics.performHapticFeedback(if (held) HapticFeedbackType.LongPress else HapticFeedbackType.GestureThresholdActivate)
        scope.launch { drag.lift.animateTo(1f, NMotion.spatialFast()) }
    }

    fun follow(to: Offset) {
        drag.position = to
        val target = targetAt(to)
        if (target != drag.target) {
            drag.target = target
            haptics.performHapticFeedback(HapticFeedbackType.SegmentTick)
        }
    }

    fun drop() {
        val button = drag.button ?: return
        val target = drag.target
        val others = buttons - button
        when {
            target != null -> change(others.toMutableList().apply { add(target.coerceIn(0, size), button) })
            drag.fromBar -> change(others)
        }
        // It lands where it now is, shrinking back as its shadow goes.
        drag.from = drag.position - Offset(0f, with(density) { LIFT_ABOVE.toPx() })
        drag.toTray = target == null
        drag.landing = button
        drag.button = null
        drag.target = null
        haptics.performHapticFeedback(HapticFeedbackType.GestureEnd)
        scope.launch {
            drag.land.snapTo(0f)
            launch { drag.lift.animateTo(0f, NMotion.spatialDefault()) }
            drag.land.animateTo(1f, NMotion.spatialDefault())
            if (drag.landing == button) drag.landing = null
        }
    }

    Box(
        Modifier
            .fillMaxWidth()
            .onPlaced { root = it }
            .pointerInput(buttons, full) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    val button = bounds.entries.firstOrNull { it.value.contains(down.position) }?.key
                        ?: return@awaitEachGesture
                    // The tray holds still while the bar is full.
                    if (full && button !in buttons) return@awaitEachGesture
                    val hold = viewConfiguration.longPressTimeoutMillis
                    pressed = button
                    val from = down.position - (bounds[button]?.topLeft ?: Offset.Zero)
                    scope.launch { fill.press(from, hold) }
                    // Held still, it lifts once the fill is full; moved, it lifts at once, before
                    // the page can take the finger for a scroll.
                    var finger = down.position
                    val press = withTimeoutOrNull(hold) {
                        awaitPress(down.id, down.position, viewConfiguration.touchSlop) { finger = it }
                    }
                    when (press) {
                        Press.TAP -> {
                            scope.launch { fill.release() }
                            menuFor = button
                        }
                        Press.TAKEN -> scope.launch { fill.cancel() }
                        else -> {
                            lift(button, finger, held = press == null)
                            scope.launch { fill.release() }
                            // Where the finger is, not how far it went: the button stays under it.
                            val finished = drag(down.id) { change ->
                                follow(change.position)
                                change.consume()
                            }
                            if (finished) {
                                drop()
                            } else {
                                drag.button = null
                                drag.target = null
                                scope.launch { drag.lift.snapTo(0f) }
                            }
                        }
                    }
                }
            },
    ) {
        Column {
            Text(
                stringResource(R.string.mini_editor_hint),
                style = text(14, lineHeight = 20.sp),
                color = colors.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 16.dp),
            )
            val previewLabel = stringResource(R.string.mini_preview)
            Row(
                Modifier
                    .padding(top = 12.dp)
                    .fillMaxWidth()
                    .height(64.dp)
                    .clip(RoundedCornerShape(16.dp))
                    .background(colors.surfaceHigh)
                    .then(if (preview.track != null) Modifier.miniProgress(progress) else Modifier)
                    .onGloballyPositioned { coordinates -> root?.let { bar = it.localBoundingBoxOf(coordinates) } }
                    .semantics { contentDescription = previewLabel }
                    .padding(start = 8.dp, end = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                PreviewTrack(preview, Modifier.weight(1f))
                val width by animateDpAsState(48.dp * entries.size, NMotion.spatialDefault(), label = "buttons")
                Box(Modifier.width(width).height(48.dp)) {
                    entries.forEachIndexed { index, entry ->
                        key(entry ?: "slot") {
                            val fromEnd by animateDpAsState(48.dp * (entries.size - 1 - index), NMotion.spatialDefault(), label = "slot")
                            val place = Modifier
                                .align(Alignment.CenterEnd)
                                .offset { IntOffset(-fromEnd.roundToPx(), 0) }
                            if (entry == null) {
                                Slot(place)
                            } else {
                                Box(place) {
                                    BarButton(
                                        entry, preview, buttons, root, bounds,
                                        fill = if (pressed == entry) fill else null,
                                        fillColor = fillColor,
                                        landing = drag.landing == entry,
                                        onMenu = { menuFor = entry },
                                        onTakeOut = { change(buttons - entry) },
                                        onMove = { move(entry, it) },
                                    )
                                    NMenu(menuFor == entry, { menuFor = null }) {
                                        val index = buttons.indexOf(entry)
                                        if (index > 0) {
                                            MenuItem(stringResource(R.string.mini_move_left), NIcons.Back, {
                                                menuFor = null
                                                move(entry, -1)
                                            })
                                        }
                                        if (index in 0 until buttons.lastIndex) {
                                            MenuItem(stringResource(R.string.mini_move_right), NIcons.Open, {
                                                menuFor = null
                                                move(entry, 1)
                                            })
                                        }
                                        MenuItem(stringResource(R.string.mini_take_out), NIcons.RemoveSource, {
                                            menuFor = null
                                            change(buttons - entry)
                                        })
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Text(
                stringResource(R.string.mini_more),
                style = text(13, FontWeight.Bold),
                color = colors.onSurfaceVariant,
                modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 16.dp),
            )
            if (full) {
                Text(
                    stringResource(R.string.mini_full, BAR_MAX),
                    style = text(13, lineHeight = 18.sp),
                    color = colors.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 2.dp),
                )
            }
            LookaheadScope {
                TrayGrid(
                    Modifier
                        .padding(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 16.dp)
                        .animateContentSize(NMotion.spatialDefault())
                        .graphicsLayer { alpha = if (full) 0.38f else 1f },
                ) {
                    for (button in tray) {
                        key(button) {
                            Box(
                                Modifier.animateBounds(this@LookaheadScope, boundsTransform = trayMove),
                                contentAlignment = Alignment.TopCenter,
                            ) {
                                if (button == dragged) {
                                    Slot(Modifier, corner = TRAY_CORNER)
                                } else {
                                    TrayButton(
                                        button, enabled = !full, root, bounds,
                                        fill = if (pressed == button) fill else null,
                                        fillColor = fillColor,
                                        landing = drag.landing == button,
                                        onMenu = { menuFor = button },
                                        onAdd = { change(buttons + button) },
                                    )
                                    NMenu(menuFor == button, { menuFor = null }) {
                                        MenuItem(stringResource(R.string.mini_add), NIcons.Add, {
                                            menuFor = null
                                            change(buttons + button)
                                        })
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        val chip = dragged ?: drag.landing
        if (chip != null) Lifted(chip, preview, drag, bounds, fill, fillColor)
    }
}

/** How a press on a button ended before it was held. */
private enum class Press {
    /** The finger let go: a tap. */
    TAP,

    /** The finger moved off as far as a drag. */
    MOVE,

    /** Another gesture took the finger. */
    TAKEN,
}

/**
 * Waits for pointer [id], down at [start], to let go, to move [slop] from there, which it takes
 * for itself, or to be taken; [onMove] hears where it is meanwhile.
 */
private suspend fun AwaitPointerEventScope.awaitPress(id: PointerId, start: Offset, slop: Float, onMove: (Offset) -> Unit): Press {
    while (true) {
        val change = awaitPointerEvent().changes.firstOrNull { it.id == id } ?: return Press.TAKEN
        if (change.isConsumed) return Press.TAKEN
        if (change.changedToUp()) return Press.TAP
        onMove(change.position)
        if ((change.position - start).getDistance() > slop) {
            change.consume()
            return Press.MOVE
        }
    }
}

/** The preview's track: what plays, or the app's name while nothing does. */
@Composable
private fun PreviewTrack(preview: PlaybackUi, modifier: Modifier) {
    val track = preview.track
    Row(modifier, verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Cover(
            track?.cover,
            Modifier.size(48.dp),
            shape = RoundedCornerShape(10.dp),
            placeholder = if (track?.loaded == false) CoverPlaceholder.UNREAD else CoverPlaceholder.ALBUM,
        )
        Column(Modifier.weight(1f)) {
            Text(
                track?.title ?: stringResource(R.string.app_name),
                style = text(14, FontWeight.Bold),
                color = colors.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            if (track != null) {
                Text(
                    track.artist.ifEmpty { stringResource(R.string.unknown_artist) },
                    style = text(13),
                    color = colors.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.padding(top = 1.dp),
                )
            }
        }
    }
}

/** Whether [button] shows as on: shuffle, repeat and a running timer, as in the player. */
private fun isOn(button: MiniButton, preview: PlaybackUi): Boolean = when (button) {
    MiniButton.SHUFFLE -> preview.shuffle
    MiniButton.REPEAT -> preview.loop != LoopStatus.OFF
    MiniButton.SLEEP_TIMER -> preview.sleeping
    else -> false
}

/** Keeps [button]'s place among [bounds], in [root]'s coordinates, for a finger to find it. */
private fun Modifier.tracked(button: MiniButton, root: LayoutCoordinates?, bounds: MutableMap<MiniButton, Rect>): Modifier =
    onGloballyPositioned { coordinates -> root?.let { bounds[button] = it.localBoundingBoxOf(coordinates) } }

/** A button in the preview bar, as the mini player draws it. */
@Composable
private fun BarButton(
    button: MiniButton,
    preview: PlaybackUi,
    buttons: List<MiniButton>,
    root: LayoutCoordinates?,
    bounds: MutableMap<MiniButton, Rect>,
    fill: HoldFill?,
    fillColor: Color,
    landing: Boolean,
    onMenu: () -> Unit,
    onTakeOut: () -> Unit,
    onMove: (Int) -> Unit,
) {
    val (icon, _, _) = miniButton(button, preview, Preview)
    val on = isOn(button, preview)
    val name = buttonName(button)
    val description = stringResource(if (on) R.string.mini_in_bar_on else R.string.mini_in_bar, name)
    val index = buttons.indexOf(button)
    val actions = buildList {
        if (index > 0) add(CustomAccessibilityAction(stringResource(R.string.mini_move_left)) { onMove(-1); true })
        if (index in 0 until buttons.lastIndex) add(CustomAccessibilityAction(stringResource(R.string.mini_move_right)) { onMove(1); true })
        add(CustomAccessibilityAction(stringResource(R.string.mini_take_out)) { onTakeOut(); true })
    }
    Box(
        Modifier
            .size(48.dp)
            .tracked(button, root, bounds)
            .semantics {
                contentDescription = description
                role = Role.Button
                onClick { onMenu(); true }
                customActions = actions
            }
            .then(if (fill != null) Modifier.holdFill(fill, fillColor, CircleShape) else Modifier)
            // Its chip shows it while it lands.
            .graphicsLayer { alpha = if (landing) 0f else 1f },
        contentAlignment = Alignment.Center,
    ) {
        if (on) {
            Box(
                Modifier
                    .size(40.dp)
                    .background(colors.secondaryContainer, CircleShape),
                contentAlignment = Alignment.Center,
            ) {
                NIcon(icon, size = 22.dp, tint = colors.onSecondaryContainer)
            }
        } else {
            NIcon(icon, tint = colors.onSurface)
        }
    }
}

/** A spare button in the tray: its icon on a tile, and its name. */
@Composable
private fun TrayButton(
    button: MiniButton,
    enabled: Boolean,
    root: LayoutCoordinates?,
    bounds: MutableMap<MiniButton, Rect>,
    fill: HoldFill?,
    fillColor: Color,
    landing: Boolean,
    onMenu: () -> Unit,
    onAdd: () -> Unit,
) {
    val name = buttonName(button)
    val description = stringResource(R.string.mini_not_in_bar, name)
    val add = stringResource(R.string.mini_add)
    Column(
        Modifier.semantics {
            contentDescription = description
            role = Role.Button
            if (enabled) {
                onClick { onMenu(); true }
                customActions = listOf(CustomAccessibilityAction(add) { onAdd(); true })
            } else {
                disabled()
            }
        },
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        val shape = RoundedCornerShape(TRAY_CORNER)
        // Landing back in the tray, its name comes back a little after its chip lets go.
        val named = remember { Animatable(if (landing) 0f else 1f) }
        LaunchedEffect(Unit) { named.animateTo(1f, NMotion.effectsDefault<Float>().delayed(NAME_BACK_MS)) }
        Box(
            Modifier
                .size(48.dp)
                .tracked(button, root, bounds)
                .then(if (fill != null) Modifier.holdFill(fill, fillColor, shape) else Modifier)
                .graphicsLayer { alpha = if (landing) 0f else 1f }
                .background(colors.surfaceHigh, shape),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(trayIcon(button), tint = colors.onSurface)
        }
        Text(
            name,
            style = text(12, FontWeight.SemiBold),
            color = colors.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.graphicsLayer { alpha = named.value },
        )
    }
}

/** A button's icon away from playback's state: repeat as repeat, play/pause as play. */
private fun trayIcon(button: MiniButton) = when (button) {
    MiniButton.SHUFFLE -> NIcons.Shuffle
    MiniButton.PREVIOUS -> NIcons.Previous
    MiniButton.PLAY_PAUSE -> NIcons.Play
    MiniButton.NEXT -> NIcons.Next
    MiniButton.REPEAT -> NIcons.Repeat
    MiniButton.OUTPUT -> NIcons.Output
    MiniButton.SLEEP_TIMER -> NIcons.SleepTimer
}

/** The dashed outline where a held button would land: round in the bar, [corner] in the tray. */
@Composable
private fun Slot(modifier: Modifier, corner: Dp? = null) {
    val color = colors.primary
    // It opens out from a little smaller.
    val grow = remember { Animatable(0f) }
    val shown = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        launch { shown.animateTo(1f, NMotion.effectsFast()) }
        grow.animateTo(1f, NMotion.spatialDefault())
    }
    Box(
        modifier
            .size(48.dp)
            .graphicsLayer {
                val scale = 0.6f + 0.4f * grow.value
                scaleX = scale
                scaleY = scale
                alpha = shown.value
            }
            .drawBehind {
                val width = 2.dp.toPx()
                val dash = 4.dp.toPx()
                drawRoundRect(
                    color,
                    topLeft = Offset(width / 2, width / 2),
                    size = Size(size.width - width, size.height - width),
                    cornerRadius = CornerRadius(corner?.toPx() ?: (size.minDimension / 2)),
                    style = Stroke(width, pathEffect = PathEffect.dashPathEffect(floatArrayOf(dash, dash))),
                )
            }
    )
}

/**
 * The held button, floating over the finger with its shadow. Let go, it lands in its new place
 * in [bounds], shrinking back as its shadow goes, and in the tray turns back into a rounded
 * square of the tray's colour.
 */
@Composable
private fun Lifted(
    button: MiniButton,
    preview: PlaybackUi,
    drag: EditorDrag,
    bounds: Map<MiniButton, Rect>,
    fill: HoldFill,
    fillColor: Color,
) {
    val (icon, _, _) = miniButton(button, preview, Preview)
    val lifted = colors.surfaceHighest
    val tray = colors.surfaceHigh
    val disc = colors.secondaryContainer
    val landing = drag.landing == button
    // An on button lands in the bar as the bar draws it: smaller, on its own disc.
    val landsOn = landing && !drag.toTray && isOn(button, preview)
    Box(
        Modifier
            .offset {
                val half = 28.dp.toPx()
                val center = if (landing) {
                    lerp(drag.from, bounds[button]?.center ?: drag.from, drag.land.value)
                } else {
                    drag.position - Offset(0f, LIFT_ABOVE.toPx())
                }
                IntOffset((center.x - half).roundToInt(), (center.y - half).roundToInt())
            }
            .size(56.dp)
            .graphicsLayer {
                val scale = 48f / 56f + (1f - 48f / 56f) * drag.lift.value
                scaleX = scale
                scaleY = scale
            }
            .dropShadow(CircleShape) {
                radius = 24.dp.toPx()
                color = Color.Black
                alpha = 0.35f * drag.lift.value.coerceIn(0f, 1f)
                offset = Offset(0f, 10.dp.toPx())
            }
            .graphicsLayer {
                val round = size.minDimension / 2
                val corner = if (landing && drag.toTray) lerp(round, CHIP_TRAY_CORNER.toPx(), drag.land.value) else round
                shape = RoundedCornerShape(corner)
                clip = true
            }
            .holdFill(fill, fillColor, RectangleShape)
            .drawBehind {
                val landed = drag.land.value.coerceIn(0f, 1f)
                when {
                    !landing -> drawRect(lifted)
                    drag.toTray -> drawRect(lerp(lifted, tray, landed))
                    landsOn -> drawCircle(lerp(lifted, disc, landed), size.minDimension / 2 * lerp(1f, 40f / 48f, landed))
                    // The bar's other buttons are bare.
                    else -> drawRect(lifted.copy(alpha = 1f - landed))
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        NIcon(
            if (landing && drag.toTray) trayIcon(button) else icon,
            Modifier.graphicsLayer {
                val scale = if (landsOn) lerp(1f, 22f / 24f, drag.land.value) else 1f
                scaleX = scale
                scaleY = scale
            },
            tint = if (landsOn) colors.onSecondaryContainer else colors.onSurface,
        )
    }
}

/** The tray's buttons, four to a row; a short row keeps the columns of full ones. */
@Composable
private fun TrayGrid(modifier: Modifier, content: @Composable () -> Unit) {
    Layout(content, modifier) { measurables, constraints ->
        val gap = 8.dp.roundToPx()
        val cell = ((constraints.maxWidth - 3 * gap) / 4).coerceAtLeast(0)
        val placeables = measurables.map { it.measure(Constraints.fixedWidth(cell)) }
        val row = placeables.maxOfOrNull { it.height } ?: 0
        val rows = (placeables.size + 3) / 4
        layout(constraints.maxWidth, (rows * row + (rows - 1) * gap).coerceAtLeast(0)) {
            placeables.forEachIndexed { index, placeable ->
                placeable.placeRelative((index % 4) * (cell + gap), (index / 4) * (row + gap))
            }
        }
    }
}
