package com.enn3developer.n_music

import android.app.Application
import android.os.Build
import android.util.Log
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
    }
}
