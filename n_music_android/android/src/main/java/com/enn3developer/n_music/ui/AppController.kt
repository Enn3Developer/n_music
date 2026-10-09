package com.enn3developer.n_music.ui

import androidx.compose.runtime.Stable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.ui.dialogs.AppDialog
import com.enn3developer.n_music.ui.components.PlaybackActions
import com.enn3developer.n_music.ui.library.TrackFilters
import com.enn3developer.n_music.ui.player.PlayerTransition
import com.enn3developer.n_music.ui.sheets.Sheet
import kotlinx.coroutines.CoroutineScope

/** Where what plays came from, for the player's "Playing from". */
sealed interface Origin {
    data object Library : Origin
    data class Search(val text: String) : Origin
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

    /** Opens the player out of the mini player. */
    fun openPlayer()

    /** The player's way in and out, which the mini player and the navigation follow. */
    val player: PlayerTransition

    /** Playback's controls, with the output and the sleep timer opening what they open. */
    val playback: PlaybackActions

    /** The filters on the library's tracks. */
    var filters: TrackFilters

    /** Shows the library's tracks filtered by [filters], on the Library tab's own page. */
    fun showTracks(filters: TrackFilters)

    /** How many times [showTracks] ran, for the library page to turn to its tracks. */
    val tracksShown: Int

    /** The sheet over the app, while one shows. */
    val sheet: Sheet?

    /** Shows [sheet] over the app, in place of the one showing. */
    fun show(sheet: Sheet)

    fun closeSheet()

    /** The dialog over the app, while one shows. */
    val dialog: AppDialog?

    fun show(dialog: AppDialog)

    fun closeDialog()

    /** The tracks picked while selecting; `null` while not selecting. */
    val selection: Selection?

    /** Starts selecting with [track], or while selecting, picks it or lets it go. */
    fun select(track: Locator)

    fun endSelection()

    /** The message over the mini player, numbered so the same one twice shows twice. */
    val snack: Pair<Long, Snack>?

    /** Shows [snack]; the one it replaces is gone. */
    fun snack(snack: Snack)

    /** Lets go of snack [id], unless another has replaced it. */
    fun dismissSnack(id: Long)

    /** Runs the action of snack [id], unless another has replaced it, and lets it go. */
    fun snackAction(id: Long)

    /** Removals waiting on their snackbar, and the lists they leave out of. */
    val removals: Removals

    /** Opens Android's folder picker; [onPicked] gets the folder as a source, readable from then on. */
    fun pickFolder(onPicked: (Locator) -> Unit)

    /** Lets go of [root]'s folder: Android stops keeping it readable for the app. */
    fun releaseFolder(root: Locator)

    /** Runs work that outlives the page or sheet that started it, like an undo's preparation. */
    val scope: CoroutineScope
}

val LocalApp = staticCompositionLocalOf<AppController> { error("No app") }

/** The space floating controls take at the bottom of a page, for its lists to scroll past. */
val LocalBottomSpace = staticCompositionLocalOf { 0.dp }

/** How much room lists leave under their last row. */
fun bottomPadding(space: Dp): Dp = space + 8.dp
