package com.enn3developer.n_music.ui.library

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.ContentTransform
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.PagerDefaults
import androidx.compose.foundation.pager.PagerState
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.layout.positionInParent
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.LibraryTab
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ScanState
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.ViewMode
import com.enn3developer.n_music.core.AlbumRow
import com.enn3developer.n_music.core.ArtistRow
import com.enn3developer.n_music.core.Filter
import com.enn3developer.n_music.core.GenreRow
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.core.defaultSourceName
import com.enn3developer.n_music.key
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.LocalBottomSpace
import com.enn3developer.n_music.ui.Origin
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.Selection
import com.enn3developer.n_music.ui.bottomPadding
import com.enn3developer.n_music.ui.components.AlbumItem
import com.enn3developer.n_music.ui.components.AlbumTile
import com.enn3developer.n_music.ui.components.ArtistItem
import com.enn3developer.n_music.ui.components.ArtistTile
import com.enn3developer.n_music.ui.components.FastScroller
import com.enn3developer.n_music.ui.components.GenreItem
import com.enn3developer.n_music.ui.components.GenreTile
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.PlayShuffle
import com.enn3developer.n_music.ui.components.SelectionBar
import com.enn3developer.n_music.ui.components.SortControl
import com.enn3developer.n_music.ui.components.TrackItem
import com.enn3developer.n_music.ui.components.TrackState
import com.enn3developer.n_music.ui.components.TrackTile
import com.enn3developer.n_music.ui.components.ViewSwitch
import com.enn3developer.n_music.ui.components.WavyProgress
import com.enn3developer.n_music.ui.components.inert
import com.enn3developer.n_music.ui.components.rememberScrolled
import com.enn3developer.n_music.ui.components.sectionLetter
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.rememberLibrary
import com.enn3developer.n_music.ui.sheets.Sheet
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.delayed
import com.enn3developer.n_music.ui.theme.text
import kotlin.math.abs
import kotlinx.coroutines.launch

/** What plays now, for lists to mark. */
data class NowPlaying(val track: TrackRow?, val playing: Boolean) {
    fun state(row: TrackRow) = TrackState(current = row.locator == track?.locator, playing = playing)

    /** One of the album's tracks plays. */
    fun album(album: AlbumRow): Boolean {
        val track = track ?: return false
        return album.name != null &&
            album.name.equals(track.album, ignoreCase = true) &&
            album.artist.equals(track.albumArtist, ignoreCase = true)
    }

    fun artist(artist: ArtistRow): Boolean =
        artist.name != null && track?.artists?.any { it.equals(artist.name, ignoreCase = true) } == true
}

@Composable
fun rememberNowPlaying(): NowPlaying {
    val current by CoreRepository.current.collectAsStateWithLifecycle()
    val playing by CoreRepository.playing.collectAsStateWithLifecycle()
    return NowPlaying(current?.track, playing)
}

/** The library: search, its four tabs, and the scan's progress while it is first built. */
@Composable
fun LibraryPage() {
    val app = LocalApp.current
    val ui by UiPreferences.settings.collectAsStateWithLifecycle()
    val pager = rememberPagerState { LibraryTab.entries.size }
    val scan by CoreRepository.scanState.collectAsStateWithLifecycle()
    val library by CoreRepository.library.collectAsStateWithLifecycle()
    val building by CoreRepository.building.collectAsStateWithLifecycle()
    val nowPlaying = rememberNowPlaying()
    val sources by CoreRepository.sources.collectAsStateWithLifecycle()
    val sourceName: (Locator) -> String = { root -> sources.find { it.root == root }?.name ?: defaultSourceName(root) }
    val scope = rememberCoroutineScope()
    // A genre shows as the tracks filtered to it.
    fun showGenre(genre: GenreRow) {
        app.filters = TrackFilters(genres = listOf(genre.name.orEmpty()))
        scope.launch { pager.animateScrollToPage(LibraryTab.TRACKS.ordinal, animationSpec = NMotion.noBounce()) }
    }
    val order = TrackOrder.parse(ui.sorts["tracks"], TrackOrder(TrackSort.ARTIST_ALBUM))
    val filters = app.filters
    val query = Query(filters.filter(), order.keys())
    val tracks = rememberLibrary<List<TrackRow>?>(null, query) { CoreRepository.tracks(query) }
    val selection = app.selection
    BackHandler(selection != null) { app.endSelection() }
    // Selecting belongs to the Tracks tab.
    LaunchedEffect(pager.currentPage) { app.endSelection() }
    Column(Modifier.fillMaxSize()) {
        AnimatedContent(
            selection != null,
            transitionSpec = { barSwap(targetState) },
            label = "header",
        ) { selecting ->
            if (selecting) {
                SelectionBar(
                    count = selection?.count ?: 0,
                    total = tracks.orEmpty().size,
                    onClose = app::endSelection,
                    onSelectAll = { selection?.addAll(tracks.orEmpty().map { it.locator }) },
                )
            } else {
                SearchHeader(onSearch = { app.open(Page.Search) }, onSettings = { app.open(Page.Settings) })
            }
        }
        LibraryTabs(pager)
        HorizontalPager(
            state = pager,
            modifier = Modifier.weight(1f),
            key = { it },
            userScrollEnabled = selection == null,
            flingBehavior = PagerDefaults.flingBehavior(pager, snapAnimationSpec = NMotion.noBounce()),
        ) { page ->
            when (LibraryTab.entries[page]) {
                LibraryTab.TRACKS -> {
                    TracksTab(
                        tracks = tracks,
                        total = if (filters.active.isEmpty()) library.tracks.toLong() else tracks.orEmpty().size.toLong(),
                        filters = filters,
                        sourceName = sourceName,
                        onFilter = { app.show(Sheet.Filters(it)) },
                        onClearFilter = { app.filters = app.filters.clear(it) },
                        view = ui.view(LibraryTab.TRACKS),
                        order = order,
                        nowPlaying = nowPlaying,
                        building = scan?.takeIf { building },
                        compact = ui.compactRows,
                        onToggleView = { UiPreferences.setView(LibraryTab.TRACKS, it) },
                        selection = selection,
                        onSelect = { track -> app.select(track.locator) },
                        onPlay = { track -> app.play(query, Origin.Library, track.locator) },
                        onPlayAll = { shuffle -> app.play(query, Origin.Library, shuffle = shuffle) },
                        onMore = {},
                        onSort = { app.show(Sheet.Sort(LibraryTab.TRACKS)) },
                        onScan = { app.open(Page.Sources) },
                    )
                }

                LibraryTab.ALBUMS -> {
                    val order = GroupOrder.parse(ui.sorts["albums"], GroupOrder.ARTIST)
                    val albums = rememberLibrary<List<AlbumRow>?>(null, order) {
                        CoreRepository.albums(Filter.All(emptyList()), "", order.sort)
                    }
                    GroupTab(
                        items = albums,
                        count = { pluralStringResource(R.plurals.albums_count, quantity(it), formatCount(it)) },
                        view = ui.view(LibraryTab.ALBUMS),
                        sortLabel = stringResource(order.label),
                        minTile = 160.dp,
                        rowGap = 18.dp,
                        onToggleView = { UiPreferences.setView(LibraryTab.ALBUMS, it) },
                        onSort = { app.show(Sheet.Sort(LibraryTab.ALBUMS)) },
                        key = { "${it.name}\u0000${it.artist}" },
                        section = { album ->
                            when (order) {
                                GroupOrder.ARTIST -> sectionLetter(album.artist)
                                GroupOrder.NAME -> sectionLetter(album.name)
                                GroupOrder.NEWEST, GroupOrder.OLDEST -> album.year?.toString() ?: "#"
                                GroupOrder.MOST_TRACKS -> null
                            }
                        },
                        item = { album -> AlbumItem(album, nowPlaying.album(album), nowPlaying.playing, { app.open(Page.Album(album.name, album.artist)) }) },
                        tile = { album -> AlbumTile(album, nowPlaying.album(album), nowPlaying.playing, { app.open(Page.Album(album.name, album.artist)) }) },
                    )
                }

                LibraryTab.ARTISTS -> {
                    val order = GroupOrder.parse(ui.sorts["artists"], GroupOrder.NAME)
                    val artists = rememberLibrary<List<ArtistRow>?>(null, order) {
                        CoreRepository.artists(Filter.All(emptyList()), "", order.sort)
                    }
                    GroupTab(
                        items = artists,
                        count = { pluralStringResource(R.plurals.artists_count, quantity(it), formatCount(it)) },
                        view = ui.view(LibraryTab.ARTISTS),
                        sortLabel = stringResource(order.label),
                        minTile = 104.dp,
                        rowGap = 16.dp,
                        onToggleView = { UiPreferences.setView(LibraryTab.ARTISTS, it) },
                        onSort = { app.show(Sheet.Sort(LibraryTab.ARTISTS)) },
                        key = { it.name.toString() },
                        section = { artist -> if (order == GroupOrder.NAME) sectionLetter(artist.name) else null },
                        item = { artist -> ArtistItem(artist, nowPlaying.artist(artist), nowPlaying.playing, { app.open(Page.Artist(artist.name)) }) },
                        tile = { artist -> ArtistTile(artist, nowPlaying.artist(artist), nowPlaying.playing, { app.open(Page.Artist(artist.name)) }) },
                    )
                }

                LibraryTab.GENRES -> {
                    val order = GroupOrder.parse(ui.sorts["genres"], GroupOrder.NAME)
                    val genres = rememberLibrary<List<GenreRow>?>(null, order) {
                        CoreRepository.genres(Filter.All(emptyList()), "", order.sort)
                    }
                    GroupTab(
                        items = genres,
                        count = { pluralStringResource(R.plurals.genres_count, quantity(it), formatCount(it)) },
                        view = ui.view(LibraryTab.GENRES),
                        sortLabel = stringResource(order.label),
                        minTile = 160.dp,
                        rowGap = 18.dp,
                        onToggleView = { UiPreferences.setView(LibraryTab.GENRES, it) },
                        onSort = { app.show(Sheet.Sort(LibraryTab.GENRES)) },
                        key = { it.name.toString() },
                        section = { genre -> if (order == GroupOrder.NAME) sectionLetter(genre.name) else null },
                        item = { genre -> GenreItem(genre, { showGenre(genre) }) },
                        tile = { genre -> GenreTile(genre, { showGenre(genre) }) },
                    )
                }
            }
        }
    }
}

/**
 * The search field giving way to the selection bar, or back: the old one leaves first, the new
 * one follows 90 ms later, and both drift 8 dp the way the change goes.
 */
private fun AnimatedContentTransitionScope<Boolean>.barSwap(selecting: Boolean): ContentTransform {
    val drift = if (selecting) 1 else -1
    val lead = NMotion.STAGGER_MS
    return (
        slideInVertically(NMotion.spatialDefault<IntOffset>().delayed(lead + 90)) { -drift * it / 9 } +
            fadeIn(NMotion.effectsDefault<Float>().delayed(lead + 90))
        ).togetherWith(
        slideOutVertically(NMotion.spatialDefault<IntOffset>().delayed(lead)) { drift * it / 9 } +
            fadeOut(NMotion.effectsFast<Float>().delayed(lead))
    ) using SizeTransform(clip = false)
}

/** The library's search field, which opens Search, with Settings at its end. */
@Composable
fun SearchHeader(onSearch: () -> Unit, onSettings: () -> Unit, modifier: Modifier = Modifier) {
    Box(
        modifier
            .windowInsetsPadding(WindowInsets.statusBars)
            .padding(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 8.dp)
    ) {
        Row(
            Modifier
                .fillMaxWidth()
                .height(56.dp)
                .clip(RoundedCornerShape(28.dp))
                .background(colors.surfaceHigh)
                .padding(end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            val hint = stringResource(R.string.search_library)
            Row(
                Modifier
                    .weight(1f)
                    .height(56.dp)
                    .clickable(role = Role.Button, onClick = onSearch)
                    .semantics { contentDescription = hint }
                    .padding(start = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                NIcon(NIcons.Search, tint = colors.onSurfaceVariant)
                Text(hint, style = text(16), color = colors.onSurfaceVariant, maxLines = 1)
            }
            NIconButton(NIcons.Settings, stringResource(R.string.settings), onSettings, tint = colors.onSurfaceVariant)
        }
    }
}

/**
 * The library's tabs. The indicator and the labels follow the pages as they move, so they
 * never drift apart. They read the pages' position while drawing: a swipe redraws the row
 * without composing it again.
 */
@Composable
fun LibraryTabs(pager: PagerState, modifier: Modifier = Modifier, tabWidth: Dp? = null) {
    val scope = rememberCoroutineScope()
    // Each label's left edge and width in the row, to place the indicator under them.
    val labels = remember { mutableStateMapOf<Int, Pair<Float, Float>>() }
    val titles = listOf(R.string.tab_tracks, R.string.tab_albums, R.string.tab_artists, R.string.tab_genres)
    val primary = colors.primary
    val quiet = colors.onSurfaceVariant
    val line = colors.outlineVariant
    // 48 dp of tabs over a 1 dp line, as the design draws its border under the row.
    Row(
        modifier
            .fillMaxWidth()
            .height(49.dp)
            .drawBehind {
                val bottom = size.height - 1.dp.toPx()
                drawRect(line, Offset(0f, bottom), Size(size.width, 1.dp.toPx()))
                // The indicator sits on the line under the label, 8 dp wider than it, and
                // slides with the pages.
                val position = pager.position
                val from = labels[position.toInt().coerceIn(0, 3)] ?: return@drawBehind
                val to = labels[(position.toInt() + 1).coerceIn(0, 3)] ?: from
                val fraction = position - position.toInt()
                val extra = 4.dp.toPx()
                val height = 3.dp.toPx()
                val left = from.first + (to.first - from.first) * fraction - extra
                val width = from.second + (to.second - from.second) * fraction + 2 * extra
                val corner = CornerRadius(height)
                drawPath(
                    Path().apply {
                        addRoundRect(
                            RoundRect(
                                Rect(Offset(left, bottom - height), Size(width, height)),
                                topLeft = corner,
                                topRight = corner,
                            )
                        )
                    },
                    primary,
                )
            }
            .padding(bottom = 1.dp)
    ) {
        titles.forEachIndexed { index, title ->
            val near by remember(pager, index) { derivedStateOf { pager.distanceTo(index) < 0.5f } }
            Box(
                (if (tabWidth != null) Modifier.width(tabWidth) else Modifier.weight(1f))
                    .fillMaxHeight()
                    .selectable(
                        selected = pager.currentPage == index,
                        role = Role.Tab,
                        onClick = { scope.launch { pager.animateScrollToPage(index, animationSpec = NMotion.noBounce()) } },
                    ),
                contentAlignment = Alignment.Center,
            ) {
                BasicText(
                    stringResource(title),
                    style = text(14, if (near) FontWeight.Bold else FontWeight.SemiBold),
                    color = { lerp(primary, quiet, pager.distanceTo(index)) },
                    modifier = Modifier.onPlaced { coordinates ->
                        val parent = coordinates.parentLayoutCoordinates
                        val left = (parent?.positionInParent()?.x ?: 0f) + coordinates.positionInParent().x
                        labels[index] = left to coordinates.size.width.toFloat()
                    },
                )
            }
        }
    }
}

/** Where the pages are: 1.5 halfway from the second to the third. */
private val PagerState.position: Float get() = currentPage + currentPageOffsetFraction

/** How far the pages are from page [index], up to 1. */
private fun PagerState.distanceTo(index: Int): Float = abs(position - index).coerceIn(0f, 1f)

/** The tracks: their sort, view and Play and Shuffle, then the list or the grid. */
@Composable
fun TracksTab(
    tracks: List<TrackRow>?,
    total: Long,
    filters: TrackFilters,
    sourceName: (Locator) -> String,
    onFilter: (FilterField?) -> Unit,
    onClearFilter: (FilterField) -> Unit,
    view: ViewMode,
    order: TrackOrder,
    nowPlaying: NowPlaying,
    building: ScanState?,
    compact: Boolean,
    onToggleView: (ViewMode) -> Unit,
    selection: Selection?,
    onSelect: (TrackRow) -> Unit,
    onPlay: (TrackRow) -> Unit,
    onPlayAll: (shuffle: Boolean) -> Unit,
    onMore: (TrackRow) -> Unit,
    onSort: () -> Unit,
    onScan: () -> Unit,
) {
    val bottom = bottomPadding(LocalBottomSpace.current)
    val haptics = LocalHapticFeedback.current
    val selecting = selection != null
    // While selecting, the filters and what plays dim and stop answering.
    val dim by animateFloatAsState(if (selecting) 0.38f else 1f, NMotion.effectsDefault(), label = "dim")
    fun press(track: TrackRow) = if (selecting) onSelect(track) else onPlay(track)
    fun hold(track: TrackRow) {
        if (!selecting) haptics.performHapticFeedback(HapticFeedbackType.LongPress)
        onSelect(track)
    }
    fun state(track: TrackRow) = nowPlaying.state(track).copy(selected = selection?.contains(track.locator) == true)
    Column(Modifier.fillMaxSize()) {
        AnimatedVisibility(
            building != null,
            enter = expandVertically(NMotion.spatialDefault()) + fadeIn(NMotion.effectsDefault()),
            exit = shrinkVertically(NMotion.spatialDefault()) + fadeOut(NMotion.effectsFast()),
        ) {
            building?.let { BuildingCard(it, onScan, Modifier.padding(start = 16.dp, end = 16.dp, top = 12.dp)) }
        }
        Column(
            Modifier
                .graphicsLayer { alpha = dim }
                .then(if (selecting) Modifier.inert() else Modifier)
        ) {
            FilterRow(filters, sourceName, onFilter, onClearFilter, Modifier.padding(top = 12.dp))
            Row(
                Modifier.padding(start = 6.dp, end = 16.dp, top = 6.dp, bottom = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                // The sort's name gives way first when the row runs short.
                Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically) {
                    SortControl(stringResource(order.sort.label), onSort, Modifier.weight(1f, fill = false))
                    ViewSwitch(
                        view,
                        { onToggleView(if (view == ViewMode.LIST) ViewMode.GRID else ViewMode.LIST) },
                        Modifier.padding(start = 6.dp, end = 2.dp),
                    )
                }
                val count = formatCount(total)
                val filtered = filters.active.isNotEmpty()
                PlayShuffle(
                    onPlay = { onPlayAll(false) },
                    onShuffle = { onPlayAll(true) },
                    large = false,
                    playLabel = if (filtered) stringResource(R.string.play_count, count) else stringResource(R.string.play),
                    playDescription = stringResource(if (filtered) R.string.play_matching else R.string.play_all, count),
                    shuffleDescription = stringResource(if (filtered) R.string.shuffle_matching else R.string.shuffle_all, count),
                )
            }
        }
        val rows = tracks.orEmpty()
        val section = { index: Int -> rows.getOrNull(index)?.let { trackSection(it, order.sort) } }
        Box(Modifier.fillMaxSize()) {
            if (view == ViewMode.LIST) {
                val list = rememberLazyListState()
                LazyColumn(Modifier.fillMaxSize(), list, contentPadding = PaddingValues(bottom = bottom)) {
                    items(rows, key = { it.locator.key }) { track ->
                        TrackItem(
                            track,
                            state(track),
                            onClick = { press(track) },
                            onLongClick = { hold(track) },
                            onMore = { onMore(track) },
                            compact = compact,
                        )
                    }
                }
                FastScroller(rememberScrolled(list), section, bottom = bottom + 16.dp)
            } else {
                val grid = rememberLazyGridState()
                LazyVerticalGrid(
                    columns = GridCells.Adaptive(104.dp),
                    modifier = Modifier.fillMaxSize(),
                    state = grid,
                    contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 8.dp, bottom = bottom),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalArrangement = Arrangement.spacedBy(14.dp),
                ) {
                    items(rows, key = { it.locator.key }) { track ->
                        TrackTile(
                            track,
                            state(track),
                            onClick = { press(track) },
                            onLongClick = { hold(track) },
                            onMore = { onMore(track) },
                        )
                    }
                }
                FastScroller(rememberScrolled(grid), section, bottom = bottom + 16.dp)
            }
        }
    }
}

/** What a track files under in a list sorted by [sort]: a letter, a year; nothing for the rest. */
fun trackSection(track: TrackRow, sort: TrackSort): String? = when (sort) {
    TrackSort.ARTIST_ALBUM -> sectionLetter(track.artist)
    TrackSort.TITLE -> sectionLetter(track.title)
    TrackSort.ALBUM -> sectionLetter(track.album)
    TrackSort.YEAR -> track.year?.toString() ?: "#"
    else -> null
}

/**
 * Albums, artists or genres: their sort, view and count, then the list or the grid of
 * [minTile] wide tiles, so wider windows get more columns.
 */
@Composable
fun <T> GroupTab(
    items: List<T>?,
    count: @Composable (Int) -> String,
    view: ViewMode,
    sortLabel: String,
    minTile: Dp,
    rowGap: Dp,
    onToggleView: (ViewMode) -> Unit,
    onSort: () -> Unit,
    key: (T) -> Any,
    section: (T) -> String?,
    item: @Composable (T) -> Unit,
    tile: @Composable (T) -> Unit,
) {
    val bottom = bottomPadding(LocalBottomSpace.current)
    val rows = items.orEmpty()
    val sectionAt = { index: Int -> rows.getOrNull(index)?.let(section) }
    Column(Modifier.fillMaxSize()) {
        Row(
            Modifier.padding(start = 6.dp, end = 16.dp, top = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically) {
                SortControl(sortLabel, onSort, Modifier.weight(1f, fill = false))
                ViewSwitch(
                    view,
                    { onToggleView(if (view == ViewMode.LIST) ViewMode.GRID else ViewMode.LIST) },
                    Modifier.padding(start = 8.dp),
                )
            }
            if (items != null) {
                Text(count(rows.size), style = text(13, tabular = true), color = colors.onSurfaceVariant)
            }
        }
        Box(Modifier.fillMaxSize()) {
            if (view == ViewMode.LIST) {
                val list = rememberLazyListState()
                LazyColumn(Modifier.fillMaxSize(), list, contentPadding = PaddingValues(top = 6.dp, bottom = bottom)) {
                    items(rows, key = key) { item(it) }
                }
                FastScroller(rememberScrolled(list), sectionAt, bottom = bottom + 16.dp)
            } else {
                val grid = rememberLazyGridState()
                LazyVerticalGrid(
                    columns = GridCells.Adaptive(minTile),
                    modifier = Modifier.fillMaxSize(),
                    state = grid,
                    contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 12.dp, bottom = bottom),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalArrangement = Arrangement.spacedBy(rowGap),
                ) {
                    items(rows, key = key) { tile(it) }
                }
                FastScroller(rememberScrolled(grid), sectionAt, bottom = bottom + 16.dp)
            }
        }
    }
}

/**
 * The first scan's card: every track found is listed already and plays; titles and covers fill
 * in as they are read.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun BuildingCard(scan: ScanState, onClick: () -> Unit, modifier: Modifier = Modifier) {
    Column(
        modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(20.dp))
            .background(colors.surfaceLow)
            .clickable(onClick = onClick)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            LoadingIndicator(Modifier.size(36.dp), color = colors.primary)
            Text(
                stringResource(R.string.building_library),
                style = text(16, FontWeight.Bold),
                color = colors.onSurface,
                modifier = Modifier.weight(1f),
            )
            Text(
                stringResource(R.string.read_of_found, formatCount(scan.read), formatCount(scan.found)),
                style = text(13, tabular = true),
                color = colors.onSurfaceVariant,
            )
        }
        WavyProgress({ scan.progress }, Modifier.fillMaxWidth().height(12.dp))
        Text(
            stringResource(R.string.building_library_hint),
            style = text(13, lineHeight = 18.sp),
            color = colors.onSurfaceVariant,
        )
    }
}
