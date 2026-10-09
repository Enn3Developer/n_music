package com.enn3developer.n_music.ui

import android.content.res.Resources
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.QueueRow
import com.enn3developer.n_music.core.TrackRow
import kotlinx.coroutines.launch

/**
 * Queues [tracks], to play [next] or after what is queued already. Returns what undoes it:
 * taking out the entries this added, and only those.
 */
fun enqueue(tracks: List<Locator>, next: Boolean): () -> Unit {
    val before = CoreRepository.queue.value.mapTo(HashSet()) { it.item }
    CoreRepository.send(Command.Enqueue(tracks, next))
    val added = tracks.toHashSet()
    return {
        for (row in CoreRepository.queue.value) {
            if (row.queued && row.item !in before && row.track.locator in added) {
                CoreRepository.send(Command.RemoveQueued(row.item))
            }
        }
    }
}

/**
 * Takes [track] out of [playlist] at once, as far as the lists show, with Undo on a snackbar; the
 * core hears of it only once the snackbar goes away, so Undo keeps its place and date added.
 */
fun AppController.removeFromPlaylist(playlist: Long, track: TrackRow, resources: Resources) {
    val held = listOf(PlaylistTrack(playlist, track.locator))
    removals.hold(held)
    snack(
        Snack(
            resources.getString(R.string.removed_track, track.title),
            resources.getString(R.string.undo),
            onAction = { removals.release(held) },
            onGone = {
                val version = CoreRepository.version.value
                CoreRepository.send(Command.RemoveFromPlaylist(playlist, listOf(track.locator)))
                removals.sent(held, version)
            },
        )
    )
}

/**
 * Takes queued [rows] out of the queue at once, as far as it shows, with Undo on a snackbar
 * saying [message]; the core hears of it once the snackbar goes away, so Undo puts them back
 * where they were. One that comes up to play meanwhile is skipped.
 */
fun AppController.removeQueued(rows: List<QueueRow>, message: String, resources: Resources) {
    val held = rows.map { QueuedItem(it.item) }
    val items = rows.mapTo(HashSet()) { it.item }
    removals.hold(held)
    val skipping = scope.launch {
        CoreRepository.current.collect { if (it?.item in items) CoreRepository.send(Command.PlayNext) }
    }
    snack(
        Snack(
            message,
            resources.getString(R.string.undo),
            onAction = {
                skipping.cancel()
                removals.release(held)
            },
            onGone = {
                skipping.cancel()
                for (item in items) CoreRepository.send(Command.RemoveQueued(item))
                removals.sent(held, CoreRepository.version.value)
            },
        )
    )
}
