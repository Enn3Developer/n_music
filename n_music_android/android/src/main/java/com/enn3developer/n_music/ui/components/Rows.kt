package com.enn3developer.n_music.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.layout
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.AlbumRow
import com.enn3developer.n_music.core.ArtistRow
import com.enn3developer.n_music.core.GenreRow
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.formatLength
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import com.enn3developer.n_music.ui.tracksCount

/**
 * How a track shows in lists: playing, picked while selecting, [held] while its ⋮ sheet is
 * open, or none of them.
 */
data class TrackState(
    val current: Boolean = false,
    val playing: Boolean = false,
    val selected: Boolean = false,
    val held: Boolean = false,
)

/** The line under a track's title: its artists and album, or a stand-in while it isn't read. */
@Composable
fun trackLine(track: TrackRow, album: Boolean = true): String {
    val artist = track.artist.ifEmpty { stringResource(R.string.unknown_artist) }
    return if (album && track.loaded) dotted(artist, track.album) else artist
}

/**
 * A track's row: its cover, title, artists and album, length and ⋮. The playing track is washed
 * in the accent; a picked one shows a check on a highlighted row. [compact] makes it tighter;
 * [quiet] fades its line and length, and [muted] its title too, for one that can't play; one
 * without [onMore] has no ⋮.
 */
@Composable
fun TrackItem(
    track: TrackRow,
    state: TrackState,
    onClick: () -> Unit,
    onLongClick: (() -> Unit)?,
    onMore: (() -> Unit)?,
    modifier: Modifier = Modifier,
    compact: Boolean = false,
    title: AnnotatedString = AnnotatedString(track.title),
    line: String = trackLine(track),
    trailing: String = formatLength(track.length),
    quiet: Boolean = false,
    muted: Boolean = false,
    cover: @Composable (Modifier) -> Unit = { coverModifier ->
        TrackCover(track, state, coverModifier, if (compact) 40.dp else 48.dp)
    },
) {
    val background by animateColorAsState(
        when {
            state.selected || state.held -> colors.surfaceHigh
            state.current -> colors.tint
            else -> Color.Transparent
        },
        NMotion.effectsDefault(),
        label = "row",
    )
    Row(
        modifier
            .fillMaxWidth()
            .height(if (compact) 56.dp else 64.dp)
            .background(background)
            // One that can't play takes no taps.
            .then(if (muted) Modifier else Modifier.tappable(onClick, onLongClick))
            .padding(start = 16.dp, end = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(if (compact) 12.dp else 14.dp),
    ) {
        cover(Modifier)
        Column(Modifier.weight(1f)) {
            Text(
                title,
                style = text(if (compact) 15 else 16, FontWeight.Medium),
                color = when {
                    state.current -> colors.primary
                    muted -> colors.onSurfaceQuiet
                    !track.loaded -> colors.onSurfaceVariant
                    else -> colors.onSurface
                },
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                line,
                style = text(if (compact) 13 else 14),
                color = if (track.loaded && !quiet && !muted) colors.onSurfaceVariant else colors.onSurfaceQuiet,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(top = if (compact) 1.dp else 2.dp),
            )
        }
        if (trailing.isNotEmpty()) {
            Text(
                trailing,
                style = text(13, tabular = true),
                color = if (quiet || muted) colors.onSurfaceQuiet else colors.onSurfaceVariant,
            )
        }
        if (onMore == null) {
            Spacer(Modifier.width(44.dp))
        } else {
            NIconButton(
                NIcons.More,
                stringResource(R.string.more_for, track.title),
                onMore,
                size = 48.dp,
                iconSize = 20.dp,
                tint = colors.onSurfaceVariant,
                modifier = Modifier.width(44.dp),
            )
        }
    }
}

/**
 * A track's cover in a list: the picked one turns into a round check, the playing one carries
 * the moving bars, one not read yet shows the app's mark.
 */
@Composable
fun TrackCover(track: TrackRow, state: TrackState, modifier: Modifier = Modifier, size: Dp = 48.dp) {
    val radius by animateDpAsState(if (state.selected) size / 2 else 8.dp, NMotion.spatialFast(), label = "radius")
    val shape = RoundedCornerShape(radius)
    Box(modifier.size(size)) {
        Cover(
            track.cover,
            Modifier.fillMaxSize(),
            shape = shape,
            placeholder = if (track.loaded) CoverPlaceholder.ALBUM else CoverPlaceholder.UNREAD,
        )
        if (state.current && !state.selected) PlayingOverlay(shape, playing = state.playing)
        AnimatedVisibility(
            state.selected,
            enter = scaleIn(NMotion.spatialFast(), initialScale = 0.6f) + fadeIn(NMotion.effectsFast()),
            exit = scaleOut(NMotion.spatialFast(), targetScale = 0.6f) + fadeOut(NMotion.effectsFast()),
        ) {
            Box(
                Modifier
                    .fillMaxSize()
                    .background(colors.primary, CircleShape),
                contentAlignment = Alignment.Center,
            ) {
                NIcon(NIcons.CheckBold, size = size / 2, tint = colors.onPrimary)
            }
        }
    }
}

/** A track's tile in a grid: its cover with the title and artists under it, and its ⋮. */
@Composable
fun TrackTile(
    track: TrackRow,
    state: TrackState,
    onClick: () -> Unit,
    onLongClick: (() -> Unit)?,
    onMore: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier.tappable(onClick, onLongClick)) {
        val radius by animateDpAsState(if (state.selected) 48.dp else 12.dp, NMotion.spatialFast(), label = "radius")
        val shape = RoundedCornerShape(radius)
        Box(Modifier.fillMaxWidth().aspectRatio(1f)) {
            Cover(
                track.cover,
                Modifier.fillMaxSize(),
                shape = shape,
                placeholder = if (track.loaded) CoverPlaceholder.ALBUM else CoverPlaceholder.UNREAD,
            )
            if (state.current && !state.selected) PlayingOverlay(shape, playing = state.playing)
            if (state.selected) {
                Box(
                    Modifier
                        .fillMaxSize()
                        .background(colors.primary.copy(alpha = 0.85f), shape),
                    contentAlignment = Alignment.Center,
                ) {
                    NIcon(NIcons.CheckBold, size = 32.dp, tint = colors.onPrimary)
                }
            }
        }
        Row(Modifier.padding(top = 6.dp), verticalAlignment = Alignment.Top) {
            Column(Modifier.weight(1f)) {
                Text(
                    track.title,
                    style = text(14, FontWeight.SemiBold),
                    color = if (state.current) colors.primary else colors.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    track.artist.ifEmpty { stringResource(R.string.unknown_artist) },
                    style = text(12),
                    color = colors.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.padding(top = 1.dp),
                )
            }
            NIconButton(
                NIcons.More,
                stringResource(R.string.more_for, track.title),
                onMore,
                size = 44.dp,
                iconSize = 18.dp,
                tint = colors.onSurfaceVariant,
                modifier = Modifier
                    .overhang(end = 12.dp, top = 6.dp)
                    .width(40.dp),
            )
        }
    }
}

/**
 * Lets an element hang [end] past its row's end and [top] above it without taking that room,
 * as negative margins do: a tile's ⋮ keeps its touch area and the title keeps its width.
 */
private fun Modifier.overhang(end: Dp, top: Dp) = layout { measurable, constraints ->
    val placeable = measurable.measure(constraints)
    val shift = top.roundToPx()
    layout(placeable.width - end.roundToPx(), placeable.height - shift) {
        placeable.place(0, -shift)
    }
}

/** The pill on the cover of the playing album. */
@Composable
fun PlayingBadge(playing: Boolean, modifier: Modifier = Modifier) {
    Row(
        modifier
            .height(28.dp)
            .background(colors.primaryContainer, RoundedCornerShape(14.dp))
            .padding(horizontal = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        PlayingBars(color = colors.onPrimaryContainer, animate = playing)
        Text(stringResource(R.string.playing), style = text(12, FontWeight.Bold), color = colors.onPrimaryContainer)
    }
}

@Composable
private fun albumLine(album: AlbumRow): String =
    dotted(album.artist ?: stringResource(R.string.unknown_artist), album.year?.toString())

@Composable
fun albumName(album: AlbumRow): String = album.name ?: stringResource(R.string.no_album)

/** An album's tile: its cover, the Playing pill while one of its tracks plays, its name. */
@Composable
fun AlbumTile(
    album: AlbumRow,
    current: Boolean,
    playing: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    onLongClick: (() -> Unit)? = null,
) {
    Column(modifier.tappable(onClick, onLongClick)) {
        Box(Modifier.fillMaxWidth().aspectRatio(1f)) {
            Cover(album.cover, Modifier.fillMaxSize(), shape = RoundedCornerShape(16.dp))
            if (current) PlayingBadge(playing, Modifier.align(Alignment.BottomStart).padding(8.dp))
        }
        Text(
            albumName(album),
            style = text(15, FontWeight.SemiBold),
            color = colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.padding(top = 8.dp),
        )
        Text(
            albumLine(album),
            style = text(13, tabular = true),
            color = colors.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.padding(top = 1.dp),
        )
    }
}

/** A row of a list of albums, artists or genres: a 56 dp picture, a name and a line. */
@Composable
fun GroupItem(
    name: String,
    line: String,
    current: Boolean,
    playing: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    onLongClick: (() -> Unit)? = null,
    picture: @Composable (Modifier) -> Unit,
) {
    Row(
        modifier
            .fillMaxWidth()
            .height(72.dp)
            .tappable(onClick, onLongClick)
            .padding(start = 16.dp, end = 24.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        picture(Modifier.size(56.dp))
        Column(Modifier.weight(1f)) {
            Text(
                name,
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
                    line,
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

@Composable
fun AlbumItem(
    album: AlbumRow,
    current: Boolean,
    playing: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    onLongClick: (() -> Unit)? = null,
) {
    GroupItem(albumName(album), albumLine(album), current, playing, onClick, modifier, onLongClick) {
        Cover(album.cover, it, shape = RoundedCornerShape(10.dp))
    }
}

@Composable
fun artistName(artist: ArtistRow): String = artist.name ?: stringResource(R.string.no_artist)

@Composable
fun ArtistItem(
    artist: ArtistRow,
    current: Boolean,
    playing: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    onLongClick: (() -> Unit)? = null,
) {
    GroupItem(artistName(artist), tracksCount(artist.tracks), current, playing, onClick, modifier, onLongClick) {
        Cover(artist.cover, it, shape = CircleShape, placeholder = CoverPlaceholder.ARTIST)
    }
}

/** An artist's tile: a round picture with the name centred under it. */
@Composable
fun ArtistTile(
    artist: ArtistRow,
    current: Boolean,
    playing: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    onLongClick: (() -> Unit)? = null,
) {
    Column(modifier.tappable(onClick, onLongClick), horizontalAlignment = Alignment.CenterHorizontally) {
        Cover(
            artist.cover,
            Modifier.fillMaxWidth().aspectRatio(1f),
            shape = CircleShape,
            placeholder = CoverPlaceholder.ARTIST,
        )
        Text(
            artistName(artist),
            style = text(14, FontWeight.SemiBold),
            color = if (current) colors.primary else colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = 8.dp),
        )
        Row(
            Modifier.padding(top = 1.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text(tracksCount(artist.tracks), style = text(12, tabular = true), color = colors.onSurfaceVariant)
            if (current) PlayingBars(color = colors.primary, animate = playing)
        }
    }
}

@Composable
fun genreName(genre: GenreRow): String = genre.name ?: stringResource(R.string.no_genre)

@Composable
fun GenreItem(genre: GenreRow, onClick: () -> Unit, modifier: Modifier = Modifier) {
    GroupItem(genreName(genre), tracksCount(genre.tracks), current = false, playing = false, onClick, modifier) {
        Mosaic(genre.covers, it, shape = RoundedCornerShape(10.dp))
    }
}

/** A genre's tile: a mosaic of its albums' covers. */
@Composable
fun GenreTile(genre: GenreRow, onClick: () -> Unit, modifier: Modifier = Modifier) {
    Column(modifier.tappable(onClick)) {
        Mosaic(genre.covers, Modifier.fillMaxWidth().aspectRatio(1f), shape = RoundedCornerShape(16.dp))
        Text(
            genreName(genre),
            style = text(15, FontWeight.SemiBold),
            color = colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.padding(top = 8.dp),
        )
        Text(
            tracksCount(genre.tracks),
            style = text(13, tabular = true),
            color = colors.onSurfaceVariant,
            modifier = Modifier.padding(top = 1.dp),
        )
    }
}

/**
 * A track's row on its album's page: its number, or the moving bars while it plays, then its
 * title and length.
 */
@Composable
fun AlbumTrackItem(
    track: TrackRow,
    state: TrackState,
    onClick: () -> Unit,
    onLongClick: (() -> Unit)?,
    onMore: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val background by animateColorAsState(
        when {
            state.selected || state.held -> colors.surfaceHigh
            state.current -> colors.tint
            else -> Color.Transparent
        },
        NMotion.effectsDefault(),
        label = "row",
    )
    Row(
        modifier
            .fillMaxWidth()
            .height(56.dp)
            .background(background)
            .tappable(onClick, onLongClick)
            .padding(start = 16.dp, end = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(Modifier.width(28.dp), contentAlignment = Alignment.Center) {
            when {
                state.selected -> NIcon(NIcons.CheckBold, size = 20.dp, tint = colors.primary)
                state.current -> PlayingBars(color = colors.primary, animate = state.playing)
                else -> Text(
                    track.trackNumber?.toString().orEmpty(),
                    style = text(14, tabular = true),
                    color = colors.onSurfaceVariant,
                )
            }
        }
        Text(
            track.title,
            style = text(16, FontWeight.Medium),
            color = if (state.current) colors.primary else colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        val length = formatLength(track.length)
        if (length.isNotEmpty()) {
            Text(length, style = text(13, tabular = true), color = colors.onSurfaceVariant)
        }
        if (onMore == null) {
            Spacer(Modifier.width(44.dp))
        } else {
            NIconButton(
                NIcons.More,
                stringResource(R.string.more_for, track.title),
                onMore,
                size = 48.dp,
                iconSize = 20.dp,
                tint = colors.onSurfaceVariant,
                modifier = Modifier.width(44.dp),
            )
        }
    }
}

/** An album on its artist's page: a 124 dp tile with its year and how many tracks it has. */
@Composable
fun SmallAlbumTile(
    album: AlbumRow,
    current: Boolean,
    playing: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier.width(124.dp).tappable(onClick)) {
        Box(Modifier.size(124.dp)) {
            Cover(album.cover, Modifier.fillMaxSize(), shape = RoundedCornerShape(16.dp))
            if (current) {
                Box(
                    Modifier
                        .align(Alignment.BottomStart)
                        .padding(8.dp)
                        .height(24.dp)
                        .background(colors.primaryContainer, RoundedCornerShape(12.dp))
                        .padding(horizontal = 8.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    PlayingBars(color = colors.onPrimaryContainer, height = 12.dp, animate = playing)
                }
            }
        }
        Text(
            albumName(album),
            style = text(14, FontWeight.Bold),
            color = colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.padding(top = 8.dp),
        )
        Text(
            dotted(album.year?.toString(), tracksCount(album.tracks)),
            style = text(12, tabular = true),
            color = colors.onSurfaceVariant,
            maxLines = 1,
            modifier = Modifier.padding(top = 1.dp),
        )
    }
}
