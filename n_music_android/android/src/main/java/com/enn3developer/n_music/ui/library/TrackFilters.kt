package com.enn3developer.n_music.ui.library

import androidx.annotation.StringRes
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Facets
import com.enn3developer.n_music.core.Filter
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.Tag

/** A part of the tracks' filters, as its chip and its section of the filter sheet show it. */
enum class FilterField(@param:StringRes val label: Int, @param:StringRes val named: Int) {
    GENRE(R.string.filter_genre, R.string.filter_genre_named),
    YEAR(R.string.filter_year, R.string.filter_year_named),
    PLAYS(R.string.filter_plays, R.string.filter_plays_named),
    LAST_PLAYED(R.string.filter_last_played, R.string.filter_last_played_named),
    FORMAT(R.string.filter_format, R.string.filter_format_named),
    SOURCE(R.string.filter_source, R.string.filter_source_named),
    ARTIST(R.string.filter_artist, R.string.filter_artist_named),
    ALBUM(R.string.filter_album, R.string.filter_album_named),
}

/** The parts the filter sheet has a section for, as [facets] and [sources] sources leave them. */
fun offeredFields(facets: Facets, sources: Int): List<FilterField> = FilterField.entries.filter { field ->
    when (field) {
        FilterField.GENRE -> facets.genres.isNotEmpty()
        FilterField.YEAR -> {
            val first = facets.firstYear
            val last = facets.lastYear
            first != null && last != null && last > first
        }
        FilterField.FORMAT -> facets.codecs.isNotEmpty()
        // Even one source, which a smart playlist can keep to as more come.
        FilterField.SOURCE -> sources > 0
        FilterField.PLAYS, FilterField.LAST_PLAYED, FilterField.ARTIST, FilterField.ALBUM -> true
    }
}

/**
 * An album as the library groups it: its name and its album artist, or first artist. The tracks
 * without an album are one album, with neither.
 */
data class AlbumKey(val name: String?, val artist: String?)

/** How often a track was played. */
enum class PlaysFilter(val min: UInt?, val max: UInt?) {
    NEVER(null, 0u),
    ONE(1u, null),
    THREE(3u, null),
    FIVE(5u, null),
    TEN(10u, null),
    TWENTY_FIVE(25u, null),
}

/** When a track was last played. */
enum class PlayedFilter(
    @param:StringRes val label: Int,
    /** What its chip says, away from the filters' Last played. */
    @param:StringRes val chip: Int,
    val days: Int,
    val within: Boolean,
) {
    PAST_WEEK(R.string.played_past_week, R.string.played_past_week_chip, 7, true),
    PAST_MONTH(R.string.played_past_month, R.string.played_past_month_chip, 30, true),
    PAST_YEAR(R.string.played_past_year, R.string.played_past_year_chip, 365, true),
    NOT_IN_MONTH(R.string.played_not_in_month, R.string.played_not_in_month_chip, 30, false),
    NOT_IN_YEAR(R.string.played_not_in_year, R.string.played_not_in_year_chip, 365, false),
}

/**
 * The filters on the library's tracks. A track has to match every part that is set; where a part
 * lists several values, any of them counts. Years are inclusive, and either end may be open.
 */
data class TrackFilters(
    /** Genres by name; an empty name keeps the tracks without one. */
    val genres: List<String> = emptyList(),
    val yearFrom: Int? = null,
    val yearTo: Int? = null,
    val plays: PlaysFilter? = null,
    val played: PlayedFilter? = null,
    val formats: List<String> = emptyList(),
    val sources: List<Locator> = emptyList(),
    val artist: String? = null,
    val album: AlbumKey? = null,
) {
    /** The parts that are set, in the chips' order. */
    val active: List<FilterField> get() = FilterField.entries.filter(::isSet)

    fun isSet(field: FilterField): Boolean = when (field) {
        FilterField.GENRE -> genres.isNotEmpty()
        FilterField.YEAR -> yearFrom != null || yearTo != null
        FilterField.PLAYS -> plays != null
        FilterField.LAST_PLAYED -> played != null
        FilterField.FORMAT -> formats.isNotEmpty()
        FilterField.SOURCE -> sources.isNotEmpty()
        FilterField.ARTIST -> artist != null
        FilterField.ALBUM -> album != null
    }

    fun clear(field: FilterField): TrackFilters = when (field) {
        FilterField.GENRE -> copy(genres = emptyList())
        FilterField.YEAR -> copy(yearFrom = null, yearTo = null)
        FilterField.PLAYS -> copy(plays = null)
        FilterField.LAST_PLAYED -> copy(played = null)
        FilterField.FORMAT -> copy(formats = emptyList())
        FilterField.SOURCE -> copy(sources = emptyList())
        FilterField.ARTIST -> copy(artist = null)
        FilterField.ALBUM -> copy(album = null)
    }

    /** The core's filter for these: everything while none is set. */
    fun filter(): Filter = Filter.All(
        buildList {
            if (genres.isNotEmpty()) add(any(genres.map { if (it.isEmpty()) Filter.Untagged(Tag.GENRE) else Filter.Genre(it) }))
            if (yearFrom != null || yearTo != null) add(Filter.Year(yearFrom, yearTo))
            plays?.let { add(Filter.Plays(it.min, it.max)) }
            played?.let {
                val seconds = it.days.toULong() * 86_400u
                add(if (it.within) Filter.PlayedWithin(seconds) else Filter.NotPlayedWithin(seconds))
            }
            if (formats.isNotEmpty()) add(any(formats.map(Filter::Codec)))
            if (sources.isNotEmpty()) add(any(sources.map(Filter::Library)))
            artist?.let { add(Filter.Artist(it)) }
            album?.let { add(albumFilter(it.name, it.artist)) }
        }
    )
}

private fun any(filters: List<Filter>): Filter = filters.singleOrNull() ?: Filter.Any(filters)

/** A codec's short name as people know it: `flac` is FLAC, `opus` is Opus. */
fun formatName(codec: String): String {
    val name = codec.lowercase()
    return when {
        name.startsWith("pcm") || name.startsWith("adpcm") -> name.uppercase().replace('_', ' ')
        name in setOf("flac", "mp1", "mp2", "mp3", "aac", "alac", "wma") -> name.uppercase()
        else -> name.replaceFirstChar { it.titlecase() }
    }
}

/** A genre as the filters name it; the empty name stands for tracks without one. */
@Composable
fun genreLabel(name: String): String = name.ifEmpty { stringResource(R.string.no_genre_short) }

/** How a plays filter reads on a chip in the sheet: Never played, 3+. */
@Composable
fun playsChoice(plays: PlaysFilter): String =
    if (plays == PlaysFilter.NEVER) {
        stringResource(R.string.plays_never)
    } else {
        stringResource(R.string.plays_at_least, plays.min!!.toInt())
    }

/** The years a filter keeps: 1990–1999, From 1990, Until 1999. */
@Composable
fun yearsLabel(from: Int?, to: Int?): String = when {
    from != null && to != null && from == to -> from.toString()
    from != null && to != null -> stringResource(R.string.years_range, from, to)
    from != null -> stringResource(R.string.years_from, from)
    to != null -> stringResource(R.string.years_to, to)
    else -> stringResource(R.string.any_year)
}

/** The chip of a part that is set: Jazz, Jazz +2, 3+ plays. */
@Composable
fun TrackFilters.label(field: FilterField, sourceName: (Locator) -> String): String =
    when (field) {
        FilterField.GENRE -> several(genreLabel(genres.first()), genres.size)
        FilterField.YEAR -> yearsLabel(yearFrom, yearTo)
        FilterField.PLAYS -> {
            val plays = plays!!
            if (plays == PlaysFilter.NEVER) {
                stringResource(R.string.plays_never)
            } else {
                val min = plays.min!!.toInt()
                pluralStringResource(R.plurals.plays_at_least_chip, min, min)
            }
        }
        FilterField.LAST_PLAYED -> stringResource(played!!.chip)
        FilterField.FORMAT -> several(formatName(formats.first()), formats.size)
        FilterField.SOURCE -> several(sourceName(sources.first()), sources.size)
        FilterField.ARTIST -> artist!!
        FilterField.ALBUM -> album!!.name ?: stringResource(R.string.no_album_short)
    }

/** The first of several values, with how many more there are: Jazz +2. */
@Composable
private fun several(first: String, count: Int): String =
    if (count > 1) stringResource(R.string.more_values, first, count - 1) else first
