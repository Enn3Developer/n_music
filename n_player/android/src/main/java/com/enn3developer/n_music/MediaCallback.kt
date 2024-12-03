package com.enn3developer.n_music

import android.media.session.MediaSession
import android.media.session.PlaybackState

// Called in mediaSession callback when we interact with notification
class MediaCallback(private val mediaSession: MediaSession,
                    private val playback: PlaybackState.Builder
) : MediaSession.Callback() {
    private external fun TogglePause()
    private external fun PlayNext()
    private external fun PlayPrevious()
    private external fun Seek(seek: Double)

    override fun onPause() {
        TogglePause()
        mediaSession.controller.playbackState?.position?.let {
            playback.setState(
                PlaybackState.STATE_PAUSED,
                it, 1.0f
            )
        }
        mediaSession.setPlaybackState(playback.build())
        super.onPause()
    }

    override fun onPlay() {
        TogglePause()
        mediaSession.controller.playbackState?.position?.let {
            playback.setState(
                PlaybackState.STATE_PLAYING,
                it, 1.0f)
        }
        mediaSession.setPlaybackState(playback.build())
        super.onPlay()
    }

    override fun onSkipToNext() {
        PlayNext()
        super.onSkipToNext()
    }

    override fun onSkipToPrevious() {
        PlayPrevious()
        playback.setState(PlaybackState.STATE_PLAYING, 0L, 1.0f)
        mediaSession.setPlaybackState(playback.build())
        super.onSkipToPrevious()
    }

    override fun onSeekTo(pos: Long) {
        Seek((pos / 1000).toDouble())
        playback.setState(PlaybackState.STATE_PLAYING, pos, 1.0f)
        mediaSession.setPlaybackState(playback.build())
        super.onSeekTo(pos)
    }
}