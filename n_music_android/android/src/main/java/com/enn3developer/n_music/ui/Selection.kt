package com.enn3developer.n_music.ui

import androidx.compose.runtime.Stable
import androidx.compose.runtime.mutableStateSetOf
import com.enn3developer.n_music.core.Locator

/** The tracks picked while selecting, in the order they were picked. */
@Stable
class Selection(first: Locator) {
    private val picked = mutableStateSetOf(first)

    val tracks: List<Locator> get() = picked.toList()

    val count: Int get() = picked.size

    operator fun contains(track: Locator): Boolean = track in picked

    /** Picks [track], or lets it go if it was picked. */
    fun toggle(track: Locator) {
        if (!picked.remove(track)) picked.add(track)
    }

    fun addAll(tracks: List<Locator>) {
        picked.addAll(tracks)
    }
}

/** A short message over the mini player, with an action like Undo. */
data class Snack(val message: String, val action: String? = null, val onAction: (() -> Unit)? = null)
