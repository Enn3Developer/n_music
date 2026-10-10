package com.enn3developer.n_music.ui.sheets

import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.ui.components.SheetClose
import com.enn3developer.n_music.ui.components.inSideSheet
import com.enn3developer.n_music.ui.library.FilterField
import com.enn3developer.n_music.ui.library.TrackFilters
import com.enn3developer.n_music.ui.library.SortedList
import com.enn3developer.n_music.ui.settings.LanguageSheet
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** A sheet the app shows over its pages. */
sealed interface Sheet {
    /** How one of the lists is sorted. */
    data class Sort(val list: SortedList) : Sheet

    /** The filters on the library's tracks, opened at [focus]. */
    data class Filters(val focus: FilterField? = null) : Sheet

    /** Adds [tracks] to a playlist, or takes them out of one that has them all. */
    data class AddToPlaylist(val tracks: List<Locator>) : Sheet

    /** What can be done with one track, from [playlist]'s page when it is on one. */
    data class TrackActions(val track: Locator, val playlist: Long? = null) : Sheet

    /** The play session: what played, what plays and what plays next. */
    data object Queue : Sheet

    /** When playback fades out and pauses. */
    data object SleepTimer : Sheet

    /**
     * A smart playlist's rules: playlist [id]'s, opened at [focus], or a new one's, starting
     * from [start].
     */
    data class SmartPlaylist(val id: Long?, val focus: FilterField? = null, val start: TrackFilters? = null) : Sheet

    /** How playlist [id]'s tracks are sorted. */
    data class PlaylistSort(val id: Long) : Sheet

    /** The kinds of source the library can get. */
    data object AddSource : Sheet

    /** The app's language, where Android has no screen for it. */
    data object Language : Sheet
}

/**
 * The open sheet, and those still on their way out: each stays until it has left the screen.
 * [onDismiss] closes the open one.
 */
@Composable
fun SheetHost(current: Sheet?, onDismiss: () -> Unit) {
    val shown = remember { mutableStateListOf<Sheet>() }
    SideEffect {
        if (current != null && current !in shown) shown.add(current)
    }
    for (sheet in shown) {
        key(sheet) {
            val open = sheet == current
            val dismiss = { if (open) onDismiss() }
            val gone: () -> Unit = { shown.remove(sheet) }
            when (sheet) {
                is Sheet.Sort -> SortSheet(sheet.list, open, dismiss, gone)
                is Sheet.Filters -> FilterSheet(sheet.focus, open, dismiss, gone)
                is Sheet.AddToPlaylist -> AddToPlaylistSheet(sheet.tracks, open, dismiss, gone)
                is Sheet.TrackActions -> TrackActionsSheet(sheet.track, sheet.playlist, open, dismiss, gone)
                Sheet.Queue -> QueueSheet(open, dismiss, gone)
                Sheet.SleepTimer -> SleepTimerSheet(open, dismiss, gone)
                is Sheet.SmartPlaylist -> SmartPlaylistSheet(sheet.id, sheet.focus, sheet.start, open, dismiss, gone)
                is Sheet.PlaylistSort -> PlaylistSortSheet(sheet.id, open, dismiss, gone)
                Sheet.AddSource -> AddSourceSheet(open, dismiss, gone)
                Sheet.Language -> LanguageSheet(open, dismiss, gone)
            }
        }
    }
}

/** A sheet's title, under its handle; from the side, beside its close button. */
@Composable
fun ColumnScope.SheetTitle(title: String) {
    val style = text(22, FontWeight.Bold)
    if (inSideSheet) {
        Row(
            Modifier
                .fillMaxWidth()
                .height(64.dp)
                .padding(start = 24.dp, end = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(title, style = style, color = colors.onSurface, modifier = Modifier.weight(1f))
            SheetClose()
        }
    } else {
        Text(
            title,
            style = style,
            color = colors.onSurface,
            modifier = Modifier.padding(start = 24.dp, end = 24.dp, top = 4.dp, bottom = 12.dp),
        )
    }
}
