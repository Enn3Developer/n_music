package com.enn3developer.n_music

import android.Manifest.permission.POST_NOTIFICATIONS
import android.app.NativeActivity
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.res.Configuration
import android.graphics.Color
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.util.Log
import android.widget.Toast
import androidx.annotation.OptIn
import androidx.annotation.RequiresApi
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.core.view.WindowCompat
import androidx.media3.common.util.UnstableApi
import java.io.File

@OptIn(UnstableApi::class)
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
        @JvmStatic external fun outputDeviceChanged()
    }

    // Called when app is open first time
    private external fun start(activity: MainActivity)

    private external fun gotDirectory(directory: String, requestId: Long)
    private external fun visibilityChanged(visible: Boolean)
    private var directoryRequest: Long? = null
    private var theme: Int = 0 // App theme: 0 = System, 1 = Light, 2 = Dark

    // The picked folder is read through the Storage Access Framework, which needs no runtime
    // permission.
    @Suppress("unused")
    private fun askDirectory(requestId: Long) {
        runOnUiThread {
            directoryRequest = requestId
            startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT_TREE), ASK_DIRECTORY)
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

    @Suppress("unused")
    private fun set_theme(value: Int) {
        theme = value
        updateStatusBarAppearance()
    }

    private fun updateStatusBarAppearance() {
        runOnUiThread {
            val window = this.window
            window.addFlags(android.view.WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS)

            val isLight = when (theme) {
                1 -> true
                2 -> false
                else -> {
                    val currentNightMode =
                        resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK
                    currentNightMode == Configuration.UI_MODE_NIGHT_NO
                }
            }

            window.statusBarColor = if (isLight) Color.WHITE else Color.BLACK

            WindowCompat.getInsetsController(window, window.decorView).isAppearanceLightStatusBars = isLight
        }
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        updateStatusBarAppearance()
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
            runCatching { File(coverPath).readBytes() }
                .onFailure { Log.w("n_music", "Could not read notification cover $coverPath", it) }
                .getOrNull()
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
        if (resultCode != RESULT_OK || uri == null) {
            finishDirectory("")
            return
        }
        val resolver = applicationContext.contentResolver
        resolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        // Only the current library folder needs access; persisted grants are capped per app.
        for (permission in resolver.persistedUriPermissions) {
            if (permission.uri != uri) {
                resolver.releasePersistableUriPermission(
                    permission.uri,
                    Intent.FLAG_GRANT_READ_URI_PERMISSION,
                )
            }
        }
        finishDirectory(uri.toString())
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        when (requestCode) {
            REQUEST_PERMISSION_CODE -> if (grantResults.isNotEmpty()) {
                val message = if (grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                    "Permission granted"
                } else {
                    "Permission denied"
                }
                Toast.makeText(applicationContext, message, Toast.LENGTH_SHORT).show()
            }
        }
    }

    @Suppress("unused")
    fun checkPermissions(): Boolean {
        val grantNotification =
            ContextCompat.checkSelfPermission(applicationContext, POST_NOTIFICATIONS)
        return grantNotification == PackageManager.PERMISSION_GRANTED
    }

    private fun requestPermissions() {
        ActivityCompat.requestPermissions(
            this,
            arrayOf(POST_NOTIFICATIONS),
            REQUEST_PERMISSION_CODE,
        )
    }
}
