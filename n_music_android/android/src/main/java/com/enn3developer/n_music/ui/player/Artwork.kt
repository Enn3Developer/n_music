package com.enn3developer.n_music.ui.player

import android.graphics.BitmapFactory
import android.util.LruCache
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.key
import com.enn3developer.n_music.ui.components.rememberCover
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * The covers the player shows, read from the tracks' files at the size it shows them: the
 * library keeps only thumbnails, too small for a cover across the screen. It keeps a few, for
 * skipping back and forth.
 */
object Artwork {
    private val cache = LruCache<String, ImageBitmap>(4)

    fun cached(track: Locator): ImageBitmap? = cache.get(track.key)

    /** The picture in the file of [track], decoded to at least [size] pixels across if it has them. */
    suspend fun load(track: Locator, size: Int): ImageBitmap? = withContext(Dispatchers.IO) {
        cache.get(track.key)?.let { return@withContext it }
        val data = CoreRepository.coverArt(track) ?: return@withContext null
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(data, 0, data.size, bounds)
        var sample = 1
        while (minOf(bounds.outWidth, bounds.outHeight) / (sample * 2) >= size) sample *= 2
        val options = BitmapFactory.Options().apply { inSampleSize = sample }
        val bitmap = BitmapFactory.decodeByteArray(data, 0, data.size, options) ?: return@withContext null
        bitmap.asImageBitmap().also { cache.put(track.key, it) }
    }
}

/**
 * The cover of [track] for the player: its thumbnail at once, then the picture in its file once
 * read, at least [size] pixels across.
 */
@Composable
fun rememberArtwork(track: TrackRow?, size: Int): ImageBitmap? {
    val thumbnail = rememberCover(track?.cover)
    val locator = track?.locator
    // Only a track with a thumbnail has a picture in its file.
    val hasCover = track?.cover != null
    val full by produceState(locator?.let(Artwork::cached), locator, hasCover, size) {
        value = locator?.let(Artwork::cached)
        if (locator != null && hasCover && value == null && size > 0) value = Artwork.load(locator, size)
    }
    return full ?: thumbnail
}
