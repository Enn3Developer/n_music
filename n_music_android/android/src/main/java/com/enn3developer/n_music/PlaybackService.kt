package com.enn3developer.n_music

import android.app.PendingIntent
import android.content.Intent
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Handler
import android.os.Looper
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService

@UnstableApi
class PlaybackService : MediaSessionService() {
    private var mediaSession: MediaSession? = null
    private val handler = Handler(Looper.getMainLooper())
    private var audioManager: AudioManager? = null
    private var outputDeviceIds = emptySet<Int>()
    private val notifyOutputDeviceChanged = Runnable { MainActivity.outputDeviceChanged() }
    private val audioDeviceCallback = object : AudioDeviceCallback() {
        override fun onAudioDevicesAdded(addedDevices: Array<out AudioDeviceInfo>) {
            updateOutputDevices()
        }

        override fun onAudioDevicesRemoved(removedDevices: Array<out AudioDeviceInfo>) {
            updateOutputDevices()
        }
    }

    override fun onCreate() {
        super.onCreate()
        val sessionActivity = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val session = MediaSession.Builder(this, PlaybackController.player())
            .setSessionActivity(sessionActivity)
            .build()
        addSession(session)
        mediaSession = session

        val manager = getSystemService(AUDIO_SERVICE) as AudioManager
        audioManager = manager
        // Registration reports the current inventory; only notify for later topology changes.
        outputDeviceIds = manager.getDevices(AudioManager.GET_DEVICES_OUTPUTS).map { it.id }.toSet()
        manager.registerAudioDeviceCallback(audioDeviceCallback, handler)
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? =
        mediaSession

    private fun updateOutputDevices() {
        val manager = audioManager ?: return
        val deviceIds = manager.getDevices(AudioManager.GET_DEVICES_OUTPUTS).map { it.id }.toSet()
        if (deviceIds == outputDeviceIds) return
        outputDeviceIds = deviceIds
        handler.removeCallbacks(notifyOutputDeviceChanged)
        handler.postDelayed(notifyOutputDeviceChanged, 200)
    }

    override fun onDestroy() {
        audioManager?.unregisterAudioDeviceCallback(audioDeviceCallback)
        audioManager = null
        handler.removeCallbacks(notifyOutputDeviceChanged)
        mediaSession?.let { session ->
            removeSession(session)
            session.release()
        }
        mediaSession = null
        super.onDestroy()
    }
}
