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
import com.enn3developer.n_music.LibraryTab
import com.enn3developer.n_music.R
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.RadioMark
import com.enn3developer.n_music.ui.components.SheetFrame
import com.enn3developer.n_music.ui.library.GroupOrder
import com.enn3developer.n_music.ui.library.TrackOrder
import com.enn3developer.n_music.ui.library.TrackSort
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/**
 * How one of the library's lists is sorted: a radio for each way. Picking one sorts the list and
 * closes the sheet; the chosen one of the tracks' carries its direction, which flips in place.
 */
@Composable
fun SortSheet(list: LibraryTab, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val ui by UiPreferences.settings.collectAsStateWithLifecycle()
    val title = stringResource(
        when (list) {
            LibraryTab.TRACKS -> R.string.sort_tracks
            LibraryTab.ALBUMS -> R.string.sort_albums
            LibraryTab.ARTISTS -> R.string.sort_artists
            LibraryTab.GENRES -> R.string.sort_genres
        }
    )
    val stored = ui.sorts[list.stored]
    SheetFrame(open, title, onDismissRequest, onGone, header = { SheetTitle(title) }) {
        Column(Modifier.selectableGroup()) {
            if (list == LibraryTab.TRACKS) {
                val order = TrackOrder.parse(stored, TrackOrder(TrackSort.ARTIST_ALBUM))
                for (sort in TrackSort.library) {
                    val chosen = order.sort == sort
                    SortOption(stringResource(sort.label), chosen, onClick = {
                        if (!chosen) UiPreferences.setSort(list.stored, TrackOrder(sort).stored)
                        onDismissRequest()
                    }) {
                        if (chosen) {
                            ReverseButton(stringResource(if (order.reversed) sort.backward else sort.forward)) {
                                UiPreferences.setSort(list.stored, order.copy(reversed = !order.reversed).stored)
                            }
                        }
                    }
                }
            } else {
                val albums = list == LibraryTab.ALBUMS
                val order = GroupOrder.parse(stored, if (albums) GroupOrder.ARTIST else GroupOrder.NAME)
                for (option in if (albums) GroupOrder.albums else GroupOrder.others) {
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
