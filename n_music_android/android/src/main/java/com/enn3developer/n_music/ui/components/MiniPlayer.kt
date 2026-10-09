package com.enn3developer.n_music.ui.components

import android.os.SystemClock
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameMillis
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.MiniButton
import com.enn3developer.n_music.Position
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import kotlinx.coroutines.launch
import kotlin.math.abs
import kotlin.math.roundToInt

/**
 * Where playback is now, in seconds: the last reported position, carried on by the clock while
 * playing, so bars move every frame between the core's reports.
 */
@Composable
fun rememberPlaybackSeconds(position: Position, playing: Boolean): State<Double> {
    val seconds = remember { mutableDoubleStateOf(position.position) }
    LaunchedEffect(position, playing) {
        seconds.doubleValue = position.position
        if (!playing) return@LaunchedEffect
        while (true) {
            withFrameMillis {
                val elapsed = (SystemClock.elapsedRealtime() - position.at) / 1000.0
                seconds.doubleValue = (position.position + elapsed).coerceAtMost(
                    if (position.length > 0) position.length else Double.MAX_VALUE
                )
            }
        }
    }
    return seconds
}

/** What the mini player and the player show of playback. */
data class PlaybackUi(
    val track: TrackRow?,
    val playing: Boolean = false,
    val shuffle: Boolean = false,
    val loop: LoopStatus = LoopStatus.PLAYLIST,
    /** A running sleep timer's minutes left; `null` while none runs. */
    val sleepMinutes: Int? = null,
)

/** What the mini player and the player ask of playback. */
interface PlaybackActions {
    fun togglePause()
    fun previous()
    fun next()
    fun toggleShuffle()
    fun cycleRepeat()
    fun openOutput()
    fun openSleepTimer()
}

/**
 * The mini player above the navigation: the playing track and its [buttons]. Tapping it or
 * swiping it up opens the player; swiping it sideways skips.
 */
@Composable
fun MiniPlayer(
    ui: PlaybackUi,
    progress: () -> Float,
    buttons: List<MiniButton>,
    actions: PlaybackActions,
    onOpen: () -> Unit,
    modifier: Modifier = Modifier,
    trailing: @Composable () -> Unit = {},
) {
    val shape = RoundedCornerShape(16.dp)
    val track = ui.track ?: return
    val scope = rememberCoroutineScope()
    val swipe = remember { Animatable(0f) }
    var width by remember { mutableIntStateOf(1) }
    // Which way the last skip went, so the new track comes in from the other side.
    var direction by remember { mutableIntStateOf(1) }
    val line = colors.outlineVariant
    val fill = colors.primary
    Box(
        modifier
            .fillMaxWidth()
            .height(64.dp)
            .floating(shape)
            .clip(shape)
            .background(colors.surfaceHigh)
            .onSizeChanged { width = it.width }
            .drawBehind {
                // The progress line along the bottom edge.
                val inset = 12.dp.toPx()
                val thickness = 3.dp.toPx()
                val top = size.height - thickness
                val width = size.width - 2 * inset
                val radius = CornerRadius(2.dp.toPx())
                drawRoundRect(line, Offset(inset, top), Size(width, thickness), radius)
                drawRoundRect(
                    fill,
                    Offset(inset, top),
                    Size(width * progress().coerceIn(0f, 1f), thickness),
                    radius,
                )
            }
            .draggable(
                rememberDraggableState { delta -> scope.launch { swipe.snapTo(swipe.value + delta) } },
                Orientation.Horizontal,
                onDragStopped = { velocity ->
                    val far = abs(swipe.value) > width / 3f || abs(velocity) > 1200f
                    if (far) {
                        val forward = (if (abs(velocity) > 1200f) velocity else swipe.value) < 0
                        direction = if (forward) 1 else -1
                        if (forward) actions.next() else actions.previous()
                    }
                    swipe.animateTo(0f, NMotion.spatialDefault())
                },
            )
            .padding(start = 8.dp, end = 4.dp),
        contentAlignment = Alignment.CenterStart,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            AnimatedContent(
                targetState = track,
                contentKey = { it.locator },
                transitionSpec = {
                    (slideInHorizontally(NMotion.spatialDefault()) { it / 3 * direction } +
                        fadeIn(NMotion.effectsDefault()))
                        .togetherWith(
                            slideOutHorizontally(NMotion.spatialDefault()) { -it / 3 * direction } +
                                fadeOut(NMotion.effectsFast())
                        )
                },
                label = "track",
                modifier = Modifier
                    .weight(1f)
                    .offset { IntOffset(swipe.value.roundToInt(), 0) },
            ) { shown ->
                Row(
                    Modifier
                        .fillMaxWidth()
                        .height(64.dp)
                        .tappable(onOpen)
                        .semantics { contentDescription = shown.title },
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Cover(
                        shown.cover,
                        Modifier.size(48.dp),
                        shape = RoundedCornerShape(10.dp),
                        placeholder = if (shown.loaded) CoverPlaceholder.ALBUM else CoverPlaceholder.UNREAD,
                    )
                    Column(
                        Modifier
                            .padding(start = 12.dp)
                            .weight(1f)
                    ) {
                        Text(
                            shown.title,
                            style = text(14, FontWeight.Bold),
                            color = colors.onSurface,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                        Text(
                            shown.artist.ifEmpty { stringResource(R.string.unknown_artist) },
                            style = text(13),
                            color = colors.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            modifier = Modifier.padding(top = 1.dp),
                        )
                    }
                }
            }
            for (button in buttons) MiniPlayerButton(button, ui, actions)
            trailing()
        }
    }
}

/** One of the mini player's buttons; shuffle, repeat and a running timer fill in while on. */
@Composable
fun MiniPlayerButton(button: MiniButton, ui: PlaybackUi, actions: PlaybackActions) {
    val (icon, description, action) = miniButton(button, ui, actions)
    val on = when (button) {
        MiniButton.SHUFFLE -> ui.shuffle
        MiniButton.REPEAT -> ui.loop != LoopStatus.OFF
        MiniButton.SLEEP_TIMER -> ui.sleepMinutes != null
        else -> false
    }
    Box(
        Modifier
            .size(48.dp)
            .clip(CircleShape)
            .tappable(action, role = Role.Button)
            .semantics { contentDescription = description },
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

/** A mini player button's icon, what it says, and what it does. */
@Composable
fun miniButton(button: MiniButton, ui: PlaybackUi, actions: PlaybackActions): Triple<ImageVector, String, () -> Unit> =
    when (button) {
        MiniButton.SHUFFLE -> Triple(NIcons.Shuffle, stringResource(R.string.shuffle), actions::toggleShuffle)
        MiniButton.PREVIOUS -> Triple(NIcons.Previous, stringResource(R.string.previous), actions::previous)
        MiniButton.PLAY_PAUSE -> if (ui.playing) {
            Triple(NIcons.Pause, stringResource(R.string.pause), actions::togglePause)
        } else {
            Triple(NIcons.Play, stringResource(R.string.play), actions::togglePause)
        }
        MiniButton.NEXT -> Triple(NIcons.Next, stringResource(R.string.next), actions::next)
        MiniButton.REPEAT -> Triple(
            if (ui.loop == LoopStatus.FILE) NIcons.RepeatOne else NIcons.Repeat,
            stringResource(
                when (ui.loop) {
                    LoopStatus.OFF -> R.string.repeat_off
                    LoopStatus.PLAYLIST -> R.string.repeat_all
                    LoopStatus.FILE -> R.string.repeat_one
                }
            ),
            actions::cycleRepeat,
        )
        MiniButton.OUTPUT -> Triple(NIcons.Output, stringResource(R.string.output), actions::openOutput)
        MiniButton.SLEEP_TIMER -> Triple(NIcons.SleepTimer, stringResource(R.string.sleep_timer), actions::openSleepTimer)
    }
