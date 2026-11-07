package com.enn3developer.n_music.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.currentCompositeKeyHashCode
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

private object CacheRead {
    class Slot(val version: Long, val value: Any?)

    private const val KEPT_READS = 8

    private val reads = object : LinkedHashMap<List<Any?>, Slot>(16, 0.75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<List<Any?>, Slot>) =
            size > KEPT_READS
    }

    fun last(key: List<Any?>): Slot? = synchronized(reads) { reads[key] }

    fun keep(key: List<Any?>, version: Long, value: Any?) {
        synchronized(reads) { reads[key] = Slot(version, value) }
    }
}

/**
 * What [load] reads from the library, read off the main thread and again whenever the library
 * changes or one of [keys] does; [initial] until the first read. A read that is outdated before
 * it ends gives way to the next.
 */
@Composable
fun <T> rememberLibrary(initial: T, vararg keys: Any?, load: () -> T): T {
    val version by CoreRepository.version.collectAsStateWithLifecycle()
    val key = listOf(currentCompositeKeyHashCode, *keys)
    @Suppress("UNCHECKED_CAST")
    val start = remember(key) { CacheRead.last(key)?.let { it.value as T } ?: initial }
    val value by produceState(start, *keys, version) {
        val last = CacheRead.last(key)
        @Suppress("UNCHECKED_CAST")
        if (last != null) value = last.value as T
        if (last?.version == version) return@produceState
        val loaded = withContext(Dispatchers.IO) { load() }
        CacheRead.keep(key, version, loaded)
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
