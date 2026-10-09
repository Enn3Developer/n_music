package com.enn3developer.n_music.ui.sheets

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.TrackDetails
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.Snack
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.CoverPlaceholder
import com.enn3developer.n_music.ui.components.InfoChip
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.SheetFrame
import com.enn3developer.n_music.ui.components.trackLine
import com.enn3developer.n_music.ui.enqueue
import com.enn3developer.n_music.ui.formatLength
import com.enn3developer.n_music.ui.trackFormat
import com.enn3developer.n_music.ui.formatWhen
import com.enn3developer.n_music.ui.rememberLibrary
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/**
 * What can be done with one track: queue it, add it to a playlist, or go to its album or artist.
 * Its header names it and tells its format, length and plays.
 */
@Composable
fun TrackActionsSheet(track: Locator, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val app = LocalApp.current
    val resources = LocalResources.current
    val row = rememberLibrary<TrackRow?>(null, track) { CoreRepository.track(track) }
    val details = rememberLibrary<TrackDetails?>(null, track) { CoreRepository.details(track) }
    fun queue(next: Boolean) {
        val title = row?.title ?: return
        val undo = enqueue(listOf(track), next)
        onDismissRequest()
        app.snack(
            Snack(
                resources.getString(if (next) R.string.track_plays_next else R.string.track_queued, title),
                resources.getString(R.string.undo),
                undo,
            )
        )
    }
    TrackActionsSheet(
        row,
        details,
        onPlayNext = { queue(next = true) },
        onQueue = { queue(next = false) },
        onAddToPlaylist = {
            onDismissRequest()
            app.show(Sheet.AddToPlaylist(listOf(track)))
        },
        onAlbum = {
            onDismissRequest()
            row?.let { app.open(albumOf(it)) }
        },
        onArtist = {
            onDismissRequest()
            row?.let { app.open(Page.Artist(it.artists.firstOrNull())) }
        },
        open = open,
        onDismissRequest = onDismissRequest,
        onGone = onGone,
    )
}

/** The album page a track is on, as the library groups albums: by album artist, or first artist. */
fun albumOf(track: TrackRow): Page.Album =
    if (track.album == null) {
        Page.Album(null, null)
    } else {
        Page.Album(track.album, track.albumArtist ?: track.artists.firstOrNull())
    }

/** The sheet itself, for [track] while it is read and [details] once they are. */
@Composable
fun TrackActionsSheet(
    track: TrackRow?,
    details: TrackDetails?,
    onPlayNext: () -> Unit,
    onQueue: () -> Unit,
    onAddToPlaylist: () -> Unit,
    onAlbum: () -> Unit,
    onArtist: () -> Unit,
    open: Boolean,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
) {
    val title = track?.title.orEmpty()
    SheetFrame(
        open, title, onDismissRequest, onGone,
        header = {
            // 4 dp less above than the design's 6, as the handle here keeps 12 below it, not 10.
            Row(
                Modifier.padding(start = 24.dp, end = 24.dp, top = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(14.dp),
            ) {
                Cover(
                    track?.cover,
                    Modifier.size(56.dp),
                    shape = RoundedCornerShape(12.dp),
                    placeholder = if (track?.loaded == false) CoverPlaceholder.UNREAD else CoverPlaceholder.ALBUM,
                )
                Column(Modifier.weight(1f)) {
                    Text(
                        title,
                        style = text(18, FontWeight.Bold),
                        color = colors.onSurface,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    if (track != null) {
                        Text(
                            trackLine(track),
                            style = text(14),
                            color = colors.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            modifier = Modifier.padding(top = 2.dp),
                        )
                    }
                }
            }
            Row(
                Modifier
                    .horizontalScroll(rememberScrollState())
                    .padding(horizontal = 24.dp, vertical = 14.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                details?.let { trackFormat(it) }?.let { InfoChip(it, colors.surfaceHigh) }
                track?.let { formatLength(it.length) }?.takeIf { it.isNotEmpty() }?.let { InfoChip(it, colors.surfaceHigh) }
                details?.let { InfoChip(playedLabel(it), colors.surfaceHigh) }
            }
        },
    ) {
        Line(Modifier.padding(start = 24.dp, end = 24.dp, bottom = 8.dp))
        Action(NIcons.PlayNext, stringResource(R.string.play_next), onPlayNext)
        Action(NIcons.AddToQueue, stringResource(R.string.add_to_queue), onQueue)
        Action(NIcons.Playlist, stringResource(R.string.add_to_playlist), onAddToPlaylist, opens = true)
        Line(Modifier.padding(horizontal = 24.dp, vertical = 8.dp))
        Action(NIcons.Album, stringResource(R.string.go_to_album), onAlbum)
        Action(NIcons.Artist, stringResource(R.string.go_to_artist), onArtist)
    }
}

/** Played 3× · yesterday, or Never played. */
@Composable
private fun playedLabel(details: TrackDetails): String {
    val last = details.lastPlayed
    return if (details.plays == 0u || last == null) {
        stringResource(R.string.plays_never)
    } else {
        stringResource(R.string.played_times, details.plays.toInt(), formatWhen(last))
    }
}

@Composable
private fun Line(modifier: Modifier) {
    Box(
        modifier
            .fillMaxWidth()
            .height(1.dp)
            .background(colors.outlineVariant)
    )
}

/** One of the sheet's actions; [opens] marks one that leads to another sheet. */
@Composable
private fun Action(icon: ImageVector, label: String, onClick: () -> Unit, opens: Boolean = false) {
    Row(
        Modifier
            .fillMaxWidth()
            .height(56.dp)
            .clickable(role = Role.Button, onClick = onClick)
            .padding(start = 24.dp, end = if (opens) 20.dp else 24.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        NIcon(icon, tint = colors.onSurfaceVariant)
        Text(label, style = text(16, FontWeight.Medium), color = colors.onSurface, modifier = Modifier.weight(1f))
        if (opens) NIcon(NIcons.Open, tint = colors.onSurfaceVariant)
    }
}
