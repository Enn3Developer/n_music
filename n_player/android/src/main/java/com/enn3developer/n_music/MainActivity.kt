package com.enn3developer.n_music

import android.app.NativeActivity
import android.content.Intent
import android.os.Bundle
import kotlin.concurrent.thread

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

        val ASK_DIRECTORY = 0
        lateinit var instance: MainActivity
        private fun checkThread() {
            while (true) {
                Thread.sleep(200)
                instance.check()
            }
        }
    }

    private external fun check()
    private external fun directory()

    private fun askDirectory() {
        println("asking directory")
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)

        startActivityForResult(intent, ASK_DIRECTORY)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        instance = this
        thread(block = { checkThread() })
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        println("got response")
        if (resultCode == RESULT_OK && requestCode == ASK_DIRECTORY) {
            data?.data?.also { uri ->
                println("got directory $uri")
//                val contentResolver = applicationContext.contentResolver
//                val takeFlags: Int = Intent.FLAG_GRANT_READ_URI_PERMISSION or
//                        Intent.FLAG_GRANT_WRITE_URI_PERMISSION
//                contentResolver.takePersistableUriPermission(uri, takeFlags)
            }
        }
    }

    fun hello() {
        println("checked")
    }
}
