package com.enn3developer.n_music.ui.sheets

import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.PlaylistRow
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.RadioMark
import com.enn3developer.n_music.ui.components.SheetFrame
import com.enn3developer.n_music.ui.library.GroupOrder
import com.enn3developer.n_music.ui.library.SortedList
import com.enn3developer.n_music.ui.library.TrackOrder
import com.enn3developer.n_music.ui.library.TrackSort
import com.enn3developer.n_music.ui.playlists.playlistOrder
import com.enn3developer.n_music.ui.rememberLibrary
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/**
 * How one of the lists is sorted: a radio for each way. Picking one sorts the list. The chosen
 * way of a list of tracks carries its direction, which flips in place, so that sheet stays open
 * until the chosen way is tapped again; the others close on a pick.
 */
@Composable
fun SortSheet(list: SortedList, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val ui by UiPreferences.settings.collectAsStateWithLifecycle()
    val title = stringResource(list.title)
    SheetFrame(open, title, onDismissRequest, onGone, header = { SheetTitle(title) }) {
        Column(Modifier.selectableGroup()) {
            if (list.ofTracks) {
                val order = list.trackOrder(ui.sorts)
                for (sort in TrackSort.library) {
                    val chosen = order.sort == sort
                    SortOption(stringResource(sort.label), chosen, onClick = {
                        if (chosen) onDismissRequest() else UiPreferences.setSort(list.stored, TrackOrder(sort).stored)
                    }) {
                        if (chosen) {
                            ReverseButton(stringResource(if (order.reversed) sort.backward else sort.forward)) {
                                UiPreferences.setSort(list.stored, order.copy(reversed = !order.reversed).stored)
                            }
                        }
                    }
                }
            } else {
                val order = list.groupOrder(ui.sorts)
                for (option in if (list == SortedList.ALBUMS) GroupOrder.albums else GroupOrder.others) {
                    val chosen = order == option
                    SortOption(stringResource(option.label), chosen, onClick = {
                        if (!chosen) UiPreferences.setSort(list.stored, option.stored)
                        onDismissRequest()
                    })
                }
            }
        }
    }
}

/**
 * How playlist [id]'s tracks are sorted, kept with the playlist: by when they were added too,
 * for a plain one. Picking a way sorts it, and the sheet stays open for its direction, which
 * flips in place, until the chosen way is tapped again.
 */
@Composable
fun PlaylistSortSheet(id: Long, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val playlist = rememberLibrary<PlaylistRow?>(null, id) { CoreRepository.playlist(id) }
    val title = stringResource(R.string.sort_tracks)
    SheetFrame(open, title, onDismissRequest, onGone, header = { SheetTitle(title) }) {
        if (playlist == null) return@SheetFrame
        val order = playlistOrder(playlist)
        fun sortBy(chosen: TrackOrder) = CoreRepository.send(Command.SetPlaylistSort(id, chosen.keys(id)))
        Column(Modifier.selectableGroup()) {
            val options = if (playlist.rule == null) listOf(TrackSort.ADDED) + TrackSort.library else TrackSort.library
            for (sort in options) {
                val chosen = order.sort == sort
                SortOption(stringResource(sort.label), chosen, onClick = {
                    if (chosen) onDismissRequest() else sortBy(TrackOrder(sort))
                }) {
                    if (chosen) {
                        ReverseButton(stringResource(if (order.reversed) sort.backward else sort.forward)) {
                            sortBy(order.copy(reversed = !order.reversed))
                        }
                    }
                }
            }
        }
    }
}

/** A way to sort: a radio and its name, and [trailing] at the end of the row. */
@Composable
private fun SortOption(label: String, chosen: Boolean, onClick: () -> Unit, trailing: @Composable () -> Unit = {}) {
    Row(
        Modifier
            .fillMaxWidth()
            .height(56.dp)
            .padding(end = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Row(
            Modifier
                .weight(1f)
                .height(56.dp)
                .selectable(chosen, role = Role.RadioButton, onClick = onClick)
                .padding(start = 24.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            RadioMark(chosen)
            Text(
                label,
                style = text(16, if (chosen) FontWeight.Bold else FontWeight.Medium),
                color = colors.onSurface,
                maxLines = 1,
            )
        }
        trailing()
    }
}

/** The chosen sort's direction, which a tap flips: A to Z, Z to A. */
@Composable
private fun ReverseButton(direction: String, onClick: () -> Unit) {
    val description = stringResource(R.string.reverse_order)
    Row(
        Modifier
            .height(36.dp)
            .clip(RoundedCornerShape(18.dp))
            .border(1.dp, colors.outline, RoundedCornerShape(18.dp))
            .clickable(role = Role.Button, onClick = onClick)
            .semantics {
                contentDescription = description
                stateDescription = direction
            }
            .padding(start = 8.dp, end = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        NIcon(NIcons.Reverse, size = 18.dp, tint = colors.onSurface)
        Text(direction, style = text(13, FontWeight.Bold), color = colors.onSurface)
    }
}
