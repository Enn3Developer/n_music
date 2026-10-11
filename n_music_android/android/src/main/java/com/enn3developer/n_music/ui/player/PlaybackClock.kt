package com.enn3developer.n_music.ui.player

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.Position
import com.enn3developer.n_music.ui.components.rememberPlaybackSeconds

/**
 * Where playback is, in seconds, and seeking it. A seek shows where it goes from when it is sent
 * until the core reports it applied, so a bar let go of doesn't jump back meanwhile.
 */
@Stable
class PlaybackClock internal constructor(private val now: State<Double>) {
    /** A seek sent and not yet applied: its request, and where it goes. */
    private var pending by mutableStateOf<Pair<ULong, Double>?>(null)

    val seconds: Double get() = pending?.second ?: now.value

    fun seek(seconds: Double) {
        pending = CoreRepository.seek(seconds) to seconds
    }

    /** The core reported [position]: the pending seek is done once a report says it applied. */
    internal fun reported(position: Position) {
        val seek = pending ?: return
        if (position.seek >= seek.first) pending = null
    }
}

/** The clock of playback last reported at [position], running on while [playing]. */
@Composable
fun rememberPlaybackClock(position: Position, playing: Boolean): PlaybackClock {
    val now = rememberPlaybackSeconds(position, playing)
    val clock = remember(now) { PlaybackClock(now) }
    LaunchedEffect(clock, position) { clock.reported(position) }
    return clock
}
