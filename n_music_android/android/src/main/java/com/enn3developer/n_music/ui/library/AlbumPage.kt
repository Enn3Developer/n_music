package com.enn3developer.n_music.ui.library

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.AlbumRow
import com.enn3developer.n_music.core.Filter
import com.enn3developer.n_music.core.GroupSort
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.core.SortField
import com.enn3developer.n_music.core.SortKey
import com.enn3developer.n_music.core.Tag
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.key
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.LocalBottomSpace
import com.enn3developer.n_music.ui.LocalPageMargins
import com.enn3developer.n_music.ui.LocalWindowLayout
import com.enn3developer.n_music.ui.Origin
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.Selection
import com.enn3developer.n_music.ui.Snack
import com.enn3developer.n_music.ui.WindowLayout
import com.enn3developer.n_music.ui.bottomPadding
import com.enn3developer.n_music.ui.components.AlbumTrackItem
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.MenuDivider
import com.enn3developer.n_music.ui.components.MenuItem
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.NMenu
import com.enn3developer.n_music.ui.components.NamedPlayShuffle
import com.enn3developer.n_music.ui.components.PageBar
import com.enn3developer.n_music.ui.components.PlayShuffle
import com.enn3developer.n_music.ui.components.SelectionBar
import com.enn3developer.n_music.ui.components.albumName
import com.enn3developer.n_music.ui.components.barDrift
import com.enn3developer.n_music.ui.components.barSwap
import com.enn3developer.n_music.ui.components.floating
import com.enn3developer.n_music.ui.components.margins
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.enqueue
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.formatDuration
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.rememberLibrary
import com.enn3developer.n_music.ui.sheets.Sheet
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import com.enn3developer.n_music.ui.tracksCount

/**
 * The tracks of an album as the library groups them: by its name, and its album artist or first
 * artist. Without an album artist tag the first artist cannot be told apart from the others, so a
 * track another album of the same name shares an artist with shows on both.
 */
fun albumFilter(name: String?, artist: String?): Filter = when {
    name == null -> Filter.Untagged(Tag.ALBUM)
    artist == null -> Filter.All(listOf(Filter.Album(name), Filter.Untagged(Tag.ARTIST)))
    else -> Filter.All(listOf(Filter.Album(name), Filter.Any(listOf(Filter.AlbumArtist(artist), Filter.Artist(artist)))))
}

/** An album's page: its cover and details, Play and Shuffle, and its tracks in disc order. */
@Composable
fun AlbumPage(page: Page.Album) {
    val app = LocalApp.current
    val resources = LocalResources.current
    val filter = remember(page) { albumFilter(page.name, page.artist) }
    val album = rememberLibrary<AlbumRow?>(null, filter) { CoreRepository.albums(filter, "", GroupSort.ARTIST).firstOrNull() }
    val query = remember(filter) { Query(filter, listOf(SortKey(SortField.Album, false))) }
    val tracks = rememberLibrary<List<TrackRow>?>(null, query) { CoreRepository.tracks(query) }
    val origin = Origin.Album(page.name, page.artist)
    val selection = app.selection
    BackHandler(selection != null) { app.endSelection() }

    fun queue(next: Boolean) {
        val all = tracks.orEmpty().map { it.locator }
        if (all.isEmpty()) return
        val undo = enqueue(all, next)
        val count = all.size
        app.snack(
            Snack(
                resources.getQuantityString(if (next) R.plurals.queued_next else R.plurals.queued, quantity(count), formatCount(count)),
                resources.getString(R.string.undo),
                undo,
            )
        )
    }

    AlbumContent(
        album = album,
        name = page.name,
        tracks = tracks,
        nowPlaying = rememberNowPlaying(),
        selection = selection,
        held = (app.sheet as? Sheet.TrackActions)?.track,
        onBack = app::back,
        onArtist = page.artist?.let { artist -> { app.open(Page.Artist(artist)) } },
        onPlay = { app.play(query, origin, it.locator) },
        onPlayAll = { shuffle -> app.play(query, origin, shuffle = shuffle) },
        onSelect = { app.select(it.locator) },
        onEndSelection = app::endSelection,
        onMore = { app.show(Sheet.TrackActions(it.locator)) },
        menu = { close ->
            MenuItem(stringResource(R.string.play_next), NIcons.PlayNext, { close(); queue(next = true) })
            MenuItem(stringResource(R.string.add_to_queue), NIcons.AddToQueue, { close(); queue(next = false) })
            MenuItem(stringResource(R.string.add_to_playlist), NIcons.Playlist, {
                close()
                tracks?.let { app.show(Sheet.AddToPlaylist(it.map(TrackRow::locator))) }
            })
            if (page.artist != null) {
                MenuDivider()
                MenuItem(stringResource(R.string.go_to_artist), NIcons.Artist, {
                    close()
                    app.open(Page.Artist(page.artist))
                })
            }
        },
    )
}

/** The album page itself, for [album] as it is read; [menu] fills its ⋮ menu. */
@Composable
fun AlbumContent(
    album: AlbumRow?,
    name: String?,
    tracks: List<TrackRow>?,
    nowPlaying: NowPlaying,
    selection: Selection?,
    held: Locator?,
    onBack: () -> Unit,
    onArtist: (() -> Unit)?,
    onPlay: (TrackRow) -> Unit,
    onPlayAll: (shuffle: Boolean) -> Unit,
    onSelect: (TrackRow) -> Unit,
    onEndSelection: () -> Unit,
    onMore: (TrackRow) -> Unit,
    menu: @Composable (close: () -> Unit) -> Unit,
) {
    val haptics = LocalHapticFeedback.current
    val rows = tracks.orEmpty()
    val bottom = bottomPadding(LocalBottomSpace.current)
    val margins = LocalPageMargins.current
    val tablet = LocalWindowLayout.current == WindowLayout.TABLET
    // A disc's header shows only on albums of several discs.
    val discs = rows.mapNotNull { it.discNumber }.distinct().size > 1
    var menuOpen by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize()) {
        val drift = barDrift()
        AnimatedContent(selection != null, transitionSpec = { barSwap(targetState, drift) }, label = "bar") { selecting ->
            if (selecting) {
                SelectionBar(selection?.count ?: 0, rows.size, onEndSelection, { selection?.addAll(rows.map { it.locator }) })
            } else {
                PageBar(onBack) {
                    Box {
                        NIconButton(
                            NIcons.More,
                            stringResource(R.string.more_for, name ?: stringResource(R.string.no_album)),
                            { menuOpen = true },
                            tint = colors.onSurface,
                        )
                        NMenu(menuOpen, { menuOpen = false }) { menu { menuOpen = false } }
                    }
                }
            }
        }
        BoxWithConstraints(Modifier.fillMaxSize()) {
            // A tablet sets the cover beside the album's details and lists the tracks in a table,
            // while the page is wide enough for them.
            val wide = tablet && maxWidth >= TrackTableWidth
            LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = bottom)) {
                if (wide) {
                    item(key = "header") {
                        TabletAlbumHeader(
                            album, name, onArtist, onPlayAll,
                            Modifier.padding(start = margins.start, end = margins.end, top = 4.dp),
                        )
                    }
                    item(key = "columns") {
                        AlbumTableHeader(Modifier.padding(start = margins.start, end = margins.end, top = 20.dp))
                    }
                } else {
                    item(key = "header") {
                        Column(
                            Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 24.dp),
                            horizontalAlignment = Alignment.CenterHorizontally,
                        ) {
                            val shape = RoundedCornerShape(24.dp)
                            Cover(album?.cover, Modifier.size(216.dp).floating(shape), shape = shape)
                            Text(
                                album?.let { albumName(it) } ?: name ?: stringResource(R.string.no_album),
                                style = text(26, FontWeight.ExtraBold, 32.sp, (-0.4).sp),
                                color = colors.onSurface,
                                textAlign = TextAlign.Center,
                                modifier = Modifier.padding(top = 20.dp),
                            )
                            val artist = album?.artist
                            if (artist != null && onArtist != null) {
                                Text(
                                    artist,
                                    style = text(16, FontWeight.SemiBold),
                                    color = colors.primary,
                                    textAlign = TextAlign.Center,
                                    modifier = Modifier
                                        .margins(top = 8.dp, bottom = 12.dp)
                                        .clip(RoundedCornerShape(8.dp))
                                        .tappable(onArtist, role = Role.Button)
                                        .padding(horizontal = 8.dp, vertical = 12.dp),
                                )
                            }
                            if (album != null) {
                                Text(
                                    dotted(
                                        stringResource(R.string.album_kind),
                                        album.year?.toString(),
                                        tracksCount(album.tracks),
                                        formatDuration(album.length),
                                    ),
                                    style = text(13, tabular = true),
                                    color = colors.onSurfaceVariant,
                                    modifier = Modifier.padding(top = 4.dp),
                                )
                            }
                        }
                    }
                    item(key = "play") {
                        PlayShuffle(
                            onPlay = { onPlayAll(false) },
                            onShuffle = { onPlayAll(true) },
                            modifier = Modifier.padding(start = margins.start, end = margins.end, top = 18.dp, bottom = 10.dp),
                        )
                    }
                }
                var disc: UInt? = null
                for (track in rows) {
                    if (discs && track.discNumber != disc) {
                        disc = track.discNumber
                        val number = disc
                        item(key = "disc $number") {
                            Text(
                                number?.let { stringResource(R.string.disc, it.toInt()) } ?: stringResource(R.string.disc_unknown),
                                style = text(16, FontWeight.Bold),
                                color = colors.onSurface,
                                modifier = Modifier.padding(start = margins.start, end = margins.end, top = 14.dp, bottom = 6.dp),
                            )
                        }
                    }
                    item(key = track.locator.key) {
                        val state = nowPlaying.state(track).copy(
                            selected = selection?.contains(track.locator) == true,
                            held = track.locator == held,
                        )
                        val onClick = { if (selection != null) onSelect(track) else onPlay(track) }
                        val onLongClick = {
                            if (selection == null) haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                            onSelect(track)
                        }
                        if (wide) {
                            AlbumTableRow(
                                track,
                                state,
                                onClick,
                                onLongClick,
                                onMore = { onMore(track) },
                                modifier = Modifier.padding(start = margins.start, end = margins.end),
                            )
                        } else {
                            AlbumTrackItem(track, state, onClick, onLongClick, onMore = { onMore(track) })
                        }
                    }
                }
            }
        }
    }
}

/**
 * A tablet's album header: the cover, and beside it what the page is, the album's name, its
 * artist, a line of details, then Play and Shuffle.
 */
@Composable
private fun TabletAlbumHeader(
    album: AlbumRow?,
    name: String?,
    onArtist: (() -> Unit)?,
    onPlayAll: (shuffle: Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(24.dp),
        verticalAlignment = Alignment.Bottom,
    ) {
        val shape = RoundedCornerShape(24.dp)
        Cover(album?.cover, Modifier.size(188.dp).floating(shape), shape = shape)
        Column(Modifier.weight(1f)) {
            Text(stringResource(R.string.album_kind), style = text(13, FontWeight.Bold), color = colors.onSurfaceVariant)
            Text(
                album?.let { albumName(it) } ?: name ?: stringResource(R.string.no_album),
                style = text(32, FontWeight.ExtraBold, 38.sp, (-0.6).sp),
                color = colors.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(top = 4.dp),
            )
            val artist = album?.artist
            if (artist != null && onArtist != null) {
                Text(
                    artist,
                    style = text(16, FontWeight.Bold),
                    color = colors.primary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier
                        .margins(top = 8.dp, bottom = 12.dp, start = 8.dp, end = 8.dp)
                        .clip(RoundedCornerShape(8.dp))
                        .tappable(onArtist, role = Role.Button)
                        .padding(horizontal = 8.dp, vertical = 12.dp),
                )
            }
            if (album != null) {
                Text(
                    dotted(album.year?.toString(), tracksCount(album.tracks), formatDuration(album.length)),
                    style = text(14, tabular = true),
                    color = colors.onSurfaceVariant,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }
            NamedPlayShuffle(
                onPlay = { onPlayAll(false) },
                onShuffle = { onPlayAll(true) },
                header = true,
                modifier = Modifier.padding(top = 16.dp),
            )
        }
    }
}
