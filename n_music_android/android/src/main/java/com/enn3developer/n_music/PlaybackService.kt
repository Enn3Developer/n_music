package com.enn3developer.n_music

import android.app.PendingIntent
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.annotation.OptIn
import androidx.core.app.NotificationChannelCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.app.ServiceCompat
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.CommandButton
import androidx.media3.session.DefaultMediaNotificationProvider
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSession.ConnectionResult
import androidx.media3.session.MediaSession.MediaItemsWithStartPosition
import androidx.media3.session.MediaLibraryService
import androidx.media3.session.MediaLibraryService.MediaLibrarySession
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionResult
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.widget.Widgets
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.google.common.util.concurrent.SettableFuture
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import java.io.File

/**
 * Keeps playback going in the background: Media3 runs this service in the foreground, with the
 * media notification, while [NPlayer] plays, and for ten minutes after a pause so playback can
 * resume from the notification or a headset without hitting background start restrictions.
 *
 * The core itself lives in the process, started by [NMusicApplication]; this service only
 * mirrors it for the system. It is a library service, with no library to browse, so Android's
 * System UI can show its card for resuming playback after a reboot.
 */
@OptIn(UnstableApi::class)
class PlaybackService : MediaLibraryService() {
    private var mediaSession: MediaLibrarySession? = null
    private val scope = MainScope()
    private var player: NPlayer? = null
    private val handler = Handler(Looper.getMainLooper())
    private val toggleRepeatCommand = SessionCommand(
        "com.enn3developer.n_music.TOGGLE_REPEAT",
        Bundle.EMPTY,
    )
    private val toggleShuffleCommand = SessionCommand(
        "com.enn3developer.n_music.TOGGLE_SHUFFLE",
        Bundle.EMPTY,
    )
    private val updateModeButtons = Runnable {
        val session = mediaSession ?: return@Runnable
        session.setMediaButtonPreferences(modeButtons(session.player))
    }
    private val modeListener = object : Player.Listener {
        // Let Media3 dispatch the new player state before publishing the matching buttons.
        override fun onRepeatModeChanged(repeatMode: Int) {
            handler.removeCallbacks(updateModeButtons)
            handler.post(updateModeButtons)
        }

        override fun onShuffleModeEnabledChanged(shuffleModeEnabled: Boolean) {
            handler.removeCallbacks(updateModeButtons)
            handler.post(updateModeButtons)
        }
    }
    private var audioManager: AudioManager? = null
    private var outputDeviceIds = emptySet<Int>()
    private val notifyOutputDeviceChanged = Runnable {
        CoreRepository.send(Command.OutputDeviceChanged)
    }

    /**
     * A widget started the service in the foreground, which Android ends the app for unless the
     * service gets there in time. Media3 does while playback runs; paused, the notification goes
     * to the foreground once; with nothing to show, a placeholder comes and goes.
     */
    private val foregroundCheck = Runnable {
        val session = mediaSession ?: return@Runnable
        val player = session.player
        when {
            player.isPlaying -> {}
            player.playbackState != Player.STATE_IDLE && !player.currentTimeline.isEmpty -> {
                onUpdateNotification(session, true)
                handler.postDelayed(settleNotification, FOREGROUND_SETTLE_MS)
            }
            else -> showPlaceholder()
        }
    }

    /** Back to what playback asks for, once Android has seen the notification in the foreground. */
    private val settleNotification = Runnable { triggerNotificationUpdate() }

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
        // NPlayer is idle before anything plays and after the notification was dismissed:
        // nothing to show then.
        setShowNotificationForIdlePlayer(SHOW_NOTIFICATION_FOR_IDLE_PLAYER_NEVER)
        setMediaNotificationProvider(
            DefaultMediaNotificationProvider.Builder(this).build().apply {
                setSmallIcon(R.drawable.ic_notification)
            }
        )
        val sessionActivity = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val player = NPlayer(this)
        this.player = player
        val callback = object : MediaLibrarySession.Callback {
            override fun onConnect(
                session: MediaSession,
                controller: MediaSession.ControllerInfo,
            ): ConnectionResult = ConnectionResult.AcceptedResultBuilder(session)
                .setAvailableSessionCommands(
                    ConnectionResult.DEFAULT_SESSION_AND_LIBRARY_COMMANDS.buildUpon()
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

            // To play, Media3 only asks when a controller plays an empty player that takes new
            // items, which NPlayer never is: a play press reaches it and the core resumes its
            // own saved session. The answer describes that session for anyone who asks anyway.
            // At boot System UI asks without playing, for its resumption card.
            override fun onPlaybackResumption(
                mediaSession: MediaSession,
                controller: MediaSession.ControllerInfo,
                isForPlayback: Boolean,
            ): ListenableFuture<MediaItemsWithStartPosition> {
                if (!isForPlayback) return resumptionCard(player)
                val (items, index, positionMs) = player.resumptionItems()
                return Futures.immediateFuture(
                    MediaItemsWithStartPosition(items, index, positionMs)
                )
            }
        }
        val session = MediaLibrarySession.Builder(this, player, callback)
            .setSessionActivity(sessionActivity)
            // Repeat and shuffle capabilities alone do not create buttons in Android's media
            // controls.
            .setMediaButtonPreferences(modeButtons(player))
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

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaLibrarySession? =
        mediaSession

    /**
     * What System UI's resumption card shows after a reboot: the track the saved session stopped
     * on, once the core has read it back, with its cover's bytes, as System UI can't open the
     * app's files. Nothing when there is no session to come back to: while Resume is off, or
     * before anything played.
     */
    private fun resumptionCard(player: NPlayer): ListenableFuture<MediaItemsWithStartPosition> {
        val nothing = MediaItemsWithStartPosition(emptyList(), C.INDEX_UNSET, C.TIME_UNSET)
        if (!CoreRepository.options.value.resume) return Futures.immediateFuture(nothing)
        val card = SettableFuture.create<MediaItemsWithStartPosition>()
        scope.launch {
            val item = withTimeoutOrNull(RESTORE_WAIT_MS) {
                var item = restoredItem(player)
                while (item == null) {
                    delay(RESTORE_POLL_MS)
                    item = restoredItem(player)
                }
                item
            }
            if (item == null) {
                card.set(nothing)
                return@launch
            }
            val cover = item.mediaMetadata.artworkUri?.path?.let { path ->
                withContext(Dispatchers.IO) { runCatching { File(path).readBytes() }.getOrNull() }
            }
            // Media3 refuses a library item that doesn't say whether it can be browsed and played.
            val metadata = item.mediaMetadata.buildUpon()
                .setArtworkData(cover, MediaMetadata.PICTURE_TYPE_FRONT_COVER)
                .setIsBrowsable(false)
                .setIsPlayable(true)
                .build()
            val shown = item.buildUpon().setMediaMetadata(metadata).build()
            card.set(MediaItemsWithStartPosition(listOf(shown), 0, player.resumptionItems().third))
        }
        return card
    }

    /** The core's current track as [player] shows it, once the player caught up with the core. */
    private fun restoredItem(player: NPlayer): MediaItem? {
        val current = CoreRepository.current.value ?: return null
        val (items, index, _) = player.resumptionItems()
        return items.getOrNull(index)?.takeIf { it.mediaId == current.item.toString() }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.getBooleanExtra(Widgets.EXTRA_WIDGET, false) == true) {
            if (intent.getBooleanExtra(Widgets.EXTRA_SHUFFLE_ALL, false)) Widgets.shuffleEverything()
            handler.removeCallbacks(foregroundCheck)
            handler.postDelayed(foregroundCheck, FOREGROUND_CHECK_MS)
        }
        // The key itself goes to the session, as a headset's does.
        return super.onStartCommand(intent, flags, startId)
    }

    private fun showPlaceholder() {
        val channel = DefaultMediaNotificationProvider.DEFAULT_CHANNEL_ID
        val notifications = NotificationManagerCompat.from(this)
        notifications.createNotificationChannel(
            NotificationChannelCompat.Builder(channel, NotificationManagerCompat.IMPORTANCE_LOW)
                .setName(getString(androidx.media3.session.R.string.default_notification_channel_name))
                .build()
        )
        val notification = NotificationCompat.Builder(this, channel)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(getString(R.string.app_name))
            .setSilent(true)
            .build()
        ServiceCompat.startForeground(
            this,
            PLACEHOLDER_NOTIFICATION_ID,
            notification,
            ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK,
        )
        ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_REMOVE)
    }

    // Android's players put the first on the left and the second on the right.
    private fun modeButtons(player: Player): List<CommandButton> =
        listOf(shuffleButton(player.shuffleModeEnabled), repeatButton(player.repeatMode))

    private fun shuffleButton(enabled: Boolean): CommandButton =
        CommandButton.Builder(
            if (enabled) CommandButton.ICON_SHUFFLE_ON else CommandButton.ICON_SHUFFLE_OFF
        )
            .setSessionCommand(toggleShuffleCommand)
            .setDisplayName(getString(if (enabled) R.string.shuffle_on else R.string.shuffle_off))
            .setSlots(CommandButton.SLOT_OVERFLOW)
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
            .setSlots(CommandButton.SLOT_OVERFLOW)
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
        handler.removeCallbacks(updateModeButtons)
        handler.removeCallbacks(foregroundCheck)
        handler.removeCallbacks(settleNotification)
        mediaSession?.let { session ->
            session.player.removeListener(modeListener)
            removeSession(session)
            session.release()
        }
        player?.release()
        player = null
        mediaSession = null
        scope.cancel()
        super.onDestroy()
    }

    private companion object {
        /** How long a widget's start waits for playback before checking the foreground. */
        const val FOREGROUND_CHECK_MS = 2_000L
        const val FOREGROUND_SETTLE_MS = 1_000L

        /** Apart from Media3's own notification. */
        const val PLACEHOLDER_NOTIFICATION_ID = 2_001

        /** How long the resumption card waits for the core to read the saved session back. */
        const val RESTORE_WAIT_MS = 5_000L
        const val RESTORE_POLL_MS = 100L
    }
}
