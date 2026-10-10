package com.enn3developer.n_music.ui.sheets

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.PlaylistId
import com.enn3developer.n_music.core.PlaylistRow
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.Snack
import com.enn3developer.n_music.ui.components.CheckMark
import com.enn3developer.n_music.ui.components.Mosaic
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.SheetClose
import com.enn3developer.n_music.ui.components.SheetFrame
import com.enn3developer.n_music.ui.components.inSideSheet
import com.enn3developer.n_music.ui.dialogs.AppDialog
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.rememberLibrary
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import com.enn3developer.n_music.ui.tracksCount
import kotlinx.coroutines.launch

/**
 * Adds [tracks] to a plain playlist, or a new one. A playlist that has them all takes them out
 * instead. Either way the sheet closes, selecting ends, and a snackbar offers to undo it.
 */
@Composable
fun AddToPlaylistSheet(tracks: List<Locator>, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val app = LocalApp.current
    val resources = LocalResources.current
    val playlists by CoreRepository.playlists.collectAsStateWithLifecycle()
    val plain = playlists.filter { it.rule == null }
    val holds = rememberLibrary(emptyMap(), plain, tracks) {
        plain.associate { it.id to CoreRepository.playlistHolds(it.id, tracks) }
    }
    val single = tracks.singleOrNull()
    val track = rememberLibrary<TrackRow?>(null, single) { single?.let(CoreRepository::track) }
    val undo = stringResource(R.string.undo)
    AddToPlaylistSheet(
        subtitle = track?.let { dotted(it.title, it.artist) } ?: tracksCount(tracks.size.toLong()),
        count = tracks.size,
        playlists = plain,
        holds = holds,
        onNew = {
            onDismissRequest()
            app.show(AppDialog.NewPlaylist(tracks))
            app.endSelection()
        },
        onPick = { playlist ->
            onDismissRequest()
            app.endSelection()
            val name = playlist.name
            if (holds[playlist.id] == tracks.size) {
                CoreRepository.send(Command.RemoveFromPlaylist(playlist.id, tracks))
                val count = tracks.size
                app.snack(
                    Snack(resources.getQuantityString(R.plurals.removed_from_playlist, quantity(count), formatCount(count), name), undo, onAction = {
                        CoreRepository.send(Command.AddToPlaylist(playlist.id, tracks))
                    })
                )
            } else {
                app.scope.launch {
                    val added = CoreRepository.addToPlaylist(playlist.id, tracks)
                    val count = added.size
                    app.snack(
                        Snack(resources.getQuantityString(R.plurals.added_to_playlist, quantity(count), formatCount(count), name), undo, onAction = {
                            CoreRepository.send(Command.RemoveFromPlaylist(playlist.id, added))
                        })
                    )
                }
            }
        },
        open = open,
        onDismissRequest = onDismissRequest,
        onGone = onGone,
    )
}

/**
 * The sheet itself: [playlists] with a tick on those that already have all [count] tracks, as
 * [holds] counts them per playlist.
 */
@Composable
fun AddToPlaylistSheet(
    subtitle: String,
    count: Int,
    playlists: List<PlaylistRow>,
    holds: Map<PlaylistId, Int>,
    onNew: () -> Unit,
    onPick: (PlaylistRow) -> Unit,
    open: Boolean,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
) {
    val title = stringResource(R.string.add_to_playlist)
    SheetFrame(
        open, title, onDismissRequest, onGone,
        end = 8.dp,
        header = {
            // 2 dp less above than the design's 4, as the handle here keeps 12 below it, not 10.
            val side = inSideSheet
            Row(
                Modifier
                    .then(if (side) Modifier.heightIn(min = 64.dp) else Modifier)
                    .padding(start = 24.dp, end = if (side) 12.dp else 24.dp, top = if (side) 0.dp else 2.dp, bottom = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text(title, style = text(22, FontWeight.Bold), color = colors.onSurface)
                    Text(
                        subtitle,
                        style = text(14),
                        color = colors.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.padding(top = 2.dp),
                    )
                }
                SheetClose()
            }
        },
    ) {
        Row(
            Modifier
                .fillMaxWidth()
                .height(64.dp)
                .clickable(role = Role.Button, onClick = onNew)
                .padding(horizontal = 24.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Box(
                Modifier
                    .size(48.dp)
                    .background(colors.secondaryContainer, RoundedCornerShape(14.dp)),
                contentAlignment = Alignment.Center,
            ) {
                NIcon(NIcons.Add, tint = colors.onSecondaryContainer)
            }
            Text(stringResource(R.string.new_playlist_item), style = text(16, FontWeight.SemiBold), color = colors.onSurface)
        }
        Box(
            Modifier
                .padding(horizontal = 24.dp, vertical = 6.dp)
                .fillMaxWidth()
                .height(1.dp)
                .background(colors.outlineVariant)
        )
        for (playlist in playlists) {
            val held = holds[playlist.id] ?: 0
            val all = count > 0 && held == count
            Row(
                Modifier
                    .fillMaxWidth()
                    .height(64.dp)
                    .toggleable(all, role = Role.Checkbox) { onPick(playlist) }
                    .padding(start = 24.dp, end = 20.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Mosaic(playlist.covers, Modifier.size(48.dp), shape = RoundedCornerShape(10.dp))
                Column(Modifier.weight(1f)) {
                    Text(
                        playlist.name,
                        style = text(16, FontWeight.SemiBold),
                        color = colors.onSurface,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        dotted(tracksCount(playlist.tracks), holdsLabel(held, count)),
                        style = text(13, tabular = true),
                        color = colors.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.padding(top = 1.dp),
                    )
                }
                CheckMark(all)
            }
        }
        Row(
            Modifier.padding(start = 24.dp, end = 24.dp, top = 10.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            NIcon(NIcons.Filter, size = 18.dp, tint = colors.onSurfaceVariant)
            Text(
                stringResource(R.string.add_to_playlist_hint),
                style = text(13, lineHeight = 16.sp),
                color = colors.onSurfaceVariant,
            )
        }
    }
}

/** What a playlist has of the tracks: this track, all of them, 2 of them; nothing for none. */
@Composable
private fun holdsLabel(held: Int, count: Int): String? = when {
    held == 0 -> null
    count == 1 -> stringResource(R.string.playlist_has_track)
    held >= count -> stringResource(R.string.playlist_has_all)
    else -> stringResource(R.string.playlist_has_some, formatCount(held))
}
