package com.enn3developer.n_music.ui.player

import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitHorizontalTouchSlopOrCancellation
import androidx.compose.foundation.gestures.horizontalDrag
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.setProgress
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.components.WavyProgress
import com.enn3developer.n_music.ui.components.margins
import com.enn3developer.n_music.ui.formatLength
import com.enn3developer.n_music.ui.formatPosition
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** How far the bar's touch area reaches above and below it. */
private val Reach = 10.dp

/**
 * Where the track is: a wave up to the position and a line after it, with the position and the
 * length under them. Dragging along it or tapping it seeks once let go; meanwhile the bar and the
 * position show where it would go. [wave] flattens the wave from 1 to 0, [phase] makes it travel.
 * The length rolls when another [item] plays, the way [skip] went.
 */
@Composable
fun SeekBar(
    seconds: () -> Double,
    length: Double,
    onSeek: (Double) -> Unit,
    modifier: Modifier = Modifier,
    item: ULong = 0u,
    skip: () -> Int = { 1 },
    wave: () -> Float = { 1f },
    phase: () -> Float = { 0f },
) {
    // Where the finger holds the bar, from 0 to 1, while it drags.
    var held by remember { mutableStateOf<Float?>(null) }
    val seek by rememberUpdatedState(onSeek)
    fun fraction(): Float = held ?: if (length > 0) (seconds() / length).toFloat().coerceIn(0f, 1f) else 0f
    // The times change once a second, not every frame.
    val shown by remember(length) { derivedStateOf { (held?.let { it * length } ?: seconds()).toLong() } }
    val position = formatPosition(shown.toDouble())
    val total = formatLength(length).ifEmpty { formatPosition(0.0) }
    val description = stringResource(R.string.position_of, position, total)
    Column(modifier) {
        WavyProgress(
            progress = ::fraction,
            modifier = Modifier
                .fillMaxWidth()
                .margins(top = Reach, bottom = Reach)
                .pointerInput(length) {
                    // The thumb's middle runs between these, as the bar draws it.
                    val inset = (1.dp + 2.5.dp).toPx()
                    fun at(x: Float) = ((x - inset) / (size.width - 2 * inset)).coerceIn(0f, 1f)
                    awaitEachGesture {
                        val down = awaitFirstDown()
                        if (length <= 0) return@awaitEachGesture
                        val drag = awaitHorizontalTouchSlopOrCancellation(down.id) { change, _ -> change.consume() }
                        if (drag == null) {
                            // A tap seeks too; a drag taken by the player's own is no tap.
                            val up = currentEvent.changes.firstOrNull { it.id == down.id }
                            if (up != null && up.changedToUp() && !up.isConsumed) {
                                up.consume()
                                seek(at(down.position.x) * length)
                            }
                            return@awaitEachGesture
                        }
                        held = at(drag.position.x)
                        val done = horizontalDrag(drag.id) { change ->
                            held = at(change.position.x)
                            change.consume()
                        }
                        val last = held
                        if (done && last != null) seek(last * length)
                        held = null
                    }
                }
                .semantics {
                    contentDescription = description
                    progressBarRangeInfo = ProgressBarRangeInfo(if (length > 0) (shown / length).toFloat() else 0f, 0f..1f)
                    setProgress { target ->
                        if (length > 0) seek(target.coerceIn(0f, 1f) * length)
                        length > 0
                    }
                }
                .padding(vertical = Reach)
                .height(28.dp),
            color = colors.primary,
            trackColor = colors.secondaryContainer,
            stroke = 4.dp,
            amplitude = { 3.dp * wave() },
            wavelength = 16.dp,
            gap = 5.dp,
            stopDot = false,
            phase = phase,
            thumb = true,
            taper = true,
        )
        Row(
            Modifier
                .fillMaxWidth()
                .padding(top = 4.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(position, style = text(13, tabular = true), color = colors.onSurfaceVariant)
            val shift = with(LocalDensity.current) { 10.dp.roundToPx() }
            AnimatedContent(
                targetState = item to total,
                contentKey = { it.first },
                transitionSpec = { roll(skip(), shift) },
                label = "length",
            ) { (_, shown) ->
                Text(shown, style = text(13, tabular = true), color = colors.onSurfaceVariant)
            }
        }
    }
}
