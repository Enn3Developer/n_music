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
    private val toggleShuffleCommand = SessionCommand(
        "com.enn3developer.n_music.TOGGLE_SHUFFLE",
        Bundle.EMPTY,
    )
    private val updateModeLayout = Runnable {
        val session = mediaSession ?: return@Runnable
        session.setCustomLayout(modeButtons(session.player))
    }
    private val modeListener = object : Player.Listener {
        // Let Media3 dispatch the new player state before publishing the matching buttons.
        override fun onRepeatModeChanged(repeatMode: Int) {
            handler.removeCallbacks(updateModeLayout)
            handler.post(updateModeLayout)
        }

        override fun onShuffleModeEnabledChanged(shuffleModeEnabled: Boolean) {
            handler.removeCallbacks(updateModeLayout)
            handler.post(updateModeLayout)
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
            // Repeat and shuffle capabilities alone do not create buttons in Android's media
            // controls.
            .setCustomLayout(modeButtons(player))
            .setCallback(object : MediaSession.Callback {
                override fun onConnect(
                    session: MediaSession,
                    controller: MediaSession.ControllerInfo,
                ): ConnectionResult = ConnectionResult.AcceptedResultBuilder(session)
                    .setAvailableSessionCommands(
                        ConnectionResult.DEFAULT_SESSION_COMMANDS.buildUpon()
                            .add(toggleRepeatCommand)
                            .add(toggleShuffleCommand)
                            .build()
                    )
                    .build()

                override fun onCustomCommand(
                    session: MediaSession,
                    controller: MediaSession.ControllerInfo,
                    customCommand: SessionCommand,
                    args: Bundle,
                ): ListenableFuture<SessionResult> {
                    val sessionPlayer = session.player
                    when (customCommand) {
                        // Off, all, one, like the native ToggleRepeat.
                        toggleRepeatCommand -> sessionPlayer.repeatMode =
                            when (sessionPlayer.repeatMode) {
                                Player.REPEAT_MODE_OFF -> Player.REPEAT_MODE_ALL
                                Player.REPEAT_MODE_ALL -> Player.REPEAT_MODE_ONE
                                else -> Player.REPEAT_MODE_OFF
                            }

                        toggleShuffleCommand ->
                            sessionPlayer.shuffleModeEnabled = !sessionPlayer.shuffleModeEnabled

                        else -> return super.onCustomCommand(session, controller, customCommand, args)
                    }
                    return Futures.immediateFuture(SessionResult(SessionResult.RESULT_SUCCESS))
                }
            })
            .build()
        mediaSession = session
        player.addListener(modeListener)
        addSession(session)

        val manager = getSystemService(AUDIO_SERVICE) as AudioManager
        audioManager = manager
        // Registration reports the current inventory; only notify for later topology changes.
        outputDeviceIds = manager.getDevices(AudioManager.GET_DEVICES_OUTPUTS).map { it.id }.toSet()
        manager.registerAudioDeviceCallback(audioDeviceCallback, handler)
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? =
        mediaSession

    private fun modeButtons(player: Player): List<CommandButton> =
        listOf(repeatButton(player.repeatMode), shuffleButton(player.shuffleModeEnabled))

    private fun shuffleButton(enabled: Boolean): CommandButton =
        CommandButton.Builder(
            if (enabled) CommandButton.ICON_SHUFFLE_ON else CommandButton.ICON_SHUFFLE_OFF
        )
            .setSessionCommand(toggleShuffleCommand)
            .setDisplayName(getString(if (enabled) R.string.shuffle_on else R.string.shuffle_off))
            .build()

    private fun repeatButton(repeatMode: Int): CommandButton {
        val (icon, name) = when (repeatMode) {
            Player.REPEAT_MODE_ONE -> CommandButton.ICON_REPEAT_ONE to R.string.repeat_one
            Player.REPEAT_MODE_ALL -> CommandButton.ICON_REPEAT_ALL to R.string.repeat_all
            else -> CommandButton.ICON_REPEAT_OFF to R.string.repeat_off
        }
        return CommandButton.Builder(icon)
            .setSessionCommand(toggleRepeatCommand)
            .setDisplayName(getString(name))
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
        handler.removeCallbacks(updateModeLayout)
        mediaSession?.let { session ->
            session.player.removeListener(modeListener)
            removeSession(session)
            session.release()
        }
        mediaSession = null
        super.onDestroy()
    }
}
