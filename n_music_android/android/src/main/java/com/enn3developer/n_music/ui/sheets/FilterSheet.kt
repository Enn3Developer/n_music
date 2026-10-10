package com.enn3developer.n_music.ui.sheets

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.RangeSlider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.layout.positionInParent
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.ArtistRow
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Facet
import com.enn3developer.n_music.core.Facets
import com.enn3developer.n_music.core.Filter
import com.enn3developer.n_music.core.GroupSort
import com.enn3developer.n_music.core.SourceRow
import com.enn3developer.n_music.core.defaultSourceName
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.components.CheckMark
import com.enn3developer.n_music.ui.components.NChip
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.OutlinedField
import com.enn3developer.n_music.ui.components.PillButton
import com.enn3developer.n_music.ui.components.RadioMark
import com.enn3developer.n_music.ui.components.SearchField
import com.enn3developer.n_music.ui.components.SheetClose
import com.enn3developer.n_music.ui.components.SheetFrame
import com.enn3developer.n_music.ui.components.TextAction
import com.enn3developer.n_music.ui.components.inSideSheet
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.library.FilterField
import com.enn3developer.n_music.ui.library.PlayedFilter
import com.enn3developer.n_music.ui.library.PlaysFilter
import com.enn3developer.n_music.ui.library.TrackFilters
import com.enn3developer.n_music.ui.library.TrackOrder
import com.enn3developer.n_music.ui.library.TrackSort
import com.enn3developer.n_music.ui.library.formatName
import com.enn3developer.n_music.ui.library.genreLabel
import com.enn3developer.n_music.ui.library.playsChoice
import com.enn3developer.n_music.ui.library.yearsLabel
import com.enn3developer.n_music.ui.playlists.rulesOf
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.rememberLibrary
import com.enn3developer.n_music.ui.rememberLibraryRead
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import com.enn3developer.n_music.ui.tracksCount
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import kotlin.math.roundToInt

/** How many genres show as chips before See all. */
private const val GENRE_CHIPS = 9

/** How many genres a sheet from the side shows as chips: two rows of its width. */
private const val SIDE_GENRE_CHIPS = 8

/**
 * The tracks' filters, a part to a section. Changes stay in the sheet until Show applies them;
 * [focus] scrolls to its part when the sheet opens, or opens the artist picker. Save as smart
 * playlist starts one with them.
 */
@Composable
fun FilterSheet(focus: FilterField?, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val app = LocalApp.current
    val facets by CoreRepository.facets.collectAsStateWithLifecycle()
    val sources by CoreRepository.sources.collectAsStateWithLifecycle()
    FilterSheet(
        filters = app.filters,
        facets = facets,
        sources = sources,
        count = { CoreRepository.summary(it.filter()).tracks },
        artists = { CoreRepository.artists(Filter.All(emptyList()), it, GroupSort.NAME) },
        onApply = { app.filters = it },
        onSaveSmart = { app.show(Sheet.SmartPlaylist(null, null, it)) },
        focus = focus,
        open = open,
        onDismissRequest = onDismissRequest,
        onGone = onGone,
    )
}

/**
 * The filter sheet over [filters], offering what [facets] and [sources] hold. [count] reads how
 * many tracks a set of filters keeps and [artists] the artists matching a search, both off the
 * main thread; [onApply] takes the filters Show applies, [onSaveSmart] those to start a smart
 * playlist with.
 */
@Composable
fun FilterSheet(
    filters: TrackFilters,
    facets: Facets,
    sources: List<SourceRow>,
    count: (TrackFilters) -> UInt,
    artists: (String) -> List<ArtistRow>,
    onApply: (TrackFilters) -> Unit,
    onSaveSmart: (TrackFilters) -> Unit,
    focus: FilterField?,
    open: Boolean,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
) {
    RulesSheet(
        title = stringResource(R.string.filters),
        filters = filters,
        facets = facets,
        sources = sources,
        count = count,
        artists = artists,
        focus = focus,
        open = open,
        onDismissRequest = onDismissRequest,
        onGone = onGone,
        intro = {
            Text(
                stringResource(R.string.filters_hint),
                style = text(13, lineHeight = 18.sp),
                color = colors.onSurfaceVariant,
                modifier = Modifier.padding(start = 24.dp, end = 24.dp, top = if (inSideSheet) 0.dp else 2.dp),
            )
        },
    ) { draft, matching ->
        TextAction(
            stringResource(R.string.save_smart_playlist),
            {
                onDismissRequest()
                onSaveSmart(draft)
            },
            height = 44.dp,
        )
        Spacer(Modifier.weight(1f))
        PillButton(
            if (matching == null) {
                stringResource(R.string.show_tracks_counting)
            } else {
                pluralStringResource(R.plurals.show_tracks, quantity(matching.toLong()), formatCount(matching))
            },
            onClick = {
                onApply(draft)
                onDismissRequest()
            },
            padding = PaddingValues(horizontal = 22.dp),
            enabled = matching != 0u,
        )
    }
}

/**
 * A smart playlist's rules: a new one's, starting from [start], under a field for its name, or
 * playlist [id]'s as they are. Saving makes the new one and opens it, or sets the rules.
 */
@Composable
fun SmartPlaylistSheet(
    id: Long?,
    focus: FilterField?,
    start: TrackFilters?,
    open: Boolean,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
) {
    val app = LocalApp.current
    val facets by CoreRepository.facets.collectAsStateWithLifecycle()
    val sources by CoreRepository.sources.collectAsStateWithLifecycle()
    val read = id?.let { rememberLibraryRead(it) { CoreRepository.playlist(it) } }
    val playlist = read?.value
    if (id != null) {
        // The sheet waits for the rules it opens on, and closes if the playlist went meanwhile.
        if (read == null) return
        if (playlist == null) {
            LaunchedEffect(Unit) {
                onDismissRequest()
                onGone()
            }
            return
        }
    }
    SmartPlaylistSheet(
        rules = playlist?.rule?.let(::rulesOf) ?: start ?: TrackFilters(),
        named = playlist == null,
        facets = facets,
        sources = sources,
        count = { CoreRepository.summary(it.filter()).tracks },
        artists = { CoreRepository.artists(Filter.All(emptyList()), it, GroupSort.NAME) },
        onSave = { name, rules ->
            if (playlist != null) {
                CoreRepository.send(Command.SetPlaylistRule(playlist.id, rules.filter()))
            } else {
                val made = System.currentTimeMillis() / 1000
                CoreRepository.send(
                    Command.CreatePlaylist(name, rules.filter(), TrackOrder(TrackSort.MOST_PLAYED).keys(), emptyList())
                )
                // Opens it once the core lists it.
                app.scope.launch {
                    val created = withTimeoutOrNull(5_000) {
                        CoreRepository.playlists.first { list -> list.any { it.name == name && it.created >= made - 1 } }
                    }
                    created?.filter { it.name == name }?.maxByOrNull { it.created }?.let { app.open(Page.Playlist(it.id)) }
                }
            }
        },
        focus = focus,
        open = open,
        onDismissRequest = onDismissRequest,
        onGone = onGone,
    )
}

/**
 * The smart playlist sheet itself over [rules]: with a field for its name and Save with how many
 * tracks match while [named], for a new one, else Done for the rules of one there is. Save waits
 * for a name.
 */
@Composable
fun SmartPlaylistSheet(
    rules: TrackFilters,
    named: Boolean,
    facets: Facets,
    sources: List<SourceRow>,
    count: (TrackFilters) -> UInt,
    artists: (String) -> List<ArtistRow>,
    onSave: (name: String, rules: TrackFilters) -> Unit,
    focus: FilterField?,
    open: Boolean,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
    name: String = "",
) {
    var chosen by rememberSaveable { mutableStateOf(name) }
    RulesSheet(
        title = stringResource(if (named) R.string.new_smart_playlist else R.string.edit_smart_rules),
        filters = rules,
        facets = facets,
        sources = sources,
        count = count,
        artists = artists,
        focus = focus,
        open = open,
        onDismissRequest = onDismissRequest,
        onGone = onGone,
        intro = {
            if (named) {
                OutlinedField(
                    chosen,
                    { chosen = it },
                    stringResource(R.string.name),
                    Modifier.padding(start = 24.dp, end = 24.dp, top = 14.dp, bottom = 2.dp),
                    fill = colors.surfaceLow,
                    focus = focus == null,
                )
            }
            Text(
                stringResource(R.string.smart_playlist_hint),
                style = text(13, lineHeight = 18.sp),
                color = colors.onSurfaceVariant,
                modifier = Modifier.padding(start = 24.dp, end = 24.dp, top = if (named) 6.dp else 2.dp),
            )
        },
    ) { draft, matching ->
        TextAction(stringResource(R.string.cancel), onDismissRequest, height = 44.dp)
        Spacer(Modifier.weight(1f))
        PillButton(
            when {
                !named -> stringResource(R.string.done)
                matching == null -> stringResource(R.string.save_counting)
                else -> pluralStringResource(R.plurals.save_tracks, quantity(matching.toLong()), formatCount(matching))
            },
            onClick = {
                onSave(chosen.trim(), draft)
                onDismissRequest()
            },
            padding = PaddingValues(horizontal = 22.dp),
            enabled = !named || chosen.isNotBlank(),
        )
    }
}

/**
 * Filter sections over a draft of [filters], for the library's filters and smart playlists'
 * rules alike: [intro] under the title, and [footer] given the draft and how many tracks it
 * keeps once counted.
 */
@Composable
private fun RulesSheet(
    title: String,
    filters: TrackFilters,
    facets: Facets,
    sources: List<SourceRow>,
    count: (TrackFilters) -> UInt,
    artists: (String) -> List<ArtistRow>,
    focus: FilterField?,
    open: Boolean,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
    intro: @Composable ColumnScope.() -> Unit,
    footer: @Composable RowScope.(draft: TrackFilters, matching: UInt?) -> Unit,
) {
    var draft by remember { mutableStateOf(filters) }
    var picker by remember { mutableStateOf(focus?.takeIf { it == FilterField.ARTIST }) }
    val matching = rememberLibrary<UInt?>(null, draft) { count(draft) }
    SheetFrame(
        open, title, onDismissRequest, onGone,
        tall = true,
        header = {
            Row(
                Modifier
                    .fillMaxWidth()
                    .then(if (inSideSheet) Modifier.height(64.dp) else Modifier)
                    .padding(start = 24.dp, end = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Text(
                    title,
                    style = text(22, FontWeight.ExtraBold),
                    color = colors.onSurface,
                    modifier = Modifier.weight(1f),
                )
                TextAction(stringResource(R.string.clear_all), { draft = TrackFilters() })
                SheetClose()
            }
            intro()
        },
    ) {
        // Back goes to the handler added last, so this one, after the sheet's, closes the picker first.
        BackHandler(open && picker != null) { picker = null }
        AnimatedContent(
            picker,
            modifier = Modifier.weight(1f),
            transitionSpec = {
                if (targetState != null) {
                    (slideInHorizontally(NMotion.noBounce()) { it / 4 } + fadeIn(NMotion.effectsDefault()))
                        .togetherWith(slideOutHorizontally(NMotion.noBounce()) { -it / 8 } + fadeOut(NMotion.effectsFast()))
                } else {
                    (slideInHorizontally(NMotion.noBounce()) { -it / 8 } + fadeIn(NMotion.effectsDefault()))
                        .togetherWith(slideOutHorizontally(NMotion.noBounce()) { it / 4 } + fadeOut(NMotion.effectsFast()))
                }
            },
            label = "picker",
        ) { shown ->
            when (shown) {
                FilterField.GENRE -> GenrePicker(
                    genres = facets.genres,
                    chosen = draft.genres,
                    onToggle = { genre -> draft = draft.copy(genres = draft.genres.toggle(genre)) },
                    onBack = { picker = null },
                )

                FilterField.ARTIST -> ArtistPicker(
                    artists = artists,
                    chosen = draft.artist,
                    onPick = { artist ->
                        draft = draft.copy(artist = artist)
                        picker = null
                    },
                    onBack = { picker = null },
                )

                else -> Sections(draft, { draft = it }, facets, sources, focus, onPicker = { picker = it })
            }
        }
        // From the side, it takes in the gesture area, as the design has it.
        val side = inSideSheet
        Row(
            Modifier
                .fillMaxWidth()
                .height(if (side) 88.dp else 72.dp)
                .topLine()
                .padding(start = 12.dp, end = if (side) 24.dp else 16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            footer(draft, matching)
        }
    }
}

/** The sections of the filters, each a part; [focus] is scrolled to when they first show. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun Sections(
    draft: TrackFilters,
    onChange: (TrackFilters) -> Unit,
    facets: Facets,
    sources: List<SourceRow>,
    focus: FilterField?,
    onPicker: (FilterField) -> Unit,
) {
    val scroll = rememberScrollState()
    // Where each section starts, to scroll to the one a chip opened the sheet at.
    val tops = remember { mutableStateMapOf<FilterField, Int>() }
    LaunchedEffect(Unit) {
        if (focus != null && focus != FilterField.GENRE) scrollTo(scroll, snapshotFlow { tops[focus] })
    }
    Column(
        Modifier
            .fillMaxSize()
            .verticalScroll(scroll)
            .padding(start = 24.dp, end = 24.dp, top = 16.dp, bottom = 16.dp),
        verticalArrangement = Arrangement.spacedBy(if (inSideSheet) 16.dp else 18.dp),
    ) {
        fun Modifier.top(field: FilterField) = onPlaced { tops[field] = it.positionInParent().y.roundToInt() }

        val shown = if (inSideSheet) SIDE_GENRE_CHIPS else GENRE_CHIPS
        val genres = facets.genres.take(shown).map(Facet::name)
        val names = genres + draft.genres.filter { chosen -> genres.none { it.equals(chosen, ignoreCase = true) } }
        if (names.isNotEmpty()) {
            Section(
                FilterField.GENRE,
                Modifier.top(FilterField.GENRE),
                trailing = {
                    if (facets.genres.size > shown) {
                        TextAction(
                            stringResource(R.string.see_all, formatCount(facets.genres.size)),
                            { onPicker(FilterField.GENRE) },
                            height = 32.dp,
                            textStyle = text(13, FontWeight.Bold),
                            padding = PaddingValues(start = 10.dp, end = 4.dp),
                            trailing = NIcons.Open,
                        )
                    }
                },
            ) {
                Chips {
                    for (genre in names) {
                        val on = draft.genres.any { it.equals(genre, ignoreCase = true) }
                        NChip(genreLabel(genre), { onChange(draft.copy(genres = draft.genres.toggle(genre))) }, selected = on)
                    }
                }
            }
        }

        val first = facets.firstYear
        val last = facets.lastYear
        if (first != null && last != null && last > first) {
            Section(
                FilterField.YEAR,
                Modifier.top(FilterField.YEAR),
                trailing = {
                    Text(
                        yearsLabel(draft.yearFrom, draft.yearTo),
                        style = text(13, FontWeight.SemiBold, tabular = true),
                        color = colors.onSurfaceVariant,
                    )
                },
            ) {
                val from = (draft.yearFrom ?: first).coerceIn(first, last)
                val to = (draft.yearTo ?: last).coerceIn(first, last)
                RangeSlider(
                    value = from.toFloat()..to.toFloat(),
                    onValueChange = { range ->
                        val start = range.start.roundToInt()
                        val end = range.endInclusive.roundToInt()
                        // The library's ends stay open, so newer and older tracks still match.
                        onChange(draft.copy(yearFrom = start.takeIf { it > first }, yearTo = end.takeIf { it < last }))
                    },
                    valueRange = first.toFloat()..last.toFloat(),
                    colors = SliderDefaults.colors(
                        thumbColor = colors.primary,
                        activeTrackColor = colors.primary,
                        inactiveTrackColor = colors.secondaryContainer,
                        activeTickColor = colors.onPrimary,
                        inactiveTickColor = colors.onSecondaryContainer,
                    ),
                    // Material's slider is 48 dp tall around the design's 44 dp handles.
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(top = 4.dp),
                )
                Row(Modifier.fillMaxWidth().padding(top = 2.dp)) {
                    Text(first.toString(), style = text(12, tabular = true), color = colors.onSurfaceVariant)
                    Spacer(Modifier.weight(1f))
                    Text(last.toString(), style = text(12, tabular = true), color = colors.onSurfaceVariant)
                }
            }
        }

        Section(FilterField.PLAYS, Modifier.top(FilterField.PLAYS), pickOne = true) {
            Chips {
                for (plays in PlaysFilter.entries) {
                    val on = draft.plays == plays
                    NChip(
                        playsChoice(plays),
                        { onChange(draft.copy(plays = if (on) null else plays)) },
                        selected = on,
                        role = Role.RadioButton,
                    )
                }
            }
        }

        Section(FilterField.LAST_PLAYED, Modifier.top(FilterField.LAST_PLAYED), pickOne = true) {
            Chips {
                for (played in PlayedFilter.entries) {
                    val on = draft.played == played
                    NChip(
                        stringResource(played.label),
                        { onChange(draft.copy(played = if (on) null else played)) },
                        selected = on,
                        role = Role.RadioButton,
                    )
                }
            }
        }

        if (facets.codecs.isNotEmpty()) {
            Section(FilterField.FORMAT, Modifier.top(FilterField.FORMAT)) {
                Chips {
                    for (codec in facets.codecs) {
                        val on = codec.name in draft.formats
                        NChip(
                            formatName(codec.name),
                            { onChange(draft.copy(formats = draft.formats.toggle(codec.name))) },
                            selected = on,
                        )
                    }
                }
            }
        }

        // One source has nothing to tell apart.
        if (sources.size > 1) {
            Section(FilterField.SOURCE, Modifier.top(FilterField.SOURCE)) {
                Chips {
                    for (source in sources) {
                        val on = source.root in draft.sources
                        NChip(
                            source.name ?: defaultSourceName(source.root),
                            {
                                val chosen = if (on) draft.sources - source.root else draft.sources + source.root
                                onChange(draft.copy(sources = chosen))
                            },
                            selected = on,
                        )
                    }
                }
            }
        }

        Row(
            Modifier
                .fillMaxWidth()
                .height(56.dp)
                .topLine()
                .clickable(role = Role.Button) { onPicker(FilterField.ARTIST) },
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(stringResource(R.string.filter_artist), style = text(16, FontWeight.Bold), color = colors.onSurface)
            Text(
                draft.artist ?: stringResource(R.string.any_artist),
                style = text(14),
                color = colors.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.End,
                modifier = Modifier.weight(1f),
            )
            NIcon(NIcons.Open, size = 20.dp, tint = colors.onSurfaceVariant)
        }
    }
}

/** Scrolls to where [top] says a section starts, once it has been placed. */
private suspend fun scrollTo(scroll: ScrollState, top: Flow<Int?>) {
    scroll.scrollTo(top.filterNotNull().first())
}

/** A part of the filters: its name, what it says on the right, and its choices under it. */
@Composable
private fun Section(
    field: FilterField,
    modifier: Modifier = Modifier,
    pickOne: Boolean = false,
    trailing: @Composable () -> Unit = {},
    content: @Composable ColumnScope.() -> Unit,
) {
    Column(modifier.fillMaxWidth()) {
        Row(Modifier.fillMaxWidth().heightIn(min = 28.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(stringResource(field.label), style = text(16, FontWeight.Bold), color = colors.onSurface)
            if (pickOne) {
                Text(
                    stringResource(R.string.pick_one),
                    style = text(13),
                    color = colors.onSurfaceVariant,
                    modifier = Modifier.padding(start = 8.dp),
                )
            }
            Spacer(Modifier.weight(1f))
            trailing()
        }
        content()
    }
}

/** Chips wrapping onto as many lines as they need. */
@Composable
private fun Chips(content: @Composable () -> Unit) {
    FlowRow(
        Modifier.padding(top = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) { content() }
}

/** Every genre, to tick as many as wanted. */
@Composable
private fun GenrePicker(genres: List<Facet>, chosen: List<String>, onToggle: (String) -> Unit, onBack: () -> Unit) {
    var search by rememberSaveable { mutableStateOf("") }
    val shown = genres.filter { it.name.contains(search.trim(), ignoreCase = true) }
    Picker(stringResource(R.string.filter_genre), search, { search = it }, stringResource(R.string.search_genres), onBack) {
        items(shown, key = Facet::name) { genre ->
            val on = chosen.any { it.equals(genre.name, ignoreCase = true) }
            PickerRow(genre.name, tracksCount(genre.tracks), on, Role.Checkbox, { onToggle(genre.name) }) { CheckMark(on) }
        }
    }
}

/** Every artist, to pick one, or any. */
@Composable
private fun ArtistPicker(
    artists: (String) -> List<ArtistRow>,
    chosen: String?,
    onPick: (String?) -> Unit,
    onBack: () -> Unit,
) {
    var search by rememberSaveable { mutableStateOf("") }
    val found = rememberLibrary(emptyList(), search) { artists(search.trim()).filter { it.name != null } }
    Picker(stringResource(R.string.filter_artist), search, { search = it }, stringResource(R.string.search_artists), onBack) {
        if (search.isBlank()) {
            item(key = "") {
                PickerRow(stringResource(R.string.any_artist), null, chosen == null, Role.RadioButton, { onPick(null) }) {
                    RadioMark(chosen == null)
                }
            }
        }
        items(found, key = { it.name!! }) { artist ->
            val name = artist.name!!
            val on = name.equals(chosen, ignoreCase = true)
            PickerRow(name, tracksCount(artist.tracks), on, Role.RadioButton, { onPick(name) }) { RadioMark(on) }
        }
    }
}

/** A list to pick from inside the sheet, with the way back and a field narrowing it. */
@Composable
private fun Picker(
    title: String,
    search: String,
    onSearch: (String) -> Unit,
    hint: String,
    onBack: () -> Unit,
    items: LazyListScope.() -> Unit,
) {
    Column(Modifier.fillMaxSize()) {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(start = 12.dp, end = 24.dp, top = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            NIconButton(NIcons.Back, stringResource(R.string.back), onBack, tint = colors.onSurface)
            Text(title, style = text(18, FontWeight.Bold), color = colors.onSurface)
        }
        SearchField(search, onSearch, hint, Modifier.padding(start = 24.dp, end = 24.dp, top = 4.dp, bottom = 8.dp))
        LazyColumn(Modifier.fillMaxSize(), content = items)
    }
}

/** A choice in a picker: its mark, its name and how many tracks it has. */
@Composable
private fun PickerRow(
    name: String,
    count: String?,
    on: Boolean,
    role: Role,
    onClick: () -> Unit,
    mark: @Composable () -> Unit,
) {
    Row(
        Modifier
            .fillMaxWidth()
            .height(56.dp)
            .toggleable(on, role = role) { onClick() }
            .padding(horizontal = 24.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        mark()
        Text(
            name,
            style = text(16, if (on) FontWeight.Bold else FontWeight.Medium),
            color = colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        if (count != null) Text(count, style = text(13, tabular = true), color = colors.onSurfaceVariant)
    }
}

/** The thin line along the top of a bar or a row. */
@Composable
private fun Modifier.topLine(): Modifier {
    val line = colors.outlineVariant
    return drawBehind { drawRect(line, size = Size(size.width, 1.dp.toPx())) }
}

/** [value] added when it is not in the list, taken out when it is, ignoring case. */
private fun List<String>.toggle(value: String): List<String> =
    if (any { it.equals(value, ignoreCase = true) }) filterNot { it.equals(value, ignoreCase = true) } else this + value
