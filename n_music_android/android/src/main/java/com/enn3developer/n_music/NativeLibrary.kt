package com.enn3developer.n_music

import android.content.Context

/**
 * Loads the native core for the JVM. UniFFI's bindings load it again through JNA, which reuses
 * the library loaded here; only [System.loadLibrary] lets the JVM find [init].
 */
object NativeLibrary {
    init {
        System.loadLibrary("n_music_ffi")
    }

    /** Hands the JVM and [context] to the native side, before `Core.start`. */
    @JvmStatic
    external fun init(context: Context)
}
