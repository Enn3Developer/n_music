package com.enn3developer.n_music.ui.components

import android.graphics.BitmapFactory
import android.util.LruCache
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import com.enn3developer.n_music.ui.theme.LogoMark
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Decoded covers by path, so the tracks of an album share one bitmap. */
object Covers {
    private val cache = object : LruCache<String, ImageBitmap>(
        (Runtime.getRuntime().maxMemory() / 16).toInt()
    ) {
        override fun sizeOf(key: String, value: ImageBitmap) = value.width * value.height * 4
    }

    fun cached(path: String): ImageBitmap? = cache.get(path)

    /** The cover at [path], decoded off the main thread; the core keeps 256 px thumbnails. */
    suspend fun load(path: String): ImageBitmap? = withContext(Dispatchers.IO) {
        cache.get(path)?.let { return@withContext it }
        val bitmap = BitmapFactory.decodeFile(path) ?: return@withContext null
        bitmap.asImageBitmap().also { cache.put(path, it) }
    }
}

/** The cover at [path], once decoded; `null` meanwhile and when there is none. */
@Composable
fun rememberCover(path: String?): ImageBitmap? {
    // The state outlives a change of path: the old cover goes as soon as the path changes.
    val bitmap by produceState(path?.let(Covers::cached), path) {
        value = path?.let(Covers::cached)
        if (path != null && value == null) value = Covers.load(path)
    }
    return bitmap
}

/** What stands in for a cover a track lacks. */
enum class CoverPlaceholder {
    /** A disc: the track was read and has no cover. */
    ALBUM,

    /** The app's mark: the track was not read yet. */
    UNREAD,

    /** An artist's silhouette. */
    ARTIST,

    /** A playlist's lines. */
    PLAYLIST,
}

/** A cover cropped to [shape], or its [placeholder] on the highest surface. */
@Composable
fun Cover(
    path: String?,
    modifier: Modifier = Modifier,
    shape: Shape = RectangleShape,
    placeholder: CoverPlaceholder = CoverPlaceholder.ALBUM,
) {
    val bitmap = rememberCover(path)
    Box(modifier.clip(shape), contentAlignment = Alignment.Center) {
        if (bitmap != null) {
            Image(
                bitmap,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize(),
            )
        } else {
            Placeholder(placeholder)
        }
    }
}

@Composable
private fun Placeholder(kind: CoverPlaceholder) {
    BoxWithConstraints(
        Modifier
            .fillMaxSize()
            .background(colors.surfaceHighest),
        contentAlignment = Alignment.Center,
    ) {
        when (kind) {
            CoverPlaceholder.UNREAD -> LogoMark(
                marks = colors.onSurfaceVariant,
                letter = colors.onSurface,
                size = maxWidth * 34 / 48,
            )

            else -> {
                val icon: ImageVector = when (kind) {
                    CoverPlaceholder.ARTIST -> NIcons.Artist
                    CoverPlaceholder.PLAYLIST -> NIcons.Playlist
                    else -> NIcons.Album
                }
                NIcon(icon, size = maxWidth * 0.42f, tint = colors.onSurfaceQuiet)
            }
        }
    }
}

/**
 * Up to four covers in a 2 × 2 mosaic, as playlists and genres show them: one cover fills it
 * while there are fewer than four.
 */
@Composable
fun Mosaic(covers: List<String>, modifier: Modifier = Modifier, shape: Shape = RectangleShape) {
    Box(modifier.clip(shape)) {
        if (covers.size < 4) {
            Cover(covers.firstOrNull(), Modifier.fillMaxSize(), placeholder = CoverPlaceholder.PLAYLIST)
        } else {
            Column(Modifier.fillMaxSize()) {
                for (row in covers.take(4).chunked(2)) {
                    Row(Modifier.fillMaxWidth().weight(1f)) {
                        for (cover in row) {
                            Cover(cover, Modifier.weight(1f).fillMaxSize())
                        }
                    }
                }
            }
        }
    }
}

/** The dark wash and moving bars on the cover of the playing track. */
@Composable
fun PlayingOverlay(shape: Shape, modifier: Modifier = Modifier, playing: Boolean = true, alpha: Float = 0.5f) {
    Box(
        modifier
            .fillMaxSize()
            .clip(shape)
            .background(Color.Black.copy(alpha = alpha)),
        contentAlignment = Alignment.Center,
    ) {
        PlayingBars(color = Color.White, animate = playing)
    }
}
