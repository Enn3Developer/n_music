package com.enn3developer.n_music

import android.Manifest
import android.content.ActivityNotFoundException
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.util.Log
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.ui.AppScreen
import com.enn3developer.n_music.ui.LocalStrings
import com.enn3developer.n_music.ui.Localizations
import com.enn3developer.n_music.ui.NMusicTheme
import com.enn3developer.n_music.ui.SettingsScreen
import com.google.common.util.concurrent.ListenableFuture

class MainActivity : ComponentActivity() {
    private var controller: ListenableFuture<MediaController>? = null

    private val pickFolder =
        registerForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
            if (uri != null) useFolder(uri)
        }

    private val askNotifications =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            val message = if (granted) "Permission granted" else "Permission denied"
            Toast.makeText(applicationContext, message, Toast.LENGTH_SHORT).show()
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent {
            val ui by UiPreferences.settings.collectAsStateWithLifecycle()
            val strings = remember(ui.locale) {
                Localizations.strings(this, Localizations.denominator(ui.locale))
            }
            NMusicTheme(ui.theme) {
                CompositionLocalProvider(LocalStrings provides strings) {
                    var settings by rememberSaveable { mutableStateOf(false) }
                    if (settings) {
                        SettingsScreen(
                            onBack = { settings = false },
                            onPickFolder = { pickFolder.launch(null) },
                            onOpenLink = ::openLink,
                        )
                    } else {
                        AppScreen(onSettings = { settings = true })
                    }
                }
            }
        }
        // The media notification needs it from Android 13.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
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

    private fun openLink(link: String) {
        try {
            startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(link)))
        } catch (error: ActivityNotFoundException) {
            Log.w("n_music", "Nothing can open $link", error)
        }
    }

    private fun useFolder(uri: Uri) {
        val resolver = contentResolver
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
        // Picking a folder replaces the libraries until there is a screen to manage several.
        CoreRepository.send(
            Command.SetLibraryRoots(listOf(Locator.DocumentTree(uri.toString())))
        )
    }
}
