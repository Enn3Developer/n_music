package com.enn3developer.n_music.ui

import androidx.compose.runtime.Stable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.Query

/** Where what plays came from, for the player's "Playing from". */
sealed interface Origin {
    data object Library : Origin
    data object Search : Origin
    data class Album(val name: String?, val artist: String?) : Origin
    data class Artist(val name: String?) : Origin
    data class Playlist(val id: Long) : Origin
    data class Source(val root: Locator) : Origin
}

/** What pages ask of the app: to move between pages and to play. */
@Stable
interface AppController {
    val navigator: Navigator

    fun open(page: Page)

    fun back()

    /**
     * Plays what [query] selects from [start], or from its first track. [shuffle] turns
     * shuffle on or off first; `null` keeps it as it is, as a tap on a row does.
     */
    fun play(query: Query, origin: Origin, start: Locator? = null, shuffle: Boolean? = null)

    fun openPlayer()
}

val LocalApp = staticCompositionLocalOf<AppController> { error("No app") }

/** The space floating controls take at the bottom of a page, for its lists to scroll past. */
val LocalBottomSpace = staticCompositionLocalOf { 0.dp }

/** How much room lists leave under their last row. */
fun bottomPadding(space: Dp): Dp = space + 8.dp
