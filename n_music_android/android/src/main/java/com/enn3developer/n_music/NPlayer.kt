package com.enn3developer.n_music

import android.os.Looper
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
    private var title = ""
    private var artist = ""
    private var durationMs = 0L
    private var artwork: ByteArray? = null

    override fun getState(): State {
        return State.Builder()
            .setAvailableCommands(COMMANDS)
            .setPlaybackState(playbackState)
            .setPlayWhenReady(playWhenReady, Player.PLAYBACK_SUPPRESSION_REASON_NONE)
            .setRepeatMode(repeatMode)
            .setShuffleModeEnabled(false)
            .setIsLoading(false)
            .setCurrentMediaItemIndex(currentIndex)
            .setContentPositionMs(positionMs)
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
        positionMs = 0
        playbackState = Player.STATE_IDLE
        playWhenReady = false
        title = ""
        artist = ""
        artwork = null
        durationMs = 0
        invalidateState()
    }

    fun updatePlayback(playing: Boolean, positionMs: Long) {
        playbackState = Player.STATE_READY
        playWhenReady = playing
        this.positionMs = positionMs
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
        when (seekCommand) {
            Player.COMMAND_SEEK_TO_NEXT,
            Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM -> MainActivity.mediaPlayNext()

            Player.COMMAND_SEEK_TO_PREVIOUS,
            Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> MainActivity.mediaPlayPrevious()

            else -> MainActivity.mediaSeek(positionMs / 1000.0)
        }
        currentIndex = mediaItemIndex.coerceIn(0, queue.lastIndex)
        this.positionMs = positionMs
        invalidateState()
        return Futures.immediateVoidFuture()
    }

    override fun handleSetRepeatMode(repeatMode: Int): ListenableFuture<*> {
        MainActivity.mediaRepeatMode(repeatMode)
        this.repeatMode = repeatMode
        invalidateState()
        return Futures.immediateVoidFuture()
    }
}
