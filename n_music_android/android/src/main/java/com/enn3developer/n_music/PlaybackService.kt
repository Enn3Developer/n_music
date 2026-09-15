package com.enn3developer.n_music

import android.app.PendingIntent
import android.content.Intent
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.CommandButton
import androidx.media3.session.DefaultMediaNotificationProvider
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSession.ConnectionResult
import androidx.media3.session.MediaSessionService
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionResult
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture

@UnstableApi
class PlaybackService : MediaSessionService() {
    private var mediaSession: MediaSession? = null
    private val handler = Handler(Looper.getMainLooper())
    private val toggleRepeatCommand = SessionCommand(
        "com.enn3developer.n_music.TOGGLE_REPEAT",
        Bundle.EMPTY,
    )
    private val updateRepeatLayout = Runnable {
        val session = mediaSession ?: return@Runnable
        session.setCustomLayout(listOf(repeatButton(session.player.repeatMode)))
    }
    private val repeatListener = object : Player.Listener {
        override fun onRepeatModeChanged(repeatMode: Int) {
            // Let Media3 dispatch the new player state before publishing the matching button.
            handler.removeCallbacks(updateRepeatLayout)
            handler.post(updateRepeatLayout)
        }
    }
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
        setMediaNotificationProvider(
            DefaultMediaNotificationProvider.Builder(this).build().apply {
                setSmallIcon(R.drawable.ic_launcher_monochrome)
            }
        )
        val sessionActivity = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val player: Player = PlaybackController.player()
        val session = MediaSession.Builder(this, player)
            .setSessionActivity(sessionActivity)
            // Repeat capability alone does not create a button in Android's media controls.
            .setCustomLayout(listOf(repeatButton(player.repeatMode)))
            .setCallback(object : MediaSession.Callback {
                override fun onConnect(
                    session: MediaSession,
                    controller: MediaSession.ControllerInfo,
                ): ConnectionResult = ConnectionResult.AcceptedResultBuilder(session)
                    .setAvailableSessionCommands(
                        ConnectionResult.DEFAULT_SESSION_COMMANDS.buildUpon()
                            .add(toggleRepeatCommand)
                            .build()
                    )
                    .build()

                override fun onCustomCommand(
                    session: MediaSession,
                    controller: MediaSession.ControllerInfo,
                    customCommand: SessionCommand,
                    args: Bundle,
                ): ListenableFuture<SessionResult> {
                    if (customCommand != toggleRepeatCommand) {
                        return super.onCustomCommand(session, controller, customCommand, args)
                    }
                    val sessionPlayer = session.player
                    sessionPlayer.repeatMode = if (sessionPlayer.repeatMode == Player.REPEAT_MODE_ONE) {
                        Player.REPEAT_MODE_ALL
                    } else {
                        Player.REPEAT_MODE_ONE
                    }
                    return Futures.immediateFuture(SessionResult(SessionResult.RESULT_SUCCESS))
                }
            })
            .build()
        mediaSession = session
        player.addListener(repeatListener)
        addSession(session)

        val manager = getSystemService(AUDIO_SERVICE) as AudioManager
        audioManager = manager
        // Registration reports the current inventory; only notify for later topology changes.
        outputDeviceIds = manager.getDevices(AudioManager.GET_DEVICES_OUTPUTS).map { it.id }.toSet()
        manager.registerAudioDeviceCallback(audioDeviceCallback, handler)
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? =
        mediaSession

    private fun repeatButton(repeatMode: Int): CommandButton {
        val repeatOne = repeatMode == Player.REPEAT_MODE_ONE
        return CommandButton.Builder(
            if (repeatOne) CommandButton.ICON_REPEAT_ONE else CommandButton.ICON_REPEAT_ALL
        )
            .setSessionCommand(toggleRepeatCommand)
            .setDisplayName(getString(if (repeatOne) R.string.repeat_one else R.string.repeat_all))
            .build()
    }

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
        handler.removeCallbacks(updateRepeatLayout)
        mediaSession?.let { session ->
            session.player.removeListener(repeatListener)
            removeSession(session)
            session.release()
        }
        mediaSession = null
        super.onDestroy()
    }
}
