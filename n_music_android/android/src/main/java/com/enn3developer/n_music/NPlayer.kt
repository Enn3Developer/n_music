package com.enn3developer.n_music

import android.content.Context
import android.net.Uri
import android.os.Looper
import android.os.SystemClock
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.audio.AudioBecomingNoisyManager
import androidx.media3.common.audio.AudioFocusManager
import androidx.media3.common.util.Clock
import androidx.annotation.OptIn
import androidx.media3.common.util.UnstableApi
import androidx.media3.common.util.WakeLockManager
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.ItemId
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.core.QueueRow
import com.enn3developer.n_music.core.Seek
import com.enn3developer.n_music.core.TrackRow
import com.google.common.collect.ImmutableList
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.google.common.util.concurrent.SettableFuture
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import java.io.File
import kotlin.math.abs

/**
 * The native player as Media3 sees it, for the session, its notification and media buttons. It
 * never decodes audio: commands go to the core through [CoreRepository], and the core's state
 * comes back through its flows.
 *
 * It also does what ExoPlayer does for itself: holds audio focus while playing, pauses when the
 * output becomes noisy (headphones unplugged) and keeps the CPU awake while playing, so the
 * native decoder keeps up with the screen off.
 */
@OptIn(UnstableApi::class)
class NPlayer(context: Context) : SimpleBasePlayer(Looper.getMainLooper()) {
    private companion object {
        val COMMANDS: Player.Commands = Player.Commands.Builder()
            .addAll(
                Player.COMMAND_PLAY_PAUSE,
                Player.COMMAND_PREPARE,
                Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM,
                Player.COMMAND_SEEK_TO_MEDIA_ITEM,
                Player.COMMAND_SEEK_BACK,
                Player.COMMAND_SEEK_FORWARD,
                Player.COMMAND_SEEK_TO_NEXT,
                Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
                Player.COMMAND_SEEK_TO_PREVIOUS,
                Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
                Player.COMMAND_SET_REPEAT_MODE,
                Player.COMMAND_SET_SHUFFLE_MODE,
                Player.COMMAND_GET_CURRENT_MEDIA_ITEM,
                Player.COMMAND_GET_METADATA,
                Player.COMMAND_GET_TIMELINE,
            )
            .build()

        /** How far the playback clock may drift from a reported position before it is reset. */
        const val MAX_DRIFT_MS = 1000L

        /** How long a command waits for the core to confirm it before Media3 hears the state. */
        const val CONFIRM_TIMEOUT_MS = 1000L

        val EMPTY: ImmutableList<MediaItemData> = ImmutableList.of(
            MediaItemData.Builder("empty").setMediaItem(MediaItem.EMPTY).build()
        )
    }

    private val scope = MainScope()
    private val focus = AudioFocusManager(
        context,
        Looper.getMainLooper(),
        object : AudioFocusManager.PlayerControl {
            // Android ducks the stream by itself, and the core's volume is the user's setting.
            override fun setVolumeMultiplier(volumeMultiplier: Float) {}

            override fun executePlayerCommand(playerCommand: Int) = onFocusChanged(playerCommand)
        },
    )
    private val noisy = AudioBecomingNoisyManager(
        context,
        Looper.getMainLooper(),
        Looper.getMainLooper(),
        { CoreRepository.send(Command.Pause) },
        Clock.DEFAULT,
    )
    private val wakeLock = WakeLockManager(context, Looper.getMainLooper(), Clock.DEFAULT)

    private var playing = false

    /** Paused for a passing focus loss, like a call or a navigation prompt: play after it. */
    private var resumeOnFocusGain = false
    private var queue: List<QueueRow> = emptyList()
    private var current: Current? = null
    private var playlist: ImmutableList<MediaItemData> = EMPTY
    private var currentIndex = 0

    /** The core reported a play session: Media3 may show it. */
    private var ready = false
    private var repeatMode = Player.REPEAT_MODE_ALL
    private var shuffleModeEnabled = false

    /** The current track's length from the decoder, which knows it before the tags do. */
    private var length = 0.0
    private var positionMs = 0L
    private var positionTimestampMs = SystemClock.elapsedRealtime()

    init {
        focus.setAudioAttributes(
            AudioAttributes.Builder()
                .setUsage(C.USAGE_MEDIA)
                .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC)
                .build()
        )
        wakeLock.setEnabled(true)
        scope.launch { CoreRepository.playing.collect(::onPlaying) }
        scope.launch { CoreRepository.queue.collect(::onQueue) }
        scope.launch { CoreRepository.current.collect(::onCurrent) }
        scope.launch { CoreRepository.position.collect(::onPosition) }
        scope.launch {
            CoreRepository.loopStatus.collect {
                repeatMode = it.toRepeatMode()
                invalidateState()
            }
        }
        scope.launch {
            CoreRepository.shuffle.collect {
                shuffleModeEnabled = it
                invalidateState()
            }
        }
    }

    private fun currentPositionMs(): Long =
        if (playing) positionMs + (SystemClock.elapsedRealtime() - positionTimestampMs) else positionMs

    private fun setPosition(positionMs: Long, at: Long = SystemClock.elapsedRealtime()) {
        this.positionMs = positionMs
        positionTimestampMs = at
    }

    override fun getState(): State = State.Builder()
        .setAvailableCommands(COMMANDS)
        .setPlaybackState(if (ready) Player.STATE_READY else Player.STATE_IDLE)
        .setPlayWhenReady(playing, Player.PLAY_WHEN_READY_CHANGE_REASON_USER_REQUEST)
        .setRepeatMode(repeatMode)
        .setShuffleModeEnabled(shuffleModeEnabled)
        .setPlaylist(playlist)
        .setCurrentMediaItemIndex(currentIndex)
        .setContentPositionMs(
            SimpleBasePlayer.PositionSupplier.getExtrapolating(
                currentPositionMs(),
                if (playing) 1f else 0f,
            )
        )
        .build()

    private fun onPlaying(playing: Boolean) {
        if (playing == this.playing) return
        setPosition(currentPositionMs())
        this.playing = playing
        if (playing) {
            ready = true
            // Playback starts from the app, the notification or a headset alike: take focus
            // whichever way it started.
            when (focus.updateAudioFocus(true, Player.STATE_READY)) {
                AudioFocusManager.PLAYER_COMMAND_DO_NOT_PLAY -> CoreRepository.send(Command.Pause)
                AudioFocusManager.PLAYER_COMMAND_WAIT_FOR_CALLBACK -> {
                    resumeOnFocusGain = true
                    CoreRepository.send(Command.Pause)
                }

                else -> resumeOnFocusGain = false
            }
        }
        noisy.setEnabled(playing)
        wakeLock.setStayAwake(playing)
        invalidateState()
    }

    private fun onFocusChanged(command: Int) {
        when (command) {
            AudioFocusManager.PLAYER_COMMAND_DO_NOT_PLAY -> {
                resumeOnFocusGain = false
                if (playing) CoreRepository.send(Command.Pause)
            }

            AudioFocusManager.PLAYER_COMMAND_WAIT_FOR_CALLBACK -> if (playing) {
                resumeOnFocusGain = true
                CoreRepository.send(Command.Pause)
            }

            AudioFocusManager.PLAYER_COMMAND_PLAY_WHEN_READY -> if (resumeOnFocusGain) {
                resumeOnFocusGain = false
                CoreRepository.send(Command.Play)
            }
        }
    }

    private fun onQueue(queue: List<QueueRow>) {
        if (queue === this.queue) return
        this.queue = queue
        playlist = if (queue.isEmpty()) {
            EMPTY
        } else {
            ImmutableList.copyOf(queue.map { item(it.item, it.track, it.track.length) })
        }
        if (queue.isNotEmpty()) ready = true
        updateCurrent()
    }

    private fun onCurrent(current: Current?) {
        if (current === this.current) return
        if (current?.item != this.current?.item) length = 0.0
        this.current = current
        updateCurrent()
    }

    /** Points at the current item and gives it the freshest metadata, from `TrackChanged`. */
    private fun updateCurrent() {
        val current = current
        val index = queue.indexOfFirst { it.item == current?.item }
        currentIndex = index.coerceAtLeast(0)
        if (current != null && index >= 0) {
            val updated = item(current.item, current.track, length)
            if (playlist[index] != updated) {
                playlist = ImmutableList.builderWithExpectedSize<MediaItemData>(playlist.size)
                    .addAll(playlist.subList(0, index))
                    .add(updated)
                    .addAll(playlist.subList(index + 1, playlist.size))
                    .build()
            }
        }
        invalidateState()
    }

    private fun onPosition(position: Position) {
        if (position.length != length) {
            length = position.length
            updateCurrent()
        }
        val reportedMs = (position.position * 1000).toLong()
        val expectedMs = if (playing) {
            reportedMs + (SystemClock.elapsedRealtime() - position.at)
        } else {
            reportedMs
        }
        if (position.discontinuity || abs(currentPositionMs() - expectedMs) > MAX_DRIFT_MS) {
            setPosition(reportedMs, position.at)
            invalidateState()
        }
    }

    private fun item(id: ItemId, track: TrackRow, length: Double): MediaItemData {
        val metadata = MediaMetadata.Builder()
            .setTitle(track.title)
            .setArtist(track.artist.ifEmpty { null })
            .setAlbumTitle(track.album)
            .setArtworkUri(track.cover?.let { Uri.fromFile(File(it)) })
            .build()
        val seconds = if (length > 0) length else track.length
        return MediaItemData.Builder(id)
            .setMediaItem(
                MediaItem.Builder()
                    .setMediaId(id.toString())
                    .setMediaMetadata(metadata)
                    .build()
            )
            .setIsSeekable(true)
            .setDurationUs(if (seconds > 0) (seconds * 1_000_000).toLong() else C.TIME_UNSET)
            .build()
    }

    /**
     * A future that completes once the core's state satisfies [done], or after
     * [CONFIRM_TIMEOUT_MS]. Until then Media3 shows the state it expects, so the notification
     * does not flicker back while the core catches up.
     */
    private fun confirmed(done: () -> Boolean): ListenableFuture<*> {
        val future = SettableFuture.create<Unit>()
        scope.launch {
            withTimeoutOrNull(CONFIRM_TIMEOUT_MS) {
                combine(
                    CoreRepository.playing,
                    CoreRepository.current,
                    CoreRepository.queue,
                ) { _, _, _ -> done() }.first { it }
            }
            // The collectors may not have run yet: catch up before Media3 reads the state.
            onQueue(CoreRepository.queue.value)
            onCurrent(CoreRepository.current.value)
            onPlaying(CoreRepository.playing.value)
            future.set(Unit)
        }
        return future
    }

    override fun handleSetPlayWhenReady(playWhenReady: Boolean): ListenableFuture<*> {
        if (!playWhenReady) {
            resumeOnFocusGain = false
            CoreRepository.send(Command.Pause)
            return confirmed { !CoreRepository.playing.value }
        }
        when (focus.updateAudioFocus(true, Player.STATE_READY)) {
            AudioFocusManager.PLAYER_COMMAND_PLAY_WHEN_READY -> {
                resumeOnFocusGain = false
                CoreRepository.send(Command.Play)
                return confirmed { CoreRepository.playing.value }
            }

            AudioFocusManager.PLAYER_COMMAND_WAIT_FOR_CALLBACK -> resumeOnFocusGain = true
        }
        return Futures.immediateVoidFuture()
    }

    override fun handlePrepare(): ListenableFuture<*> = Futures.immediateVoidFuture()

    override fun handleSeek(
        mediaItemIndex: Int,
        positionMs: Long,
        seekCommand: Int,
    ): ListenableFuture<*> {
        val position = if (positionMs == C.TIME_UNSET) 0L else positionMs
        val before = CoreRepository.current.value
        val changesItem = when (seekCommand) {
            // Media3 guesses the neighbour in the list, but repeat, up next and the end of the
            // queue decide otherwise: the core reports where it went.
            Player.COMMAND_SEEK_TO_NEXT,
            Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM -> {
                CoreRepository.send(Command.PlayNext)
                true
            }

            Player.COMMAND_SEEK_TO_PREVIOUS,
            Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> {
                CoreRepository.send(Command.PlayPrevious)
                true
            }

            Player.COMMAND_SEEK_TO_MEDIA_ITEM -> {
                val item = queue.getOrNull(mediaItemIndex)?.item
                if (item != null) {
                    CoreRepository.send(Command.Seek(Seek.ToItem(item, position / 1000.0)))
                }
                item != null && item != before?.item
            }

            else -> {
                CoreRepository.send(Command.Seek(Seek.Absolute(position / 1000.0)))
                false
            }
        }
        setPosition(position)
        if (changesItem) return confirmed { CoreRepository.current.value !== before }
        invalidateState()
        return Futures.immediateVoidFuture()
    }

    override fun handleSetRepeatMode(repeatMode: Int): ListenableFuture<*> {
        val status = when (repeatMode) {
            Player.REPEAT_MODE_ONE -> LoopStatus.FILE
            Player.REPEAT_MODE_ALL -> LoopStatus.PLAYLIST
            else -> LoopStatus.OFF
        }
        CoreRepository.send(Command.SetLoopStatus(status))
        this.repeatMode = repeatMode
        return Futures.immediateVoidFuture()
    }

    override fun handleSetShuffleModeEnabled(shuffleModeEnabled: Boolean): ListenableFuture<*> {
        CoreRepository.send(Command.SetShuffle(shuffleModeEnabled))
        this.shuffleModeEnabled = shuffleModeEnabled
        return Futures.immediateVoidFuture()
    }

    override fun handleRelease(): ListenableFuture<*> {
        scope.cancel()
        focus.release()
        noisy.setEnabled(false)
        wakeLock.setEnabled(false)
        return Futures.immediateVoidFuture()
    }

    /** What to play when the system or a headset resumes: the session as the core has it. */
    fun resumptionItems(): Triple<List<MediaItem>, Int, Long> =
        Triple(
            playlist.map { it.mediaItem },
            currentIndex,
            currentPositionMs(),
        )
}

private fun LoopStatus.toRepeatMode(): Int = when (this) {
    LoopStatus.OFF -> Player.REPEAT_MODE_OFF
    LoopStatus.PLAYLIST -> Player.REPEAT_MODE_ALL
    LoopStatus.FILE -> Player.REPEAT_MODE_ONE
}
