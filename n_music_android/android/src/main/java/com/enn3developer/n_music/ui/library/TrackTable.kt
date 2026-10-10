package com.enn3developer.n_music.ui.library

import androidx.annotation.StringRes
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.ui.LocalPageMargins
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.TrackCover
import com.enn3developer.n_music.ui.components.TrackState
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.components.trackLine
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.formatLength
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** How wide a table must be to list tracks at all; narrower, they show as rows. */
val TrackTableWidth = 560.dp

/** How wide a table must be to keep its album column; narrower, the album joins the artist. */
val AlbumColumnWidth = 720.dp

private val AlbumColumn = 180.dp
private val NumberColumn = 48.dp
private val MoreColumn = 40.dp
private val ColumnGap = 16.dp

private fun coverSize(compact: Boolean): Dp = if (compact) 40.dp else 48.dp

/** The table's column names, over a line, held above the tracks as they scroll. */
@Composable
fun TrackTableHeader(album: Boolean, compact: Boolean, modifier: Modifier = Modifier) {
    val margins = LocalPageMargins.current
    val line = colors.outlineVariant
    Row(
        modifier
            .fillMaxWidth()
            .height(33.dp)
            .drawBehind { drawRect(line, Offset(0f, size.height - 1.dp.toPx()), Size(size.width, 1.dp.toPx())) }
            .padding(bottom = 1.dp)
            // Title starts over the titles, past the covers.
            .padding(start = margins.start + coverSize(compact) + ColumnGap, end = margins.end),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ColumnGap),
    ) {
        ColumnName(R.string.column_title, Modifier.weight(1f))
        if (album) ColumnName(R.string.column_album, Modifier.width(AlbumColumn))
        ColumnName(R.string.column_year, Modifier.width(NumberColumn), TextAlign.End)
        ColumnName(R.string.column_plays, Modifier.width(NumberColumn), TextAlign.End)
        ColumnName(R.string.column_time, Modifier.width(NumberColumn), TextAlign.End)
        Spacer(Modifier.width(MoreColumn))
    }
}

@Composable
private fun ColumnName(@StringRes name: Int, modifier: Modifier, align: TextAlign = TextAlign.Start) {
    Text(
        stringResource(name),
        style = text(12, FontWeight.Bold),
        color = colors.onSurfaceVariant,
        textAlign = align,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier = modifier,
    )
}

/**
 * A track's row in a table: its cover, title and artists, then its [album] when the table keeps
 * that column, year, plays and length, and ⋮. It shows the playing, picked and held tracks as
 * the list's rows do.
 */
@Composable
fun TrackTableRow(
    track: TrackRow,
    state: TrackState,
    onClick: () -> Unit,
    onLongClick: (() -> Unit)?,
    onMore: () -> Unit,
    album: Boolean,
    compact: Boolean,
    modifier: Modifier = Modifier,
) {
    val margins = LocalPageMargins.current
    val background by animateColorAsState(
        when {
            state.selected || state.held -> colors.surfaceHigh
            state.current -> colors.tint
            else -> Color.Transparent
        },
        NMotion.effectsDefault(),
        label = "row",
    )
    val quiet = if (track.loaded) colors.onSurfaceVariant else colors.onSurfaceQuiet
    Row(
        modifier
            .fillMaxWidth()
            .height(if (compact) 52.dp else 60.dp)
            .background(background)
            .tappable(onClick, onLongClick)
            .padding(start = margins.start, end = margins.end),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ColumnGap),
    ) {
        TrackCover(track, state, size = coverSize(compact))
        Column(Modifier.weight(1f)) {
            Text(
                track.title,
                style = text(15, FontWeight.SemiBold),
                color = when {
                    state.current -> colors.primary
                    !track.loaded -> colors.onSurfaceVariant
                    else -> colors.onSurface
                },
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                trackLine(track, album = !album),
                style = text(13),
                color = quiet,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(top = 1.dp),
            )
        }
        if (album) {
            Text(
                track.album.orEmpty(),
                style = text(14),
                color = quiet,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.width(AlbumColumn),
            )
        }
        NumberCell(track.year?.toString().orEmpty(), quiet)
        NumberCell(formatCount(track.plays), quiet)
        NumberCell(formatLength(track.length), quiet)
        NIconButton(
            NIcons.More,
            stringResource(R.string.more_for, track.title),
            onMore,
            size = 48.dp,
            iconSize = 20.dp,
            tint = colors.onSurfaceVariant,
            modifier = Modifier.width(MoreColumn),
        )
    }
}

@Composable
private fun NumberCell(value: String, color: Color) {
    Text(
        value,
        style = text(14, tabular = true),
        color = color,
        textAlign = TextAlign.End,
        maxLines = 1,
        modifier = Modifier.width(NumberColumn),
    )
}
