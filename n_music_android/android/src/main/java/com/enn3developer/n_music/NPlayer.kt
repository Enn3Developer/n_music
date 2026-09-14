package com.enn3developer.n_music

import android.os.Looper
import android.os.SystemClock
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.SimpleBasePlayer.MediaItemData
import androidx.media3.common.SimpleBasePlayer.State
import androidx.media3.common.util.UnstableApi
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture

/**
 * Control surface for the native player. It never decodes audio itself: commands are forwarded to
 * the Rust side through [MainActivity] and the state is pushed back with the [setQueue],
 * [updatePlayback] and [updateMetadata] setters.
 */
@UnstableApi
class NPlayer(looper: Looper) : SimpleBasePlayer(looper) {
    companion object {
        val COMMANDS: Player.Commands = Player.Commands.Builder()
            .addAll(
                Player.COMMAND_PLAY_PAUSE,
                Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM,
                Player.COMMAND_SEEK_TO_MEDIA_ITEM,
                Player.COMMAND_SEEK_BACK,
                Player.COMMAND_SEEK_FORWARD,
                Player.COMMAND_SEEK_TO_NEXT,
                Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
                Player.COMMAND_SEEK_TO_PREVIOUS,
                Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
                Player.COMMAND_SET_REPEAT_MODE,
                Player.COMMAND_GET_CURRENT_MEDIA_ITEM,
                Player.COMMAND_GET_METADATA,
                Player.COMMAND_GET_TIMELINE,
            )
            .build()
    }

    private var queue: List<String> = listOf("")
    private var currentIndex = 0
    private var repeatMode = Player.REPEAT_MODE_ALL
    private var playbackState = Player.STATE_IDLE
    private var playWhenReady = false
    private var positionMs = 0L
    private var positionTimestampMs = SystemClock.elapsedRealtime()
    private var title = ""
    private var artist = ""
    private var durationMs = 0L
    private var artwork: ByteArray? = null

    private fun playbackClockActive(): Boolean =
        playWhenReady && playbackState == Player.STATE_READY

    private fun currentPositionMs(): Long {
        if (!playbackClockActive()) return positionMs
        return positionMs + (SystemClock.elapsedRealtime() - positionTimestampMs)
    }

    private fun setPosition(positionMs: Long) {
        this.positionMs = positionMs
        positionTimestampMs = SystemClock.elapsedRealtime()
    }

    override fun getState(): State {
        return State.Builder()
            .setAvailableCommands(COMMANDS)
            .setPlaybackState(playbackState)
            .setPlayWhenReady(playWhenReady, Player.PLAYBACK_SUPPRESSION_REASON_NONE)
            .setRepeatMode(repeatMode)
            .setShuffleModeEnabled(false)
            .setIsLoading(false)
            .setCurrentMediaItemIndex(currentIndex)
            .setContentPositionMs(currentPositionMs())
            .setPlaylist(queue.indices.map(::buildItem))
            .build()
    }

    private fun buildItem(index: Int): MediaItemData {
        val name = queue.getOrElse(index) { "" }
        val isCurrent = index == currentIndex
        val metadata = MediaMetadata.Builder()
            .setTitle(if (isCurrent && title.isNotEmpty()) title else name)
            .apply {
                if (isCurrent) {
                    if (artist.isNotEmpty()) setArtist(artist)
                    artwork?.let { setArtworkData(it, MediaMetadata.PICTURE_TYPE_FRONT_COVER) }
                }
            }
            .build()
        return MediaItemData.Builder(index)
            .setMediaItem(MediaItem.Builder().setMediaId(name).build())
            .setMediaMetadata(metadata)
            .setIsSeekable(true)
            .setDurationUs(if (isCurrent) durationMs * 1000 else 0L)
            .build()
    }

    fun setQueue(names: List<String>) {
        queue = names.ifEmpty { listOf("") }
        currentIndex = 0
        setPosition(0)
        playbackState = Player.STATE_IDLE
        playWhenReady = false
        title = ""
        artist = ""
        artwork = null
        durationMs = 0
        invalidateState()
    }

    private fun selectTrack(index: Int) {
        val next = index.coerceIn(0, queue.lastIndex)
        if (next == currentIndex) return
        currentIndex = next
        title = ""
        artist = ""
        artwork = null
        durationMs = 0
        invalidateState()
    }

    fun updateTrack(index: Int) {
        selectTrack(index)
    }

    fun updatePlayback(playing: Boolean, positionMs: Long) {
        playbackState = Player.STATE_READY
        playWhenReady = playing
        setPosition(positionMs)
        invalidateState()
    }

    fun updateMetadata(title: String, artist: String, artwork: ByteArray?, durationMs: Long) {
        this.title = title
        this.artist = artist
        this.artwork = artwork
        this.durationMs = durationMs
        invalidateState()
    }

    fun updateRepeatMode(mode: Int) {
        repeatMode = mode
        invalidateState()
    }

    override fun handleSetPlayWhenReady(playWhenReady: Boolean): ListenableFuture<*> {
        if (playWhenReady) MainActivity.mediaPlay() else MainActivity.mediaPause()
        setPosition(currentPositionMs())
        this.playWhenReady = playWhenReady
        this.playbackState = Player.STATE_READY
        invalidateState()
        return Futures.immediateVoidFuture()
    }

    override fun handlePrepare(): ListenableFuture<*> {
        playbackState = Player.STATE_READY
        invalidateState()
        return Futures.immediateVoidFuture()
    }

    override fun handleStop(): ListenableFuture<*> {
        setPosition(currentPositionMs())
        playbackState = Player.STATE_IDLE
        playWhenReady = false
        invalidateState()
        return Futures.immediateVoidFuture()
    }

    override fun handleSeek(
        mediaItemIndex: Int,
        positionMs: Long,
        seekCommand: Int,
    ): ListenableFuture<*> {
        val index = mediaItemIndex.coerceIn(0, queue.lastIndex)
        val position = if (positionMs == C.TIME_UNSET) 0L else positionMs
        when (seekCommand) {
            Player.COMMAND_SEEK_TO_NEXT,
            Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM -> {
                MainActivity.mediaPlayNext()
                selectTrack(index)
            }

            Player.COMMAND_SEEK_TO_PREVIOUS,
            Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> {
                MainActivity.mediaPlayPrevious()
                selectTrack(index)
            }

            Player.COMMAND_SEEK_TO_MEDIA_ITEM -> {
                // The index is forwarded unconditionally: the native player is the authority on
                // which track is active, and a queued changeTrack callback may lag behind it.
                selectTrack(index)
                MainActivity.mediaSeekTo(index, position / 1000.0)
            }

            else -> MainActivity.mediaSeek(position / 1000.0)
        }
        setPosition(position)
        invalidateState()
        return Futures.immediateVoidFuture()
    }

    override fun handleSetRepeatMode(repeatMode: Int): ListenableFuture<*> {
        // The native player only supports playlist and single-track looping, so requests for
        // REPEAT_MODE_OFF are normalized to REPEAT_MODE_ALL instead of being echoed back.
        val nativeMode =
            if (repeatMode == Player.REPEAT_MODE_ONE) Player.REPEAT_MODE_ONE else Player.REPEAT_MODE_ALL
        MainActivity.mediaRepeatMode(nativeMode)
        this.repeatMode = nativeMode
        invalidateState()
        return Futures.immediateVoidFuture()
    }
}
