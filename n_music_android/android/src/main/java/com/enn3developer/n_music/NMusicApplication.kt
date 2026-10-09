package com.enn3developer.n_music

import android.app.Application
import android.os.Build
import android.util.Log
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner
import com.enn3developer.n_music.core.Command
import java.io.File

class NMusicApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        val reportFile = File(getExternalFilesDir(null) ?: filesDir, "config/n_music_panic.log")
        val previous = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { thread, error ->
            try {
                val report = "\n=== n_music ${BuildConfig.VERSION_NAME} Android exception; " +
                    "SDK=${Build.VERSION.SDK_INT}; device=${Build.MANUFACTURER}/${Build.MODEL}; " +
                    "unix_ms=${System.currentTimeMillis()} ===\n" +
                    "Thread: ${thread.name}\n${error.stackTraceToString()}\n"
                Log.e("n_music", report)
                reportFile.parentFile?.mkdirs()
                reportFile.appendText(report)
            } catch (reportError: Throwable) {
                Log.e("n_music", "Could not write crash report to $reportFile", reportError)
            } finally {
                if (previous != null) previous.uncaughtException(thread, error)
                else error.printStackTrace()
            }
        }

        // Every process start runs this, whether for the UI, the playback service or a media
        // button, so the core is there for whichever comes first and lives as long as the
        // process.
        NativeLibrary.init(this)
        CoreRepository.start(this)
        UiPreferences.load()
        PlayingFrom.load()
        SleepTimer.recover()
        // Hidden until an activity starts: a process started for a headset button has none.
        CoreRepository.send(Command.AppVisibilityChanged(false))

        // Visibility is the process's, not an activity's: rotating or switching activities is
        // not leaving the app.
        ProcessLifecycleOwner.get().lifecycle.addObserver(object : DefaultLifecycleObserver {
            override fun onStart(owner: LifecycleOwner) {
                CoreRepository.send(Command.AppVisibilityChanged(true))
            }

            override fun onStop(owner: LifecycleOwner) {
                CoreRepository.send(Command.AppVisibilityChanged(false))
            }
        })
    }
}
