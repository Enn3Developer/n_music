package com.enn3developer.n_music

import android.os.SystemClock
import com.enn3developer.n_music.core.Command
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.json.JSONObject

/** What a sleep timer waits for. */
sealed interface SleepMode {
    /** A number of minutes. */
    data class After(val minutes: Int) : SleepMode

    /** The end of the track playing when it was set. */
    data object TrackEnd : SleepMode

    /** The end of the queue's last track, even on repeat. */
    data object QueueEnd : SleepMode
}

/**
 * A running sleep timer: the [option] picked, `null` once added time makes it none of them, and
 * when it ends in [SystemClock.elapsedRealtime] milliseconds, for a timer on the clock. [item]
 * played when it was set; [lastPlayed] tells the queue's last item played, for [SleepMode.QueueEnd].
 */
data class SleepState(
    val option: SleepMode?,
    val ends: Long?,
    val item: ULong?,
    val lastPlayed: Boolean = false,
)

/**
 * The sleep timer, for the whole process, so it runs on with the screen off: playback fades out
 * over its last 10 seconds, then pauses, and the volume comes back for the next time it plays.
 */
object SleepTimer {
    /** How long the fade out takes. */
    const val FADE_MS = 10_000L

    /** How close to the end a track pauses, so the next one never starts. */
    private const val MARGIN_MS = 150L

    /** Where the volume before a fade is kept while it fades, in case the process ends meanwhile. */
    private const val KEY = "android.sleep"

    private val scope = MainScope()
    private val _state = MutableStateFlow<SleepState?>(null)
    val state: StateFlow<SleepState?> = _state.asStateFlow()
    private var job: Job? = null

    /** The volume before the fade, to put back once paused. */
    private var volume: Double? = null

    /** Puts back the volume a fade left lowered when the process ended during it. */
    fun recover() {
        val stored = CoreRepository.setting(KEY)?.let { runCatching { JSONObject(it) }.getOrNull() } ?: return
        if (!stored.has("volume")) return
        CoreRepository.send(Command.SetVolume(stored.getDouble("volume")))
        CoreRepository.setSetting(KEY, "{}")
    }

    fun start(mode: SleepMode) {
        val now = SystemClock.elapsedRealtime()
        val ends = if (mode is SleepMode.After) now + mode.minutes * 60_000L else null
        run(SleepState(mode, ends, CoreRepository.current.value?.item))
    }

    /** Ten more minutes: on the clock, or after what was left until the track or queue ends. */
    fun extend() {
        val state = _state.value ?: return
        val now = SystemClock.elapsedRealtime()
        val left = remaining(state, now, assumePlaying = true) ?: 0L
        val option = state.option.takeIf { it is SleepMode.After }
        run(state.copy(option = option, ends = now + left.coerceAtLeast(0L) + 10 * 60_000L))
    }

    fun cancel() {
        job?.cancel()
        restoreVolume()
        _state.value = null
    }

    /**
     * How long until [state] pauses playback, in milliseconds; `null` while that waits on a track
     * that is not playing. [assumePlaying] counts a paused track's rest as if it played on.
     */
    fun remaining(state: SleepState, now: Long, assumePlaying: Boolean = false): Long? {
        state.ends?.let { return it - now }
        val current = CoreRepository.current.value ?: return 0L
        return when (state.option) {
            SleepMode.TrackEnd -> if (current.item != state.item) 0L else trackLeft(now, assumePlaying)
            SleepMode.QueueEnd -> {
                val last = CoreRepository.queue.value.lastOrNull()?.item
                when {
                    current.item == last -> trackLeft(now, assumePlaying)
                    // Past the last item, as repeat goes back to the first: the queue ended.
                    state.lastPlayed -> 0L
                    else -> null
                }
            }
            else -> null
        }
    }

    /** What is left of the current track: `null` while it is paused, unless [assumePlaying]. */
    private fun trackLeft(now: Long, assumePlaying: Boolean): Long? {
        val playing = CoreRepository.playing.value
        if (!playing && !assumePlaying) return null
        val position = CoreRepository.position.value
        val length = position.length.takeIf { it > 0 } ?: CoreRepository.current.value?.track?.length ?: return null
        val at = position.position + if (playing) (now - position.at) / 1000.0 else 0.0
        return ((length - at) * 1000).toLong()
    }

    private fun run(state: SleepState) {
        job?.cancel()
        _state.value = state
        job = scope.launch {
            var current = state
            while (true) {
                val now = SystemClock.elapsedRealtime()
                // The queue's last item playing: once another plays, the queue ended.
                if (current.option == SleepMode.QueueEnd && !current.lastPlayed &&
                    CoreRepository.current.value?.item == CoreRepository.queue.value.lastOrNull()?.item
                ) {
                    current = current.copy(lastPlayed = true)
                    _state.value = current
                }
                val left = remaining(current, now)
                when {
                    left != null && left <= MARGIN_MS -> {
                        stop()
                        return@launch
                    }
                    left != null && left <= FADE_MS -> fade(left)
                    else -> restoreVolume()
                }
                delay(if (left != null && left <= FADE_MS + 1_000) 100 else 500)
            }
        }
    }

    /** Lowers the volume to where it is [left] milliseconds from the end of the fade. */
    private fun fade(left: Long) {
        val full = volume ?: CoreRepository.volume.value.also {
            volume = it
            CoreRepository.setSetting(KEY, JSONObject().put("volume", it).toString())
        }
        val share = (left.toDouble() / FADE_MS).coerceIn(0.0, 1.0)
        // Squared, so the fade sounds even to the ear rather than dropping off at the end.
        CoreRepository.send(Command.SetVolume(full * share * share))
    }

    private fun restoreVolume() {
        val full = volume ?: return
        volume = null
        CoreRepository.send(Command.SetVolume(full))
        CoreRepository.setSetting(KEY, "{}")
    }

    private suspend fun stop() {
        CoreRepository.send(Command.Pause)
        // The pause comes first, so the volume coming back is not heard.
        delay(300)
        restoreVolume()
        _state.value = null
    }
}
