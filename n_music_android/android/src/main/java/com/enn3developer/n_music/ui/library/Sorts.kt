package com.enn3developer.n_music.ui.library

import androidx.annotation.StringRes
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.GroupSort
import com.enn3developer.n_music.core.SortField
import com.enn3developer.n_music.core.SortKey

/** What a list of tracks can be sorted by, and how each way of it reads. */
enum class TrackSort(
    val stored: String,
    @param:StringRes val label: Int,
    @param:StringRes val forward: Int,
    @param:StringRes val backward: Int,
    /** It starts with the largest values: the most plays, the latest play. */
    val startsDescending: Boolean = false,
) {
    ARTIST_ALBUM("artist", R.string.sort_artist_album, R.string.sort_a_to_z, R.string.sort_z_to_a),
    TITLE("title", R.string.sort_title, R.string.sort_a_to_z, R.string.sort_z_to_a),
    ALBUM("album", R.string.sort_album, R.string.sort_a_to_z, R.string.sort_z_to_a),
    YEAR("year", R.string.sort_year, R.string.sort_oldest_first, R.string.sort_newest_first),
    MOST_PLAYED("plays", R.string.sort_most_played, R.string.sort_most_first, R.string.sort_fewest_first, true),
    RECENTLY_PLAYED("lastPlayed", R.string.sort_recently_played, R.string.sort_latest_first, R.string.sort_earliest_first, true),
    LENGTH("length", R.string.sort_length, R.string.sort_shortest_first, R.string.sort_longest_first),
    LOCATION("location", R.string.sort_location, R.string.sort_a_to_z, R.string.sort_z_to_a),

    /** When tracks were added to a playlist: plain playlists only. */
    ADDED("added", R.string.sort_recently_added, R.string.sort_latest_first, R.string.sort_earliest_first, true);

    /** The core's keys for this sort, [reversed] or not; [playlist] for [ADDED]. */
    fun keys(reversed: Boolean, playlist: Long? = null): List<SortKey> {
        val descending = startsDescending != reversed
        fun key(field: SortField, flip: Boolean = descending) = SortKey(field, flip)
        return when (this) {
            ARTIST_ALBUM -> listOf(key(SortField.Artist), key(SortField.Album))
            TITLE -> listOf(key(SortField.Title))
            ALBUM -> listOf(key(SortField.Album))
            YEAR -> listOf(key(SortField.Year), key(SortField.Album, false))
            MOST_PLAYED -> listOf(key(SortField.Plays), key(SortField.Artist, false))
            RECENTLY_PLAYED -> listOf(key(SortField.LastPlayed))
            LENGTH -> listOf(key(SortField.Length))
            LOCATION -> listOf(key(SortField.Location))
            ADDED -> listOfNotNull(playlist?.let { key(SortField.Added(it)) })
        }
    }

    companion object {
        /** The options of the sort sheet for the library's and a source's tracks. */
        val library = listOf(ARTIST_ALBUM, TITLE, ALBUM, YEAR, MOST_PLAYED, RECENTLY_PLAYED, LENGTH, LOCATION)
    }
}

/** A track sort and its way: what lists keep in the preferences. */
data class TrackOrder(val sort: TrackSort, val reversed: Boolean = false) {
    fun keys(playlist: Long? = null) = sort.keys(reversed, playlist)

    val stored: String get() = if (reversed) "-${sort.stored}" else sort.stored

    companion object {
        fun parse(text: String?, default: TrackOrder): TrackOrder {
            text ?: return default
            val reversed = text.startsWith("-")
            val sort = TrackSort.entries.find { it.stored == text.removePrefix("-") } ?: return default
            return TrackOrder(sort, reversed)
        }

        /** The order of a playlist's own [keys], as far as the sort sheet can show it. */
        fun of(keys: List<SortKey>): TrackOrder {
            val first = keys.firstOrNull() ?: return TrackOrder(TrackSort.ADDED)
            val sort = when (first.field) {
                SortField.Artist -> TrackSort.ARTIST_ALBUM
                SortField.Title -> TrackSort.TITLE
                SortField.Album -> TrackSort.ALBUM
                SortField.Year -> TrackSort.YEAR
                SortField.Plays -> TrackSort.MOST_PLAYED
                SortField.LastPlayed -> TrackSort.RECENTLY_PLAYED
                SortField.Length -> TrackSort.LENGTH
                SortField.Location -> TrackSort.LOCATION
                is SortField.Added -> TrackSort.ADDED
                SortField.Genre, SortField.Codec -> TrackSort.ARTIST_ALBUM
            }
            return TrackOrder(sort, first.descending != sort.startsDescending)
        }
    }
}

/** How albums, artists or genres can be sorted. */
enum class GroupOrder(val stored: String, @param:StringRes val label: Int, val sort: GroupSort) {
    ARTIST("artist", R.string.sort_album_artist, GroupSort.ARTIST),
    NAME("name", R.string.sort_name, GroupSort.NAME),
    NEWEST("newest", R.string.sort_newest, GroupSort.NEWEST),
    OLDEST("oldest", R.string.sort_oldest, GroupSort.OLDEST),
    MOST_TRACKS("tracks", R.string.sort_most_tracks, GroupSort.MOST_TRACKS);

    companion object {
        val albums = listOf(ARTIST, NAME, NEWEST, OLDEST, MOST_TRACKS)
        val others = listOf(NAME, MOST_TRACKS)

        fun parse(text: String?, default: GroupOrder) = entries.find { it.stored == text } ?: default
    }
}

/** A list someone can sort, and the key its sort is kept under in the preferences. */
enum class SortedList(val stored: String, @param:StringRes val title: Int) {
    TRACKS("tracks", R.string.sort_tracks),
    ALBUMS("albums", R.string.sort_albums),
    ARTISTS("artists", R.string.sort_artists),
    GENRES("genres", R.string.sort_genres),

    /** An artist's tracks, the most played first unless chosen otherwise. */
    ARTIST_TRACKS("artist", R.string.sort_tracks);

    /** Its tracks' order in [sorts]; only for lists of tracks. */
    fun trackOrder(sorts: Map<String, String>): TrackOrder = TrackOrder.parse(
        sorts[stored],
        TrackOrder(if (this == ARTIST_TRACKS) TrackSort.MOST_PLAYED else TrackSort.ARTIST_ALBUM),
    )

    /** Its order in [sorts]; only for albums, artists and genres. */
    fun groupOrder(sorts: Map<String, String>): GroupOrder =
        GroupOrder.parse(sorts[stored], if (this == ALBUMS) GroupOrder.ARTIST else GroupOrder.NAME)

    val ofTracks: Boolean get() = this == TRACKS || this == ARTIST_TRACKS
}
