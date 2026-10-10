package com.enn3developer.n_music.ui.library

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.core.AlbumRow
import com.enn3developer.n_music.core.ArtistRow
import com.enn3developer.n_music.core.Filter
import com.enn3developer.n_music.core.GroupSort
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.core.Tag
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.key
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.LocalBottomSpace
import com.enn3developer.n_music.ui.LocalPageMargins
import com.enn3developer.n_music.ui.Origin
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.Selection
import com.enn3developer.n_music.ui.Snack
import com.enn3developer.n_music.ui.bottomPadding
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.CoverPlaceholder
import com.enn3developer.n_music.ui.components.MenuItem
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.NMenu
import com.enn3developer.n_music.ui.components.PageBar
import com.enn3developer.n_music.ui.components.PlayShuffle
import com.enn3developer.n_music.ui.components.SelectionBar
import com.enn3developer.n_music.ui.components.SmallAlbumTile
import com.enn3developer.n_music.ui.components.SortControl
import com.enn3developer.n_music.ui.components.TrackItem
import com.enn3developer.n_music.ui.components.artistName
import com.enn3developer.n_music.ui.components.barDrift
import com.enn3developer.n_music.ui.components.barSwap
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.enqueue
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.formatDuration
import com.enn3developer.n_music.ui.playsCount
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.rememberLibrary
import com.enn3developer.n_music.ui.sheets.Sheet
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import com.enn3developer.n_music.ui.tracksCount

/** An artist's tracks: those with them among their artists, or without any for no name. */
fun artistFilter(name: String?): Filter = name?.let(Filter::Artist) ?: Filter.Untagged(Tag.ARTIST)

/**
 * An artist's page: their picture and details, Play and Shuffle, the albums they are on, newest
 * first, and their tracks, the most played first unless sorted otherwise.
 */
@Composable
fun ArtistPage(page: Page.Artist) {
    val app = LocalApp.current
    val resources = LocalResources.current
    val ui by UiPreferences.settings.collectAsStateWithLifecycle()
    val filter = remember(page) { artistFilter(page.name) }
    val artist = rememberLibrary<ArtistRow?>(null, filter) {
        CoreRepository.artists(filter, "", GroupSort.NAME).firstOrNull { it.name == page.name }
    }
    val albums = rememberLibrary<List<AlbumRow>>(emptyList(), filter) { CoreRepository.albums(filter, "", GroupSort.NEWEST) }
    val order = SortedList.ARTIST_TRACKS.trackOrder(ui.sorts)
    val query = Query(filter, order.keys())
    val tracks = rememberLibrary<List<TrackRow>?>(null, query) { CoreRepository.tracks(query) }
    val origin = Origin.Artist(page.name)
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

    ArtistContent(
        artist = artist,
        name = page.name,
        albums = albums,
        tracks = tracks,
        order = order,
        nowPlaying = rememberNowPlaying(),
        selection = selection,
        held = (app.sheet as? Sheet.TrackActions)?.track,
        onBack = app::back,
        onAlbum = { app.open(Page.Album(it.name, it.artist)) },
        onPlay = { app.play(query, origin, it.locator) },
        onPlayAll = { shuffle -> app.play(query, origin, shuffle = shuffle) },
        onSort = { app.show(Sheet.Sort(SortedList.ARTIST_TRACKS)) },
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
        },
    )
}

/** The artist page itself, for [artist] as it is read; [menu] fills its ⋮ menu. */
@Composable
fun ArtistContent(
    artist: ArtistRow?,
    name: String?,
    albums: List<AlbumRow>,
    tracks: List<TrackRow>?,
    order: TrackOrder,
    nowPlaying: NowPlaying,
    selection: Selection?,
    held: Locator?,
    onBack: () -> Unit,
    onAlbum: (AlbumRow) -> Unit,
    onPlay: (TrackRow) -> Unit,
    onPlayAll: (shuffle: Boolean) -> Unit,
    onSort: () -> Unit,
    onSelect: (TrackRow) -> Unit,
    onEndSelection: () -> Unit,
    onMore: (TrackRow) -> Unit,
    menu: @Composable (close: () -> Unit) -> Unit,
) {
    val haptics = LocalHapticFeedback.current
    val rows = tracks.orEmpty()
    val bottom = bottomPadding(LocalBottomSpace.current)
    val margins = LocalPageMargins.current
    val title = artist?.let { artistName(it) } ?: name ?: stringResource(R.string.no_artist)
    var menuOpen by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize()) {
        val drift = barDrift()
        AnimatedContent(selection != null, transitionSpec = { barSwap(targetState, drift) }, label = "bar") { selecting ->
            if (selecting) {
                SelectionBar(selection?.count ?: 0, rows.size, onEndSelection, { selection?.addAll(rows.map { it.locator }) })
            } else {
                PageBar(onBack) {
                    Box {
                        NIconButton(NIcons.More, stringResource(R.string.more_for, title), { menuOpen = true }, tint = colors.onSurface)
                        NMenu(menuOpen, { menuOpen = false }) { menu { menuOpen = false } }
                    }
                }
            }
        }
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = bottom)) {
            item(key = "header") {
                Column(
                    Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Cover(artist?.cover, Modifier.size(112.dp), shape = CircleShape, placeholder = CoverPlaceholder.ARTIST)
                    Text(
                        title,
                        style = text(28, FontWeight.ExtraBold, 34.sp, (-0.5).sp),
                        color = colors.onSurface,
                        textAlign = TextAlign.Center,
                        modifier = Modifier.padding(top = 14.dp),
                    )
                    if (artist != null) {
                        Text(
                            dotted(stringResource(R.string.artist_kind), tracksCount(artist.tracks), formatDuration(artist.length)),
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
                    modifier = Modifier.padding(start = margins.start, end = margins.end, top = 18.dp),
                )
            }
            if (albums.isNotEmpty()) {
                item(key = "albums") {
                    Column(Modifier.padding(top = 22.dp)) {
                        Row(
                            Modifier.padding(start = margins.start, end = margins.end, bottom = 10.dp),
                            verticalAlignment = Alignment.Bottom,
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Text(stringResource(R.string.tab_albums), style = text(16, FontWeight.Bold), color = colors.onSurface)
                            Text(
                                formatCount(albums.size),
                                style = text(13, tabular = true),
                                color = colors.onSurfaceVariant,
                                modifier = Modifier.padding(bottom = 1.dp),
                            )
                        }
                        LazyRow(
                            contentPadding = PaddingValues(start = margins.start, end = margins.end),
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            items(albums, key = { "${it.name}\u0000${it.artist}" }) { album ->
                                SmallAlbumTile(album, nowPlaying.album(album), nowPlaying.playing, { onAlbum(album) })
                            }
                        }
                    }
                }
            }
            item(key = "tracks") {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .padding(start = margins.start, end = margins.endLess(8.dp), top = 14.dp, bottom = 2.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        stringResource(R.string.tab_tracks),
                        style = text(16, FontWeight.Bold),
                        color = colors.onSurface,
                        modifier = Modifier.weight(1f),
                    )
                    SortControl(stringResource(order.sort.label), onSort)
                }
            }
            items(rows, key = { it.locator.key }) { track ->
                TrackItem(
                    track,
                    nowPlaying.state(track).copy(selected = selection?.contains(track.locator) == true, held = track.locator == held),
                    onClick = { if (selection != null) onSelect(track) else onPlay(track) },
                    onLongClick = {
                        if (selection == null) haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                        onSelect(track)
                    },
                    onMore = { onMore(track) },
                    line = dotted(track.album, playsCount(track.plays)),
                )
            }
        }
    }
}
