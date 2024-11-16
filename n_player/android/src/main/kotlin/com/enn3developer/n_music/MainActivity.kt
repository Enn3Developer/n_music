package com.enn3developer.n_music

import android.Manifest.permission.POST_NOTIFICATIONS
import android.Manifest.permission.READ_MEDIA_AUDIO
import android.annotation.SuppressLint
import android.app.NativeActivity
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.BitmapFactory
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Looper
import android.widget.Toast
import androidx.annotation.OptIn
import androidx.annotation.RequiresApi
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaStyleNotificationHelper

@UnstableApi
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

        const val NOTIFICATION_ID = 1
        const val CHANNEL_ID = "NMusic"
        const val ASK_DIRECTORY = 0
        const val ASK_FILE = 1
        const val REQUEST_PERMISSION_CODE = 1
    }
    lateinit var notification: NotificationCompat.Builder

    private external fun start(activity: MainActivity)
    private external fun gotDirectory(directory: String)
    private external fun gotFile(file: String)

    private fun askDirectoryWithPermission() {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
        }
        startActivityForResult(intent, ASK_DIRECTORY)
    }

    @Suppress("unused")
    private fun askDirectory() {
        println("asking directory")
        //Check if permission has been granted
        if (!checkPermissions()) {
            requestPermissions()
        } else {
            askDirectoryWithPermission()
        }
    }

    @Suppress("unused")
    private fun askFile() {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
        }
        startActivityForResult(intent, ASK_FILE)
    }

    @Suppress("unused")
    private fun openLink(link: String) {
        val browserIntent = Intent(Intent.ACTION_VIEW, Uri.parse(link))
        startActivity(browserIntent)
    }

    @RequiresApi(Build.VERSION_CODES.TIRAMISU)
    @OptIn(UnstableApi::class)
    @Suppress("unused")
    private fun createNotification() {
        if (
            checkSelfPermission(POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(POST_NOTIFICATIONS), REQUEST_PERMISSION_CODE)
        }

        val sessionActivityPendingIntent =
            applicationContext.packageManager?.getLaunchIntentForPackage(
                applicationContext.packageName)?.let {
                    sessionIntent -> PendingIntent.getActivity(
                applicationContext,
                0,
                sessionIntent,
                PendingIntent.FLAG_IMMUTABLE
            )
            }
        val player = NPlayer(Looper.getMainLooper())
        val mediaSession = MediaSession.Builder(applicationContext, player)
            .setSessionActivity(sessionActivityPendingIntent!!)
            .build()
        val channel = NotificationChannel(
            CHANNEL_ID,
            NOTIFICATION_SERVICE,
            NotificationManager.IMPORTANCE_DEFAULT
        )
        notification = NotificationCompat.Builder(applicationContext, CHANNEL_ID)
            .setStyle(
                MediaStyleNotificationHelper.MediaStyle(mediaSession)
                    .setShowActionsInCompactView(1 /* #1: pause button */)
            )
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .setSmallIcon(R.mipmap.ic_launcher_round)
        NotificationManagerCompat.from(applicationContext).createNotificationChannel(channel)
    }

    @OptIn(UnstableApi::class)
    @Suppress("unused")
    private fun changeNotification(title: String, artist: String, coverPath: String) {
        val icon = BitmapFactory.decodeFile(coverPath)
        notification
            .setContentTitle(title)
            .setContentText(artist)
            .setLargeIcon(icon)
        with(NotificationManagerCompat.from(this)) {
            if (ActivityCompat.checkSelfPermission(
                    applicationContext,
                    POST_NOTIFICATIONS
                ) != PackageManager.PERMISSION_GRANTED
            ) {
                return@with
            }

            notify(NOTIFICATION_ID, notification.build())
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        start(this)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (resultCode == RESULT_OK) {
            println("activity result ok")
            if (requestCode == ASK_DIRECTORY) {
                println("activity ask directory")
                data?.data?.also { uri ->
                    println("got data")
                    if (uri.path != null) {
                        println("path is not null")
                        val contentResolver = applicationContext.contentResolver
                        val takeFlags: Int = Intent.FLAG_GRANT_READ_URI_PERMISSION
                        contentResolver.takePersistableUriPermission(uri, takeFlags)
                        val path = uri.path!!.replace("/tree/primary:", "/storage/emulated/0/")
                        Toast.makeText(applicationContext, "Loading music...", Toast.LENGTH_LONG)
                            .show()
                        gotDirectory(path)
                    }
                }
            } else if (requestCode == ASK_FILE) {
                data?.data?.also { uri ->
                    val path = uri.path!!.replace("/tree/primary:", "/storage/emulated/0/")
                    gotFile(path)
                }
            }
        }
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
                    askDirectoryWithPermission()
                } else {
                    Toast.makeText(applicationContext, "Permission denied", Toast.LENGTH_SHORT)
                        .show()
                }
            }
        }
    }

    @SuppressLint("InlinedApi")
    fun checkPermissions(): Boolean {
        val readMediaAudio = ContextCompat.checkSelfPermission(applicationContext, READ_MEDIA_AUDIO)
        return readMediaAudio == PackageManager.PERMISSION_GRANTED
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
