package com.enn3developer.n_music

import android.Manifest
import android.content.ActivityNotFoundException
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.media.MediaRouter2
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.util.Log
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.net.toUri
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import com.enn3developer.n_music.ui.AppHost
import com.enn3developer.n_music.ui.NMusicApp
import com.google.common.util.concurrent.ListenableFuture

/**
 * The app's one activity. An AppCompat one, so the per-app language Android keeps from
 * Android 13 also applies on Android 11 and 12.
 */
class MainActivity : AppCompatActivity(), AppHost {
    private var controller: ListenableFuture<MediaController>? = null

    /** Gets the folder the open picker picks; a tap meanwhile does not open a second one. */
    private var picked: ((Uri) -> Unit)? = null

    private val folderPicker =
        registerForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
            val onPicked = picked
            picked = null
            if (uri != null && onPicked != null) {
                contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
                onPicked(uri)
            }
        }

    private val askNotifications =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) {}

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent { NMusicApp(this) }
        // Asked at launch, as the Slint app did, not again when the activity is recreated.
        if (savedInstanceState == null &&
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            askNotifications.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
    }

    override fun onStart() {
        super.onStart()
        // A connected controller keeps PlaybackService running while the app shows, so playback
        // started here gets its notification and foreground service right away.
        val token = SessionToken(this, ComponentName(this, PlaybackService::class.java))
        controller = MediaController.Builder(this, token).buildAsync()
    }

    override fun onStop() {
        controller?.let(MediaController::releaseFuture)
        controller = null
        super.onStop()
    }

    override fun pickFolder(onPicked: (Uri) -> Unit) {
        if (picked != null) return
        try {
            folderPicker.launch(null)
            picked = onPicked
        } catch (error: ActivityNotFoundException) {
            Log.w("n_music", "Nothing can pick a folder", error)
        }
    }

    override fun releaseFolder(folder: Uri) {
        try {
            contentResolver.releasePersistableUriPermission(folder, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        } catch (error: SecurityException) {
            // Already let go of, or never kept.
            Log.w("n_music", "Could not release $folder", error)
        }
    }

    override fun openLink(url: String) {
        try {
            startActivity(Intent(Intent.ACTION_VIEW, url.toUri()))
        } catch (error: ActivityNotFoundException) {
            Log.w("n_music", "Nothing can open $url", error)
        }
    }

    /**
     * Android's output switcher for this app's playback: MediaRouter2's from Android 14, System
     * UI's media output dialog from Android 12, the settings panel on Android 11, and the
     * Bluetooth settings where none of those is there.
     */
    override fun openOutputSwitcher() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE &&
            MediaRouter2.getInstance(this).showSystemOutputSwitcher()
        ) {
            return
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            val dialog = Intent("com.android.systemui.action.LAUNCH_MEDIA_OUTPUT_DIALOG")
                .setPackage("com.android.systemui")
                .putExtra("package_name", packageName)
            if (packageManager.queryBroadcastReceivers(dialog, 0).isNotEmpty()) {
                sendBroadcast(dialog)
                return
            }
        }
        val panel = Intent("com.android.settings.panel.action.MEDIA_OUTPUT")
            .putExtra("com.android.settings.panel.extra.PACKAGE_NAME", packageName)
        for (intent in listOf(panel, Intent(Settings.ACTION_BLUETOOTH_SETTINGS))) {
            try {
                startActivity(intent)
                return
            } catch (error: ActivityNotFoundException) {
                Log.w("n_music", "No output switcher at ${intent.action}", error)
            }
        }
    }
}
