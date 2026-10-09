package com.enn3developer.n_music.ui.sheets

import android.os.SystemClock
import android.text.format.DateFormat
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.SleepMode
import com.enn3developer.n_music.SleepState
import com.enn3developer.n_music.SleepTimer
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.PillButton
import com.enn3developer.n_music.ui.components.PillStyle
import com.enn3developer.n_music.ui.components.RadioMark
import com.enn3developer.n_music.ui.components.SheetFrame
import com.enn3developer.n_music.ui.components.margins
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.formatPosition
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import kotlinx.coroutines.delay
import java.util.Date

/** The times a sleep timer offers, in minutes. */
private val Durations = listOf(15, 30, 45, 60)

/**
 * The milliseconds of [SystemClock.elapsedRealtime], ticking every second while shown, for
 * what counts down.
 */
@Composable
fun rememberClock(): Long {
    val now = remember { mutableLongStateOf(SystemClock.elapsedRealtime()) }
    LaunchedEffect(Unit) {
        while (true) {
            now.longValue = SystemClock.elapsedRealtime()
            delay(1_000 - now.longValue % 1_000)
        }
    }
    return now.longValue
}

/**
 * The sleep timer: a time, the end of the track, or the end of the queue, after which playback
 * fades out and pauses. Picking one starts it and closes the sheet; while one runs, its time left
 * shows by the title, and it can take ten more minutes or be turned off.
 */
@Composable
fun SleepTimerSheet(open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val state by SleepTimer.state.collectAsStateWithLifecycle()
    val queue by CoreRepository.queue.collectAsStateWithLifecycle()
    val current by CoreRepository.current.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val now = rememberClock()
    val running = state
    val left = running?.let { SleepTimer.remaining(it, now, assumePlaying = true) }
    val index = queue.indexOfFirst { it.item == current?.item }
    val trackLeft = SleepTimer.remaining(SleepState(SleepMode.TrackEnd, null, current?.item), now, assumePlaying = true)
    SleepTimerSheet(
        selected = running?.option,
        left = left?.let { formatPosition(it.coerceAtLeast(0) / 1000.0) },
        until = running?.ends?.let { ends ->
            DateFormat.getTimeFormat(context).format(Date(System.currentTimeMillis() + (ends - now)))
        },
        trackLeft = trackLeft?.let { formatPosition(it.coerceAtLeast(0) / 1000.0) },
        tracksLeft = if (index < 0) queue.size else queue.size - index - 1,
        onPick = {
            SleepTimer.start(it)
            onDismissRequest()
        },
        onExtend = SleepTimer::extend,
        onTurnOff = {
            SleepTimer.cancel()
            onDismissRequest()
        },
        open = open,
        onDismissRequest = onDismissRequest,
        onGone = onGone,
    )
}

/**
 * The sheet itself: [selected] is the option running, [left] its time left and [until] the
 * time of day it ends, while on the clock; [trackLeft] and [tracksLeft] tell how long the track
 * and the queue have to go.
 */
@Composable
fun SleepTimerSheet(
    selected: SleepMode?,
    left: String?,
    until: String?,
    trackLeft: String?,
    tracksLeft: Int,
    onPick: (SleepMode) -> Unit,
    onExtend: () -> Unit,
    onTurnOff: () -> Unit,
    open: Boolean,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
) {
    val title = stringResource(R.string.sleep_timer)
    SheetFrame(
        open, title, onDismissRequest, onGone,
        // The design's 24 dp under the buttons take in the gesture area's 16.
        end = 8.dp,
        header = {
            // 2 dp higher than the short sheets' handle leaves it, as the design's is shorter.
            Row(
                Modifier
                    .fillMaxWidth()
                    .margins(top = 2.dp)
                    .heightIn(min = 32.dp)
                    .padding(horizontal = 24.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text(title, style = text(22, FontWeight.Bold), color = colors.onSurface, modifier = Modifier.weight(1f))
                if (left != null) {
                    Row(
                        Modifier
                            .height(32.dp)
                            .background(colors.secondaryContainer, RoundedCornerShape(16.dp))
                            .padding(start = 10.dp, end = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        NIcon(NIcons.SleepTimer, size = 16.dp, tint = colors.onSecondaryContainer)
                        Text(
                            stringResource(R.string.sleep_left, left),
                            style = text(14, FontWeight.Bold, tabular = true),
                            color = colors.onSecondaryContainer,
                        )
                    }
                }
            }
            Text(
                stringResource(R.string.sleep_fades),
                style = text(14, lineHeight = 20.sp),
                color = colors.onSurfaceVariant,
                modifier = Modifier.padding(start = 24.dp, end = 24.dp, top = 6.dp, bottom = 4.dp),
            )
        },
    ) {
        Column(Modifier.selectableGroup()) {
            for (minutes in Durations) {
                val option = SleepMode.After(minutes)
                Option(
                    label = if (minutes % 60 == 0) {
                        pluralStringResource(R.plurals.sleep_hours, minutes / 60, minutes / 60)
                    } else {
                        pluralStringResource(R.plurals.sleep_minutes, minutes, minutes)
                    },
                    hint = until?.takeIf { selected == option }?.let { stringResource(R.string.sleep_until, it) },
                    selected = selected == option,
                    onClick = { onPick(option) },
                )
            }
            Option(
                label = stringResource(R.string.sleep_track_end),
                hint = trackLeft?.let { stringResource(R.string.sleep_left, it) },
                selected = selected == SleepMode.TrackEnd,
                onClick = { onPick(SleepMode.TrackEnd) },
            )
            Option(
                label = stringResource(R.string.sleep_queue_end),
                hint = pluralStringResource(R.plurals.sleep_queue_tracks, quantity(tracksLeft), formatCount(tracksLeft)),
                selected = selected == SleepMode.QueueEnd,
                onClick = { onPick(SleepMode.QueueEnd) },
            )
        }
        if (left != null) {
            Row(
                Modifier
                    .fillMaxWidth()
                    .padding(start = 24.dp, end = 24.dp, top = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                PillButton(
                    stringResource(R.string.sleep_extend),
                    onExtend,
                    Modifier.weight(1f),
                    style = PillStyle.OUTLINED,
                    padding = PaddingValues(horizontal = 8.dp),
                )
                PillButton(
                    stringResource(R.string.sleep_off),
                    onTurnOff,
                    Modifier.weight(1f),
                    style = PillStyle.TONAL,
                    padding = PaddingValues(horizontal = 8.dp),
                )
            }
        }
    }
}

/** One of the timer's options: a radio, its name, and a [hint] at the row's end. */
@Composable
private fun Option(label: String, hint: String?, selected: Boolean, onClick: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .height(56.dp)
            .selectable(selected, role = Role.RadioButton, onClick = onClick)
            .padding(horizontal = 24.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        RadioMark(selected)
        Text(
            label,
            style = text(16, if (selected) FontWeight.Bold else FontWeight.Medium),
            color = colors.onSurface,
            maxLines = 1,
            modifier = Modifier.weight(1f),
        )
        if (hint != null) {
            Text(hint, style = text(13, FontWeight.Medium, tabular = true), color = colors.onSurfaceVariant, maxLines = 1)
        }
    }
}
