package com.enn3developer.n_music.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class CacheSlot<T>(initial: T) {
    internal var keys: List<Any?>? = null
    internal var version: Long = -1
    internal var value: T = initial
}

/**
 * What [load] reads from the library, read off the main thread and again whenever the library
 * changes or one of [keys] does; [initial] until the first read. A read that is outdated before
 * it ends gives way to the next.
 */
@Composable
fun <T> rememberLibrary(initial: T, vararg keys: Any?, cache: CacheSlot<T>? = null, load: () -> T): T {
    val version by CoreRepository.version.collectAsStateWithLifecycle()
    val current = keys.toList()
    val start = if (cache != null && cache.keys == current) cache.value else initial
    val value by produceState(start, *keys, version) {
        if (cache != null && cache.keys == current && cache.version == version) {
            return@produceState
        }
        val loaded = withContext(Dispatchers.IO) { load() }
        cache?.apply {
            this.keys = current
            this.version = version
            this.value = loaded
        }
        value = loaded
    }
    return value
}

/** A read of the library, and the library's version as it began. */
data class LibraryRead<T>(val value: T, val version: Long)

/**
 * [rememberLibrary] with the version each read began at, for lists that leave out what
 * [Removals] holds; `null` until the first read ends.
 */
@Composable
fun <T> rememberLibraryRead(vararg keys: Any?, load: () -> T): LibraryRead<T>? {
    val version by CoreRepository.version.collectAsStateWithLifecycle()
    val read by produceState<LibraryRead<T>?>(null, *keys, version) {
        val at = version
        value = LibraryRead(withContext(Dispatchers.IO) { load() }, at)
    }
    return read
}
