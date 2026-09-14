package com.enn3developer.n_music

import android.Manifest.permission.POST_NOTIFICATIONS
import android.Manifest.permission.READ_MEDIA_AUDIO
import android.app.NativeActivity
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.widget.Toast
import androidx.annotation.RequiresApi
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import java.io.File

class MainActivity : NativeActivity() {
    companion object {
        init {
            // Load the native library.
            System.loadLibrary("n_music_android")
        }

        const val NOTIFICATION_NAME_SERVICE = "NPlayer"
        const val ASK_DIRECTORY = 0
        const val REQUEST_PERMISSION_CODE = 1

        @JvmStatic external fun mediaPause()
        @JvmStatic external fun mediaPlay()
        @JvmStatic external fun mediaPlayNext()
        @JvmStatic external fun mediaPlayPrevious()
        @JvmStatic external fun mediaSeekTo(index: Int, position: Double)
        @JvmStatic external fun mediaSeek(seek: Double)
        @JvmStatic external fun mediaRepeatMode(mode: Int)
    }

    // Called when app is open first time
    private external fun start(activity: MainActivity)

    private external fun gotDirectory(directory: String, requestId: Long)
    private external fun visibilityChanged(visible: Boolean)
    private var directoryRequest: Long? = null

    private fun askDirectoryWithPermission() {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
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
    private fun set_clipboard_text(text: String) {
        val clipboard: ClipboardManager = getSystemService(CLIPBOARD_SERVICE) as ClipboardManager
        val clip = ClipData.newPlainText(text, text)
        clipboard.setPrimaryClip(clip)
    }

    @Suppress("unused")
    private fun openLink(link: String) {
        val browserIntent = Intent(Intent.ACTION_VIEW, Uri.parse(link))
        startActivity(browserIntent)
    }

    // It's the playback shown in the notification
    @RequiresApi(Build.VERSION_CODES.TIRAMISU)
    @Suppress("unused")
    private fun createNotification() {
        runOnUiThread {
            if (!checkPermissions()) {
                requestPermissions()
            }
        }
        startService(Intent(this, PlaybackService::class.java))
    }

    @Suppress("unused")
    private fun changePlaybackState(playing: Boolean, position: Double) {
        PlaybackController.updatePlayback(playing, (position * 1000.0).toLong())
    }

    @Suppress("unused")
    private fun changeNotification(
        title: String,
        artists: String,
        coverPath: String,
        songLength: Double,
    ) {
        val artwork = if (coverPath.isNotEmpty()) {
            runCatching { File(coverPath).readBytes() }.getOrNull()
        } else {
            null
        }
        PlaybackController.updateMetadata(title, artists, artwork, (songLength * 1000.0).toLong())
    }

    @Suppress("unused")
    private fun changeRepeatMode(mode: Int) {
        PlaybackController.setRepeatMode(mode)
    }

    @Suppress("unused")
    private fun changeTrack(index: Int) {
        PlaybackController.updateTrack(index)
    }

    @Suppress("unused")
    private fun changeQueue(names: String) {
        PlaybackController.setQueue(if (names.isEmpty()) emptyList() else names.split('\u001f'))
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        start(this)
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
        applicationContext.contentResolver.takePersistableUriPermission(
            uri,
            Intent.FLAG_GRANT_READ_URI_PERMISSION,
        )
        finishDirectory(uri.path!!.replace("/tree/primary:", "/storage/emulated/0/"))
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
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
            } else {
                finishDirectory("")
            }
        }
    }

    @Suppress("unused")
    fun checkPermissions(): Boolean {
        val readMediaAudio =
            ContextCompat.checkSelfPermission(applicationContext, READ_MEDIA_AUDIO)
        val grantNotification =
            ContextCompat.checkSelfPermission(applicationContext, POST_NOTIFICATIONS)
        return (readMediaAudio == PackageManager.PERMISSION_GRANTED) &&
            (grantNotification == PackageManager.PERMISSION_GRANTED)
    }

    private fun requestPermissions() {
        ActivityCompat.requestPermissions(
            this,
            arrayOf(READ_MEDIA_AUDIO, POST_NOTIFICATIONS),
            REQUEST_PERMISSION_CODE,
        )
    }
}
