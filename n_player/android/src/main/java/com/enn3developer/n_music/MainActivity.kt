package com.enn3developer.n_music

import android.Manifest.permission.READ_MEDIA_AUDIO
import android.annotation.SuppressLint
import android.app.NativeActivity
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Bundle
import android.os.Looper
import android.widget.Toast
import androidx.annotation.OptIn
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaStyleNotificationHelper


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

        const val ASK_DIRECTORY = 0
        const val ASK_FILE = 1
        const val REQUEST_PERMISSION_CODE = 1
    }

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

    @OptIn(UnstableApi::class)
    @Suppress("unused")
    private fun createNotification() {
        val mediaSession =
            MediaSession.Builder(
                applicationContext,
                NPlayer(Looper.getMainLooper())
            )
                .build()
        var notification = NotificationCompat.Builder(applicationContext, "n_music")
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setSmallIcon(R.mipmap.ic_launcher_round).setStyle(
                MediaStyleNotificationHelper.MediaStyle(mediaSession)
                    .setShowActionsInCompactView(1 /* #1: pause button */)
            ).build()

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
            arrayOf(READ_MEDIA_AUDIO),
            REQUEST_PERMISSION_CODE
        )
    }
}
