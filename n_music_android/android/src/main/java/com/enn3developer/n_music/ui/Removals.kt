package com.enn3developer.n_music.ui

import androidx.compose.runtime.Stable
import androidx.compose.runtime.mutableStateMapOf
import com.enn3developer.n_music.core.Locator

/**
 * Removals held back until their snackbar goes away, so Undo needs nothing from the core: what
 * they take out leaves the lists at once, and once sent, stays out of those read before the core
 * had it.
 */
@Stable
class Removals {
    /** Each key held, and the library version it was sent at; [WAITING] until then. */
    private val held = mutableStateMapOf<Any, Long>()

    /** Whether a list read at library [version] leaves out [key]. */
    fun hides(key: Any, version: Long): Boolean = held[key]?.let { version <= it } ?: false

    /** Takes [keys] out of the lists while their snackbar shows. */
    fun hold(keys: Collection<Any>) {
        for (key in keys) held[key] = WAITING
    }

    /** Puts [keys] back, for Undo. */
    fun release(keys: Collection<Any>) {
        for (key in keys) held.remove(key)
    }

    /** Marks [keys] as sent to the core at library [version]; lists read since show them gone. */
    fun sent(keys: Collection<Any>, version: Long) {
        // Those sent before the library changed since are in no list read now.
        for (stale in held.filterValues { it < version }.keys) held.remove(stale)
        for (key in keys) held[key] = version
    }

    private companion object {
        const val WAITING = Long.MAX_VALUE
    }
}

/** A track a playlist is about to lose. */
data class PlaylistTrack(val playlist: Long, val track: Locator)
