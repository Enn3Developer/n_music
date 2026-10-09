package com.enn3developer.n_music.ui.settings

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.drag
import androidx.compose.foundation.gestures.waitForUpOrCancellation
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.shadow.Shadow
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.layout.LayoutCoordinates
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
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.enn3developer.n_music.MiniButton
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.CoverPlaceholder
import com.enn3developer.n_music.ui.components.MenuItem
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NMenu
import com.enn3developer.n_music.ui.components.PlaybackActions
import com.enn3developer.n_music.ui.components.PlaybackUi
import com.enn3developer.n_music.ui.components.miniButton
import com.enn3developer.n_music.ui.components.miniProgress
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
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
}

/**
 * The mini player's buttons as a preview bar and a tray of the rest. Holding a button lifts it,
 * and it goes where it is let go: along the bar, into it or out of it; a tap gives the same moves
 * as a menu, and accessibility services as actions. A full bar dims the tray.
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
    val tray = TRAY_ORDER.filter { it !in buttons && it != dragged }

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

    fun lift(button: MiniButton, at: Offset) {
        drag.fromBar = button in buttons
        drag.button = button
        drag.position = at
        drag.target = targetAt(at)
        haptics.performHapticFeedback(HapticFeedbackType.LongPress)
        scope.launch { drag.lift.animateTo(1f, NMotion.spatialFast()) }
    }

    fun follow(by: Offset) {
        drag.position += by
        val target = targetAt(drag.position)
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
        drag.button = null
        drag.target = null
        haptics.performHapticFeedback(HapticFeedbackType.GestureEnd)
        scope.launch { drag.lift.snapTo(0f) }
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
                    var held = true
                    val up = withTimeoutOrNull(viewConfiguration.longPressTimeoutMillis) {
                        waitForUpOrCancellation().also { held = false }
                    }
                    when {
                        up != null -> menuFor = button
                        // A scroll took the finger before it held.
                        !held -> {}
                        else -> {
                            lift(button, down.position)
                            val finished = drag(down.id) { change ->
                                follow(change.positionChange())
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
            Column(
                Modifier
                    .padding(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 16.dp)
                    .graphicsLayer { alpha = if (full) 0.38f else 1f },
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                for (row in tray.chunked(4)) {
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        for (button in row) {
                            Box(Modifier.weight(1f), contentAlignment = Alignment.Center) {
                                TrayButton(button, enabled = !full, root, bounds, onMenu = { menuFor = button }, onAdd = { change(buttons + button) })
                                NMenu(menuFor == button, { menuFor = null }) {
                                    MenuItem(stringResource(R.string.mini_add), NIcons.Add, {
                                        menuFor = null
                                        change(buttons + button)
                                    })
                                }
                            }
                        }
                        // Short rows keep the columns of full ones.
                        repeat(4 - row.size) { Box(Modifier.weight(1f)) }
                    }
                }
            }
        }
        if (dragged != null) Lifted(dragged, preview, drag)
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
            },
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
        Box(
            Modifier
                .size(48.dp)
                .tracked(button, root, bounds)
                .background(colors.surfaceHigh, RoundedCornerShape(16.dp)),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(trayIcon(button), tint = colors.onSurface)
        }
        Text(name, style = text(12, FontWeight.SemiBold), color = colors.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
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

/** The dashed ring where a held button would land in the bar. */
@Composable
private fun Slot(modifier: Modifier) {
    val color = colors.primary
    Box(
        modifier
            .size(48.dp)
            .drawBehind {
                val width = 2.dp.toPx()
                val dash = 4.dp.toPx()
                drawRoundRect(
                    color,
                    topLeft = Offset(width / 2, width / 2),
                    size = Size(size.width - width, size.height - width),
                    cornerRadius = CornerRadius(size.minDimension / 2),
                    style = Stroke(width, pathEffect = PathEffect.dashPathEffect(floatArrayOf(dash, dash))),
                )
            }
    )
}

/** The held button, floating over the finger with its shadow. */
@Composable
private fun Lifted(button: MiniButton, preview: PlaybackUi, drag: EditorDrag) {
    val density = LocalDensity.current
    val (icon, _, _) = miniButton(button, preview, Preview)
    Box(
        Modifier
            .offset {
                val half = with(density) { 28.dp.toPx() }
                val above = with(density) { LIFT_ABOVE.toPx() }
                IntOffset((drag.position.x - half).roundToInt(), (drag.position.y - half - above).roundToInt())
            }
            .size(56.dp)
            .graphicsLayer {
                val lift = drag.lift.value
                val scale = 48f / 56f + (1f - 48f / 56f) * lift
                scaleX = scale
                scaleY = scale
            }
            .dropShadow(CircleShape, Shadow(24.dp, Color.Black.copy(alpha = 0.35f), offset = DpOffset(0.dp, 10.dp)))
            .background(colors.surfaceHighest, CircleShape),
        contentAlignment = Alignment.Center,
    ) {
        NIcon(icon, tint = colors.onSurface)
    }
}
