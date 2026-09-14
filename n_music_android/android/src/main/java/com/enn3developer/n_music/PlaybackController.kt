package com.enn3developer.n_music

import android.os.Handler
import android.os.Looper
import androidx.media3.common.util.UnstableApi

/**
 * Single source of truth for the media3 player shared by the activity and the session service.
 * All mutations are posted to the main thread because media3 requires commands to run on the
 * player's application thread.
 */
@UnstableApi
object PlaybackController {
    private val handler = Handler(Looper.getMainLooper())
    private val player: NPlayer by lazy { NPlayer(Looper.getMainLooper()) }

    fun player(): NPlayer = player

    fun updatePlayback(playing: Boolean, positionMs: Long) {
        handler.post { player.updatePlayback(playing, positionMs) }
    }

    fun updateMetadata(title: String, artist: String, artwork: ByteArray?, durationMs: Long) {
        handler.post { player.updateMetadata(title, artist, artwork, durationMs) }
    }

    fun setQueue(names: List<String>) {
        handler.post { player.setQueue(names) }
    }

    fun setRepeatMode(mode: Int) {
        handler.post { player.updateRepeatMode(mode) }
    }
}
