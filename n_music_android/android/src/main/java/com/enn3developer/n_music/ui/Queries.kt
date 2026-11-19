package com.enn3developer.n_music.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.LibraryQuery
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

@Composable
fun <T> rememberLibrary(initial: T, query: LibraryQuery<T>): T {
    val version by CoreRepository.version.collectAsStateWithLifecycle()
    val start = remember(query) { CoreRepository.kept(query).let { if (it != null) it.value else initial } }
    val value by produceState(start, query, version) {
        CoreRepository.kept(query)?.let { value = it.value }
        value = withContext(Dispatchers.IO) { CoreRepository.read(query) }
    }
    return value
}

/**
 * What [load] reads from the library, read off the main thread and again whenever the library
 * changes or one of [keys] does; [initial] until the first read. Nothing is kept: for reads that
 * aren't worth keeping, like a search as it's typed.
 */
@Composable
fun <T> rememberLibrary(initial: T, vararg keys: Any?, load: () -> T): T {
    val version by CoreRepository.version.collectAsStateWithLifecycle()
    val value by produceState(initial, *keys, version) {
        value = withContext(Dispatchers.IO) { load() }
    }
    return value
}

/**
 * [rememberLibrary] with the version each read began at, for lists that leave out what
 * [Removals] holds; `null` until the first read ends.
 */
@Composable
fun <T> rememberLibraryRead(query: LibraryQuery<T>): CoreRepository.LibraryRead<T>? {
    val version by CoreRepository.version.collectAsStateWithLifecycle()
    val read by produceState(remember(query) { CoreRepository.kept(query) }, query, version) {
        CoreRepository.kept(query)?.let { value = it }
        val at = version
        value = CoreRepository.LibraryRead(
            withContext(Dispatchers.IO) { CoreRepository.read(query) },
            at
        )
    }
    return read
}
