package com.enn3developer.n_music

import android.Manifest.permission.POST_NOTIFICATIONS
import android.Manifest.permission.READ_MEDIA_AUDIO
import android.annotation.SuppressLint
import android.app.NativeActivity
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.graphics.BitmapFactory
import android.media.AudioManager
import android.media.MediaMetadata
import android.media.session.MediaSession
import android.media.session.PlaybackState
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.SystemClock
import android.os.Looper
import android.widget.Toast
import androidx.annotation.OptIn
import androidx.annotation.RequiresApi
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.media3.common.util.UnstableApi


@OptIn(UnstableApi::class)
class MainActivity : NativeActivity() {
    companion object {
        init {
            // Load the STL first to workaround issues on old Android versions:
            // "if your app targets a version of Android earlier than Android 4.3
            // (Android API level 18),
            // and you use libc++_shared.so, you must load the shared library before any other
            // library that depends on it."
            // See https://developer.android.com/ndk/guides/cpp-support#shared_runtimes
            //System.loadLibrary("c++_shared");

            // Load the native library.
            // The name "android-game" depends on your CMake configuration, must be
            // consistent here and inside AndroidManifest.xml
            System.loadLibrary("n_player")
        }

        const val NOTIFICATION_NAME_SERVICE = "NPlayer"
        const val NOTIFICATION_ID = 1
        const val CHANNEL_ID = "NMusic"
        const val ASK_DIRECTORY = 0
        const val REQUEST_PERMISSION_CODE = 1
        const val ACTIONS = PlaybackState.ACTION_PLAY or PlaybackState.ACTION_PAUSE or PlaybackState.ACTION_SKIP_TO_NEXT or PlaybackState.ACTION_SKIP_TO_PREVIOUS or PlaybackState.ACTION_SEEK_TO
    }

    @SuppressLint("RestrictedApi")
    // It's the playback in the notification
    public var playback: PlaybackState.Builder? = null

    // It's used to set metadata of the song and playback
    public var mediaSession: MediaSession? = null

    // We set here mediaSession token for style
    private var notification: Notification.Builder? = null

    // Called when app is open first time
    private external fun start(activity: MainActivity)

    private external fun gotDirectory(directory: String, requestId: Long)
    private external fun visibilityChanged(visible: Boolean)
    private var directoryRequest: Long? = null


    private val bluetoothBroadcastReceiver = object : BroadcastReceiver() {
        override fun onReceive(p0: Context?, p1: Intent?) {
        }
    }

    private fun askDirectoryWithPermission() {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
        }
        startActivityForResult(intent, ASK_DIRECTORY)
    }

    @Suppress("unused")
    private fun askDirectory(requestId: Long) {
        runOnUiThread {
            directoryRequest = requestId
            if (!checkPermissions()) {
                requestPermissions()
            } else {
                askDirectoryWithPermission()
            }
        }
    }

    @Suppress("unused")
    private fun set_clipboard_text(text: String){
        val clipboard: ClipboardManager = getSystemService(CLIPBOARD_SERVICE) as ClipboardManager
        val clip = ClipData.newPlainText(text, text)
        clipboard.setPrimaryClip(clip)
    }

    @Suppress("unused")
    private fun openLink(link: String) {
        val browserIntent = Intent(Intent.ACTION_VIEW, Uri.parse(link))
        startActivity(browserIntent)
    }

    @SuppressLint("RestrictedApi")
    @RequiresApi(Build.VERSION_CODES.TIRAMISU)
    @Suppress("unused")
    private fun createNotification() {
        if (!checkPermissions()) {
            requestPermissions()
        }
        val TAG = "PlaybackService"
        mediaSession = MediaSession(applicationContext, TAG)
        val handler = Handler(Looper.getMainLooper())
        handler.post {
            mediaSession?.setCallback(MediaCallback())
        }
        val bluetoothReceiver = IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY)
        applicationContext.registerReceiver(bluetoothBroadcastReceiver, bluetoothReceiver)
        playback = PlaybackState.Builder()
            .setActions(ACTIONS)
            .setActiveQueueItemId(ACTIONS)
        val channel = NotificationChannel(
            CHANNEL_ID,
            NOTIFICATION_NAME_SERVICE,
            NotificationManager.IMPORTANCE_LOW
        )
        playback?.setState(
            PlaybackState.STATE_PAUSED,
            0L, 0.0f
        )
        mediaSession?.setPlaybackState(playback?.build())
        NotificationManagerCompat.from(applicationContext).createNotificationChannel(channel)
        notification = Notification.Builder(applicationContext, CHANNEL_ID).apply {
            setSmallIcon(R.drawable.ic_launcher_monochrome)
            style = Notification.MediaStyle().setMediaSession(mediaSession?.sessionToken)
            setOngoing(true)
        }
    }

    private fun changePlaybackState(playing: Boolean, position: Double) {
        playback?.setState(
            if (playing) PlaybackState.STATE_PLAYING else PlaybackState.STATE_PAUSED,
            (position * 1000.0).toLong(),
            if (playing) 1.0f else 0.0f,
            SystemClock.elapsedRealtime()
        )
        mediaSession?.setPlaybackState(playback?.build())
    }

    @OptIn(UnstableApi::class)
    @SuppressLint("RestrictedApi")
    @Suppress("unused")
    private fun changeNotification(
        title: String,
        artists: String,
        coverPath: String,
        songLength: Double
    ) {
        var intent = applicationContext.packageManager.getLaunchIntentForPackage(packageName)

        if (intent == null) {
            intent = Intent(applicationContext, MainActivity::class.java)
        }

        intent.flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED

        val pendingIntent =
            PendingIntent.getActivity(
                applicationContext, 0, intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )

        val duration = songLength.toLong() * 1000
        val metadata = MediaMetadata.Builder()
            .apply {
                putString(MediaMetadata.METADATA_KEY_TITLE, title)
                putString(MediaMetadata.METADATA_KEY_ARTIST, artists)
                putLong(MediaMetadata.METADATA_KEY_DURATION, duration)
                if (coverPath.isNotEmpty()) {
                    val cover = BitmapFactory.decodeFile(coverPath)
                    putBitmap(MediaMetadata.METADATA_KEY_ALBUM_ART, cover)
                }
            }
            .build()
        mediaSession?.apply {
            setMetadata(metadata)
        }
        notification?.apply {
            setContentTitle(title)
            setContentText(artists)
            setContentIntent(pendingIntent)
            val cover = BitmapFactory.decodeFile(coverPath)
            if (cover != null) {
                setLargeIcon(cover)
            }
        }
        with(getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager) {
            if (ActivityCompat.checkSelfPermission(
                    applicationContext,
                    POST_NOTIFICATIONS
                ) != PackageManager.PERMISSION_GRANTED
            ) {
                return@with
            }
            notify(NOTIFICATION_ID, notification?.build())
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        start(this)
    }

    override fun onDestroy() {
        val notificationManager =
            getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        notificationManager.cancel(NOTIFICATION_ID)
        super.onDestroy()
    }

    override fun onResume() {
        super.onResume()
        visibilityChanged(true)
    }

    override fun onPause() {
        visibilityChanged(false)
        super.onPause()
    }

    private fun finishDirectory(path: String) {
        directoryRequest?.let { gotDirectory(path, it) }
        directoryRequest = null
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != ASK_DIRECTORY) return
        val uri = data?.data
        if (resultCode != RESULT_OK || uri?.path == null) {
            finishDirectory("")
            return
        }
        applicationContext.contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        finishDirectory(uri.path!!.replace("/tree/primary:", "/storage/emulated/0/"))
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        when (requestCode) {
            REQUEST_PERMISSION_CODE -> if (grantResults.isNotEmpty()) {
                if (grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                    Toast.makeText(applicationContext, "Permission granted", Toast.LENGTH_SHORT)
                        .show()
                    if (directoryRequest != null) askDirectoryWithPermission()
                } else {
                    finishDirectory("")
                    Toast.makeText(applicationContext, "Permission denied", Toast.LENGTH_SHORT)
                        .show()
                }
            } else { finishDirectory("") }
        }
    }

    @SuppressLint("InlinedApi")
    fun checkPermissions(): Boolean {
        val readMediaAudio = ContextCompat.checkSelfPermission(applicationContext, READ_MEDIA_AUDIO)
        val grantNotification =
            ContextCompat.checkSelfPermission(applicationContext, POST_NOTIFICATIONS)
        return (readMediaAudio == PackageManager.PERMISSION_GRANTED) && (grantNotification == PackageManager.PERMISSION_GRANTED)
    }

    @SuppressLint("InlinedApi")
    private fun requestPermissions() {
        ActivityCompat.requestPermissions(
            this,
            arrayOf(READ_MEDIA_AUDIO, POST_NOTIFICATIONS),
            REQUEST_PERMISSION_CODE
        )
    }
}