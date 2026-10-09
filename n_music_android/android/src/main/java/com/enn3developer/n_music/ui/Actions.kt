package com.enn3developer.n_music.ui

import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator

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
