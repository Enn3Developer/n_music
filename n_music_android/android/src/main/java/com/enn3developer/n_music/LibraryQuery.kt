package com.enn3developer.n_music

import com.enn3developer.n_music.core.AlbumRow
import com.enn3developer.n_music.core.ArtistRow
import com.enn3developer.n_music.core.Filter
import com.enn3developer.n_music.core.GenreRow
import com.enn3developer.n_music.core.GroupSort
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.PlaylistId
import com.enn3developer.n_music.core.PlaylistRow
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.core.TrackDetails
import com.enn3developer.n_music.core.TrackRow

sealed interface LibraryQuery<out T> {
    fun run(): T

    data class Tracks(val query: Query) : LibraryQuery<List<TrackRow>> {
        override fun run() = CoreRepository.tracks(query)
    }

    data class Albums(val filter: Filter, val search: String, val sort: GroupSort) : LibraryQuery<List<AlbumRow>> {
        override fun run() = CoreRepository.albums(filter, search, sort)
    }

    data class Artists(
        val filter: Filter,
        val search: String,
        val sort: GroupSort
    ) : LibraryQuery<List<ArtistRow>> {
        override fun run() = CoreRepository.artists(filter, search, sort)
    }

    data class Genres(val filter: Filter, val search: String, val sort: GroupSort) : LibraryQuery<List<GenreRow>> {
        override fun run() = CoreRepository.genres(filter, search, sort)
    }

    data class Playlist(val id: PlaylistId) : LibraryQuery<PlaylistRow?> {
        override fun run() = CoreRepository.playlist(id)
    }

    data class Details(val locator: Locator) : LibraryQuery<TrackDetails?> {
        override fun run() = CoreRepository.details(locator)
    }

    data class Summary(val filter: Filter) : LibraryQuery<com.enn3developer.n_music.core.Summary> {
        override fun run() = CoreRepository.summary(filter)
    }
}