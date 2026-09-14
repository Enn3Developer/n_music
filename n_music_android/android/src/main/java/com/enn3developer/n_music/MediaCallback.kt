package com.enn3developer.n_music

import android.media.session.MediaSession

class MediaCallback : MediaSession.Callback() {
    private external fun Pause()
    private external fun Play()
    private external fun PlayNext()
    private external fun PlayPrevious()
    private external fun Seek(seek: Double)

    override fun onPause() { Pause() }
    override fun onPlay() { Play() }
    override fun onSkipToNext() { PlayNext() }
    override fun onSkipToPrevious() { PlayPrevious() }
    override fun onSeekTo(pos: Long) { Seek(pos.toDouble() / 1000.0) }
}
