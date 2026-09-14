package com.enn3developer.n_music

import android.os.Looper
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.util.UnstableApi

@UnstableApi
class NPlayer(looper: Looper) : SimpleBasePlayer(looper) {
    override fun getState(): State {
        return State.Builder().setVolume(1.0f).setRepeatMode(REPEAT_MODE_ALL)
            .setPlaybackState(STATE_IDLE)
            .build()
    }
}