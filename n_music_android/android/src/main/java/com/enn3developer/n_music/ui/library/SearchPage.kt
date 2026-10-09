package com.enn3developer.n_music.ui.library

import androidx.annotation.StringRes
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsTopHeight
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.AlbumRow
import com.enn3developer.n_music.core.ArtistRow
import com.enn3developer.n_music.core.Filter
import com.enn3developer.n_music.core.GenreRow
import com.enn3developer.n_music.core.GroupSort
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.core.SortField
import com.enn3developer.n_music.core.SortKey
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.key
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.LocalBottomSpace
import com.enn3developer.n_music.ui.Origin
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.bottomPadding
import com.enn3developer.n_music.ui.components.AlbumItem
import com.enn3developer.n_music.ui.components.ArtistItem
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.CoverPlaceholder
import com.enn3developer.n_music.ui.components.EmptyState
import com.enn3developer.n_music.ui.components.NChip
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.TextAction
import com.enn3developer.n_music.ui.components.TrackItem
import com.enn3developer.n_music.ui.components.albumName
import com.enn3developer.n_music.ui.components.artistName
import com.enn3developer.n_music.ui.components.genreName
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.sheets.Sheet
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import com.enn3developer.n_music.ui.tracksCount
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext

/** What search shows: everything it found, or one kind of it. */
enum class SearchKind(@param:StringRes val label: Int) {
    ALL(R.string.search_all),
    TRACKS(R.string.tab_tracks),
    ALBUMS(R.string.tab_albums),
    ARTISTS(R.string.tab_artists),
    GENRES(R.string.tab_genres),
}

/** What a search found. */
data class SearchResults(
    val tracks: List<TrackRow>,
    val albums: List<AlbumRow>,
    val artists: List<ArtistRow>,
    val genres: List<GenreRow>,
) {
    val empty: Boolean get() = tracks.isEmpty() && albums.isEmpty() && artists.isEmpty() && genres.isEmpty()
}

/** How many of each kind All shows before See all. */
private const val TRACKS_SHOWN = 3
private const val GENRES_SHOWN = 5

/** Tracks whose title, artists or album hold [text], by title, as search lists and plays them. */
fun searchQuery(text: String) = Query(Filter.Search(text), listOf(SortKey(SortField.Title, false)))

/** Search: a field, a chip for each kind, and what matches as one types. */
@Composable
fun SearchPage() {
    val app = LocalApp.current
    var text by rememberSaveable { mutableStateOf("") }
    var kind by rememberSaveable { mutableStateOf(SearchKind.ALL) }
    val version by CoreRepository.version.collectAsStateWithLifecycle()
    val search = text.trim()
    // Typing settles for a moment before the library is read again.
    val results by produceState<SearchResults?>(null, search, version) {
        if (search.isEmpty()) {
            value = null
            return@produceState
        }
        delay(150)
        value = withContext(Dispatchers.IO) {
            val all = Filter.All(emptyList())
            SearchResults(
                tracks = CoreRepository.tracks(searchQuery(search)),
                albums = CoreRepository.albums(all, search, GroupSort.NAME),
                artists = CoreRepository.artists(all, search, GroupSort.NAME),
                genres = CoreRepository.genres(all, search, GroupSort.NAME),
            )
        }
    }
    SearchContent(
        text = text,
        onText = { text = it },
        kind = kind,
        onKind = { kind = it },
        results = results,
        nowPlaying = rememberNowPlaying(),
        onBack = app::back,
        onTrack = { app.play(searchQuery(search), Origin.Search, it.locator) },
        onMore = { app.show(Sheet.TrackActions(it.locator)) },
        onAlbum = { app.open(Page.Album(it.name, it.artist)) },
        onArtist = { app.open(Page.Artist(it.name)) },
        onGenre = { genre -> app.showTracks(TrackFilters(genres = listOf(genre.name.orEmpty()))) },
        focus = true,
    )
}

/** The search page itself, showing [results] for [text]. */
@Composable
fun SearchContent(
    text: String,
    onText: (String) -> Unit,
    kind: SearchKind,
    onKind: (SearchKind) -> Unit,
    results: SearchResults?,
    nowPlaying: NowPlaying,
    onBack: () -> Unit,
    onTrack: (TrackRow) -> Unit,
    onMore: (TrackRow) -> Unit,
    onAlbum: (AlbumRow) -> Unit,
    onArtist: (ArtistRow) -> Unit,
    onGenre: (GenreRow) -> Unit,
    focus: Boolean = false,
) {
    val bottom = bottomPadding(LocalBottomSpace.current)
    val search = text.trim()
    Column(Modifier.fillMaxSize()) {
        SearchBar(text, onText, onBack, focus)
        Row(
            Modifier
                .horizontalScroll(rememberScrollState())
                .padding(start = 16.dp, end = 16.dp, top = 12.dp, bottom = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            for (option in SearchKind.entries) {
                NChip(stringResource(option.label), { onKind(option) }, selected = option == kind, role = Role.RadioButton)
            }
        }
        when {
            results == null -> Spacer(Modifier.weight(1f))
            results.empty -> EmptyState(
                NIcons.Search,
                stringResource(R.string.search_nothing),
                stringResource(R.string.search_nothing_hint),
                bottom = 40.dp,
            )
            else -> LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = bottom)) {
                val all = kind == SearchKind.ALL
                if ((all || kind == SearchKind.TRACKS) && results.tracks.isNotEmpty()) {
                    val shown = if (all) results.tracks.take(TRACKS_SHOWN) else results.tracks
                    item(key = "tracks") {
                        SectionHeader(
                            R.string.tab_tracks,
                            more = results.tracks.size.takeIf { all && it > TRACKS_SHOWN },
                            onMore = { onKind(SearchKind.TRACKS) },
                        )
                    }
                    items(shown, key = { "track " + it.locator.key }) { track ->
                        TrackItem(
                            track,
                            nowPlaying.state(track),
                            onClick = { onTrack(track) },
                            onLongClick = null,
                            onMore = { onMore(track) },
                            title = highlight(track.title, search),
                        )
                    }
                }
                if ((all || kind == SearchKind.ALBUMS) && results.albums.isNotEmpty()) {
                    item(key = "albums") { SectionHeader(R.string.tab_albums, top = 18.dp, bottom = 10.dp) }
                    if (all) {
                        item(key = "album row") {
                            LazyRow(
                                contentPadding = PaddingValues(horizontal = 16.dp),
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                items(results.albums, key = { "${it.name}\u0000${it.artist}" }) { album ->
                                    FoundAlbum(album, search) { onAlbum(album) }
                                }
                            }
                        }
                    } else {
                        items(results.albums, key = { "album ${it.name}\u0000${it.artist}" }) { album ->
                            AlbumItem(album, nowPlaying.album(album), nowPlaying.playing, { onAlbum(album) })
                        }
                    }
                }
                if ((all || kind == SearchKind.ARTISTS) && results.artists.isNotEmpty()) {
                    item(key = "artists") { SectionHeader(R.string.tab_artists, top = 20.dp, bottom = 10.dp) }
                    if (all) {
                        item(key = "artist row") {
                            LazyRow(
                                contentPadding = PaddingValues(horizontal = 16.dp),
                                horizontalArrangement = Arrangement.spacedBy(12.dp),
                            ) {
                                items(results.artists, key = { it.name.toString() }) { artist ->
                                    FoundArtist(artist, search) { onArtist(artist) }
                                }
                            }
                        }
                    } else {
                        items(results.artists, key = { "artist " + it.name }) { artist ->
                            ArtistItem(artist, nowPlaying.artist(artist), nowPlaying.playing, { onArtist(artist) })
                        }
                    }
                }
                if ((all || kind == SearchKind.GENRES) && results.genres.isNotEmpty()) {
                    val shown = if (all) results.genres.take(GENRES_SHOWN) else results.genres
                    item(key = "genres") {
                        SectionHeader(
                            R.string.tab_genres,
                            top = 20.dp,
                            bottom = 6.dp,
                            more = results.genres.size.takeIf { all && it > GENRES_SHOWN },
                            onMore = { onKind(SearchKind.GENRES) },
                        )
                    }
                    items(shown, key = { "genre " + it.name }) { genre ->
                        FoundGenre(genre, search) { onGenre(genre) }
                    }
                }
            }
        }
    }
}

/** The field along the top, tinted up under the status bar: Close, what is typed, Clear. */
@Composable
private fun SearchBar(text: String, onText: (String) -> Unit, onBack: () -> Unit, focus: Boolean) {
    val keyboard = LocalSoftwareKeyboardController.current
    val requester = remember { FocusRequester() }
    if (focus) {
        LaunchedEffect(Unit) { requester.requestFocus() }
    }
    Column(
        Modifier
            .fillMaxWidth()
            .background(colors.surfaceHigh)
    ) {
        Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars))
        Row(
            Modifier
                .fillMaxWidth()
                .height(64.dp)
                .padding(horizontal = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            NIconButton(NIcons.Back, stringResource(R.string.close_search), onBack, tint = colors.onSurface)
            val hint = stringResource(R.string.search_library)
            Box(Modifier.weight(1f), contentAlignment = Alignment.CenterStart) {
                if (text.isEmpty()) Text(hint, style = text(18), color = colors.onSurfaceVariant, maxLines = 1)
                BasicTextField(
                    text,
                    onText,
                    singleLine = true,
                    textStyle = text(18).copy(color = colors.onSurface),
                    cursorBrush = SolidColor(colors.primary),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                    keyboardActions = KeyboardActions(onSearch = { keyboard?.hide() }),
                    modifier = Modifier
                        .fillMaxWidth()
                        .focusRequester(requester)
                        .semantics { contentDescription = hint },
                )
            }
            if (text.isNotEmpty()) {
                NIconButton(NIcons.Close, stringResource(R.string.clear_search), { onText("") }, tint = colors.onSurfaceVariant)
            }
        }
    }
}

/** A kind's heading, with See all when All shows only some of the [more] there are. */
@Composable
private fun SectionHeader(
    @StringRes title: Int,
    top: Dp = 16.dp,
    bottom: Dp = 4.dp,
    more: Int? = null,
    onMore: () -> Unit = {},
) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(start = 16.dp, end = 8.dp, top = top, bottom = bottom)
            .then(if (more != null) Modifier.height(40.dp) else Modifier),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(stringResource(title), style = text(16, FontWeight.Bold), color = colors.onSurface, modifier = Modifier.weight(1f))
        if (more != null) TextAction(stringResource(R.string.see_all, formatCount(more)), onMore)
    }
}

@Composable
private fun FoundAlbum(album: AlbumRow, search: String, onClick: () -> Unit) {
    Column(Modifier.width(140.dp).tappable(onClick)) {
        Cover(album.cover, Modifier.size(140.dp), shape = RoundedCornerShape(16.dp))
        Text(
            highlight(albumName(album), search),
            style = text(14, FontWeight.SemiBold),
            color = colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.padding(top = 8.dp),
        )
        Text(
            album.artist ?: stringResource(R.string.unknown_artist),
            style = text(13),
            color = colors.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

@Composable
private fun FoundArtist(artist: ArtistRow, search: String, onClick: () -> Unit) {
    Column(Modifier.width(104.dp).tappable(onClick), horizontalAlignment = Alignment.CenterHorizontally) {
        Cover(artist.cover, Modifier.size(104.dp), shape = CircleShape, placeholder = CoverPlaceholder.ARTIST)
        Text(
            highlight(artistName(artist), search),
            style = text(14, FontWeight.SemiBold),
            color = colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = 8.dp),
        )
        Text(tracksCount(artist.tracks), style = text(12, tabular = true), color = colors.onSurfaceVariant)
    }
}

@Composable
private fun FoundGenre(genre: GenreRow, search: String, onClick: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .height(56.dp)
            .tappable(onClick)
            .padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Box(
            Modifier
                .size(40.dp)
                .background(colors.surfaceHigh, RoundedCornerShape(12.dp)),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(NIcons.Genre, size = 20.dp, tint = colors.onSurfaceVariant)
        }
        Text(
            highlight(genreName(genre), search),
            style = text(16, FontWeight.Medium),
            color = colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        Text(tracksCount(genre.tracks), style = text(13, tabular = true), color = colors.onSurfaceVariant)
    }
}

/** [value] with where [search] first shows in it, ignoring case, in the primary colour and heavier. */
@Composable
fun highlight(value: String, search: String): AnnotatedString {
    val color = colors.primary
    return remember(value, search, color) { highlighted(value, search, color) }
}

private fun highlighted(value: String, search: String, color: Color): AnnotatedString {
    val start = if (search.isEmpty()) -1 else value.indexOf(search, ignoreCase = true)
    if (start < 0) return AnnotatedString(value)
    return buildAnnotatedString {
        append(value, 0, start)
        withStyle(SpanStyle(color = color, fontWeight = FontWeight.ExtraBold)) {
            append(value, start, start + search.length)
        }
        append(value, start + search.length, value.length)
    }
}
