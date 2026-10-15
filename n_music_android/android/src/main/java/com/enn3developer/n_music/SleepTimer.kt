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
import kotlin.math.abs

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
 * over its last 10 seconds, then pauses, and plays at full volume the next time. The core fades
 * the audio as it queues it, so the fade lands on time however much the output holds.
 */
object SleepTimer {
    /** How long the fade out takes. */
    const val FADE_MS = 10_000L

    /** How close to the end a track pauses, so the next one never starts. */
    private const val MARGIN_MS = 150L

    /**
     * How far the end may move before the core hears of it: a track's end shifts a little with
     * every position, and a new end reaching queued audio queues it again.
     */
    private const val RESEND_MS = 250L

    /** Where older versions kept the volume before their fade, while it faded. */
    private const val KEY = "android.sleep"

    private val scope = MainScope()
    private val _state = MutableStateFlow<SleepState?>(null)
    val state: StateFlow<SleepState?> = _state.asStateFlow()
    private var job: Job? = null

    /** When the fade the core has ends, in [SystemClock.elapsedRealtime] milliseconds. */
    private var fadeEnd: Long? = null

    /** Puts back the volume an older version's fade left lowered when the process ended. */
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
        clearFade()
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
                    left != null -> fadeBy(now + left - MARGIN_MS, now)
                    // Not on the queue's last track yet. A paused track keeps its fade.
                    CoreRepository.playing.value -> clearFade()
                }
                delay(if (left != null && left <= FADE_MS + 1_000) 100 else 500)
            }
        }
    }

    /** Has the core fade playback out to silence by [end], when playback pauses. */
    private fun fadeBy(end: Long, now: Long) {
        val sent = fadeEnd
        if (sent != null && abs(end - sent) < RESEND_MS) return
        fadeEnd = end
        CoreRepository.setSleepFade(end - now, FADE_MS)
    }

    private fun clearFade() {
        if (fadeEnd == null) return
        fadeEnd = null
        CoreRepository.setSleepFade(null, FADE_MS)
    }

    private suspend fun stop() {
        CoreRepository.send(Command.Pause)
        // The pause comes first, so the audio queued again at full volume is not heard.
        delay(300)
        clearFade()
        _state.value = null
    }
}
