package com.enn3developer.n_music.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * What [load] reads from the library, read off the main thread and again whenever the library
 * changes or one of [keys] does; [initial] until the first read. A read that is outdated before
 * it ends gives way to the next.
 */
@Composable
fun <T> rememberLibrary(initial: T, vararg keys: Any?, load: () -> T): T {
    val version by CoreRepository.version.collectAsStateWithLifecycle()
    val value by produceState(initial, *keys, version) {
        value = withContext(Dispatchers.IO) { load() }
    }
    return value
}
