package com.enn3developer.n_music.ui.playlists

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.PlayingFrom
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.PlaylistRow
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.LocalBottomSpace
import com.enn3developer.n_music.ui.LocalPageMargins
import com.enn3developer.n_music.ui.LocalWindowLayout
import com.enn3developer.n_music.ui.Origin
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.bottomPadding
import com.enn3developer.n_music.ui.components.EmptyState
import com.enn3developer.n_music.ui.components.Mosaic
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.PlayingBars
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.formatWhen
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NType
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import com.enn3developer.n_music.ui.tracksCount

/** Every playlist, plain and smart, by name; the one playing shows it. */
@Composable
fun PlaylistsPage() {
    val app = LocalApp.current
    val playlists by CoreRepository.playlists.collectAsStateWithLifecycle()
    val origin by PlayingFrom.origin.collectAsStateWithLifecycle()
    val current by CoreRepository.current.collectAsStateWithLifecycle()
    val playing by CoreRepository.playing.collectAsStateWithLifecycle()
    PlaylistsContent(
        playlists = playlists,
        playingFrom = (origin as? Origin.Playlist)?.id?.takeIf { current != null },
        playing = playing,
        onOpen = { app.open(Page.Playlist(it.id)) },
        // Beside a rail, Settings is on it.
        onSettings = { app.open(Page.Settings) }.takeUnless { LocalWindowLayout.current.rail },
    )
}

/** The page itself: [playingFrom] is the playlist playing, its bars moving while [playing]. */
@Composable
fun PlaylistsContent(
    playlists: List<PlaylistRow>,
    playingFrom: Long?,
    playing: Boolean,
    onOpen: (PlaylistRow) -> Unit,
    onSettings: (() -> Unit)?,
) {
    val smart = playlists.count { it.rule != null }
    val margins = LocalPageMargins.current
    LazyColumn(
        Modifier.fillMaxSize(),
        // The last row scrolls clear of the new playlist button.
        contentPadding = PaddingValues(bottom = bottomPadding(LocalBottomSpace.current) + 72.dp),
    ) {
        item(key = "bar") {
            Row(
                Modifier
                    .windowInsetsPadding(WindowInsets.statusBars)
                    .fillMaxWidth()
                    .height(64.dp)
                    .padding(horizontal = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.End,
            ) {
                if (onSettings != null) {
                    NIconButton(NIcons.Settings, stringResource(R.string.settings), onSettings, tint = colors.onSurfaceVariant)
                }
            }
        }
        item(key = "title") {
            Column(Modifier.padding(start = margins.start, end = margins.end, bottom = 10.dp)) {
                Text(stringResource(R.string.playlists_title), style = NType.headline, color = colors.onSurface)
                Text(
                    dotted(
                        pluralStringResource(R.plurals.playlists_count, quantity(playlists.size), formatCount(playlists.size)),
                        if (smart > 0) stringResource(R.string.playlists_smart_count, formatCount(smart)) else null,
                    ),
                    style = text(14, tabular = true),
                    color = colors.onSurfaceVariant,
                    modifier = Modifier.padding(top = 2.dp),
                )
            }
        }
        if (playlists.isEmpty()) {
            item(key = "empty") {
                EmptyState(
                    NIcons.Playlist,
                    stringResource(R.string.playlists_empty),
                    stringResource(R.string.playlists_empty_hint),
                    Modifier.fillParentMaxHeight(0.7f),
                )
            }
        }
        items(playlists, key = { it.id }) { playlist ->
            PlaylistItem(playlist, playlist.id == playingFrom, playing, { onOpen(playlist) })
        }
    }
}

/** A playlist's row: its tile, name and what it holds. */
@Composable
private fun PlaylistItem(playlist: PlaylistRow, current: Boolean, playing: Boolean, onClick: () -> Unit) {
    val margins = LocalPageMargins.current
    Row(
        Modifier
            .fillMaxWidth()
            .height(72.dp)
            .tappable(onClick)
            .padding(start = margins.start, end = margins.end),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        PlaylistTile(playlist, 56.dp)
        Column(Modifier.weight(1f)) {
            Text(
                playlist.name,
                style = text(16, FontWeight.SemiBold),
                color = if (current) colors.primary else colors.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Row(
                Modifier.padding(top = 2.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(
                    playlistLine(playlist),
                    style = text(14, tabular = true),
                    color = colors.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f, fill = false),
                )
                if (current) PlayingBars(color = colors.primary, animate = playing)
            }
        }
    }
}

/** Smart · 63 tracks, or 34 tracks · changed yesterday. */
@Composable
private fun playlistLine(playlist: PlaylistRow): String =
    if (playlist.rule != null) {
        stringResource(R.string.playlist_smart_line, tracksCount(playlist.tracks))
    } else {
        dotted(tracksCount(playlist.tracks), stringResource(R.string.playlist_changed, formatWhen(playlist.modified)))
    }

/**
 * A playlist's tile at [size]: a smart one's filter, a plain one's covers, or the playlist mark
 * while it has none.
 */
@Composable
fun PlaylistTile(playlist: PlaylistRow, size: Dp) {
    val big = size > 64.dp
    if (playlist.rule != null) {
        val shape = RoundedCornerShape(if (big) 20.dp else 14.dp)
        Box(
            Modifier
                .size(size)
                .background(colors.secondaryContainer, shape),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(
                if (big) NIcons.FilterThin else NIcons.Filter,
                size = if (big) 44.dp else 24.dp,
                tint = colors.onSecondaryContainer,
            )
        }
    } else if (playlist.covers.isEmpty()) {
        val shape = RoundedCornerShape(if (big) 20.dp else 12.dp)
        Box(
            Modifier
                .size(size)
                .background(colors.surfaceHigh, shape),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(
                if (big) NIcons.PlaylistThin else NIcons.Playlist,
                size = if (big) 44.dp else 24.dp,
                tint = colors.onSurfaceVariant,
            )
        }
    } else {
        Mosaic(playlist.covers, Modifier.size(size), RoundedCornerShape(if (big) 20.dp else 12.dp))
    }
}
