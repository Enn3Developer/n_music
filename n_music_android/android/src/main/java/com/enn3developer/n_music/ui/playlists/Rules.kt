package com.enn3developer.n_music.ui.playlists

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Filter
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.Tag
import com.enn3developer.n_music.core.defaultSourceName
import com.enn3developer.n_music.ui.library.PlayedFilter
import com.enn3developer.n_music.ui.library.PlaysFilter
import com.enn3developer.n_music.ui.library.TrackFilters
import com.enn3developer.n_music.ui.library.formatName

private const val DAY = 86_400uL

/**
 * A smart playlist's [rule] as the filter sections set it; `null` when it uses what they cannot
 * show, like "is not", which only the desktop app edits.
 */
fun rulesOf(rule: Filter): TrackFilters? {
    val parts = if (rule is Filter.All) rule.v1 else listOf(rule)
    var rules = TrackFilters()
    val seen = mutableSetOf<String>()
    for (part in parts) {
        // Each section once, as the sheet writes them.
        fun once(section: String) = seen.add(section)
        val values = if (part is Filter.Any) part.v1 else listOf(part)
        rules = when {
            values.all { it is Filter.Genre || (it is Filter.Untagged && it.v1 == Tag.GENRE) } && once("genre") ->
                rules.copy(genres = values.map { if (it is Filter.Genre) it.v1 else "" })
            values.all { it is Filter.Codec } && once("format") ->
                rules.copy(formats = values.map { (it as Filter.Codec).v1 })
            values.all { it is Filter.Library } && once("source") ->
                rules.copy(sources = values.map { (it as Filter.Library).v1 })
            values.size > 1 -> return null
            part is Filter.Year && once("year") -> rules.copy(yearFrom = part.from, yearTo = part.to)
            part is Filter.Plays && once("plays") ->
                rules.copy(plays = PlaysFilter.entries.find { it.min == part.min && it.max == part.max } ?: return null)
            part is Filter.PlayedWithin && once("played") ->
                rules.copy(played = PlayedFilter.entries.find { it.within && it.days.toULong() * DAY == part.seconds } ?: return null)
            part is Filter.NotPlayedWithin && once("played") ->
                rules.copy(played = PlayedFilter.entries.find { !it.within && it.days.toULong() * DAY == part.seconds } ?: return null)
            part is Filter.Artist && once("artist") -> rules.copy(artist = part.v1)
            else -> return null
        }
    }
    return rules
}

/** Whether [rule] keeps a track when it matches all of its parts, rather than any. */
fun matchesAll(rule: Filter): Boolean = rule !is Filter.Any

/** The parts of [rule] a locked smart playlist lists, each in words. */
@Composable
fun ruleLines(rule: Filter): List<String> {
    val parts = when (rule) {
        is Filter.All -> rule.v1
        is Filter.Any -> rule.v1
        else -> listOf(rule)
    }
    return parts.map { describe(it) }
}

/** [filter] in words: Played 5 times or more, Genre isn't Christmas. */
@Composable
private fun describe(filter: Filter): String = when (filter) {
    is Filter.All -> filter.v1.map { describe(it) }.joinToString(stringResource(R.string.rule_and))
    is Filter.Any -> anyOf(filter.v1) ?: filter.v1.map { describe(it) }.joinToString(stringResource(R.string.rule_or))
    is Filter.Not -> negated(filter.v1)
    is Filter.Genre -> stringResource(R.string.rule_genre_is, filter.v1)
    is Filter.Artist -> stringResource(R.string.rule_artist_is, filter.v1)
    is Filter.AlbumArtist -> stringResource(R.string.rule_album_artist_is, filter.v1)
    is Filter.Album -> stringResource(R.string.rule_album_is, filter.v1)
    is Filter.Codec -> stringResource(R.string.rule_format_is, formatName(filter.v1))
    is Filter.Untagged -> stringResource(untagged(filter.v1))
    is Filter.Year -> years(filter.from, filter.to)
    is Filter.Search -> stringResource(R.string.rule_search, filter.v1)
    is Filter.Folder -> stringResource(R.string.rule_folder, filter.v1)
    is Filter.Library -> stringResource(R.string.rule_source, sourceName(filter.v1))
    is Filter.Playlist -> stringResource(R.string.rule_playlist, playlistName(filter.v1))
    is Filter.Plays -> plays(filter.min, filter.max)
    is Filter.PlayedWithin -> stringResource(R.string.rule_played_within, period(filter.seconds))
    is Filter.NotPlayedWithin -> stringResource(R.string.rule_not_played_within, period(filter.seconds))
}

/** Values of one tag, any of which counts, on one line: Genre is Jazz or Blues. */
@Composable
private fun anyOf(values: List<Filter>): String? {
    val or = stringResource(R.string.rule_or_value)
    return when {
        values.all { it is Filter.Genre } -> stringResource(R.string.rule_genre_is, values.joinToString(or) { (it as Filter.Genre).v1 })
        values.all { it is Filter.Artist } -> stringResource(R.string.rule_artist_is, values.joinToString(or) { (it as Filter.Artist).v1 })
        values.all { it is Filter.Codec } -> stringResource(R.string.rule_format_is, values.joinToString(or) { formatName((it as Filter.Codec).v1) })
        else -> null
    }
}

/** What "not" makes of [filter]: Genre isn't Christmas. */
@Composable
private fun negated(filter: Filter): String = when (filter) {
    is Filter.Genre -> stringResource(R.string.rule_genre_is_not, filter.v1)
    is Filter.Artist -> stringResource(R.string.rule_artist_is_not, filter.v1)
    is Filter.AlbumArtist -> stringResource(R.string.rule_album_artist_is_not, filter.v1)
    is Filter.Album -> stringResource(R.string.rule_album_is_not, filter.v1)
    is Filter.Codec -> stringResource(R.string.rule_format_is_not, formatName(filter.v1))
    is Filter.Library -> stringResource(R.string.rule_source_not, sourceName(filter.v1))
    is Filter.Playlist -> stringResource(R.string.rule_playlist_not, playlistName(filter.v1))
    is Filter.Untagged -> stringResource(tagged(filter.v1))
    is Filter.PlayedWithin -> stringResource(R.string.rule_not_played_within, period(filter.seconds))
    is Filter.NotPlayedWithin -> stringResource(R.string.rule_played_within, period(filter.seconds))
    else -> stringResource(R.string.rule_not, describe(filter))
}

@Composable
private fun years(from: Int?, to: Int?): String = when {
    from != null && to != null && from == to -> stringResource(R.string.rule_year_exactly, from)
    from != null && to != null -> stringResource(R.string.rule_years_between, from, to)
    from != null -> stringResource(R.string.rule_years_from, from)
    to != null -> stringResource(R.string.rule_years_to, to)
    else -> stringResource(R.string.any_year)
}

@Composable
private fun plays(min: UInt?, max: UInt?): String = when {
    max == 0u -> stringResource(R.string.plays_never)
    min != null && max != null && min == max -> pluralStringResource(R.plurals.rule_played_exactly, min.toInt(), min.toInt())
    min != null && max != null -> pluralStringResource(R.plurals.rule_played_between, max.toInt(), min.toInt(), max.toInt())
    min != null -> pluralStringResource(R.plurals.rule_played_at_least, min.toInt(), min.toInt())
    max != null -> pluralStringResource(R.plurals.rule_played_at_most, max.toInt(), max.toInt())
    else -> stringResource(R.string.rule_any_plays)
}

/** A span of [seconds] the way people say it: the last 6 months, the last 2 weeks. */
@Composable
private fun period(seconds: ULong): String {
    val days = (seconds / DAY).toInt().coerceAtLeast(1)
    return when {
        days % 365 == 0 -> pluralStringResource(R.plurals.rule_years, days / 365, days / 365)
        days % 30 == 0 -> pluralStringResource(R.plurals.rule_months, days / 30, days / 30)
        days % 7 == 0 -> pluralStringResource(R.plurals.rule_weeks, days / 7, days / 7)
        else -> pluralStringResource(R.plurals.rule_days, days, days)
    }
}

private fun untagged(tag: Tag): Int = when (tag) {
    Tag.GENRE -> R.string.rule_no_genre
    Tag.ARTIST -> R.string.rule_no_artist
    Tag.ALBUM -> R.string.rule_no_album
}

private fun tagged(tag: Tag): Int = when (tag) {
    Tag.GENRE -> R.string.rule_has_genre
    Tag.ARTIST -> R.string.rule_has_artist
    Tag.ALBUM -> R.string.rule_has_album
}

@Composable
private fun sourceName(root: Locator): String {
    val sources by CoreRepository.sources.collectAsStateWithLifecycle()
    return sources.find { it.root == root }?.name ?: defaultSourceName(root)
}

@Composable
private fun playlistName(id: Long): String {
    val playlists by CoreRepository.playlists.collectAsStateWithLifecycle()
    return playlists.find { it.id == id }?.name ?: stringResource(R.string.rule_deleted_playlist)
}
