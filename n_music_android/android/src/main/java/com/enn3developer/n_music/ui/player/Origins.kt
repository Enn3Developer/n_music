package com.enn3developer.n_music.ui.player

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.AppController
import com.enn3developer.n_music.ui.Origin
import com.enn3developer.n_music.ui.Page

/** What "Playing from" names [origin]; `null` when it is not known, or gone, like a deleted playlist. */
@Composable
fun originName(origin: Origin?): String? {
    val playlists by CoreRepository.playlists.collectAsStateWithLifecycle()
    val sources by CoreRepository.sources.collectAsStateWithLifecycle()
    return when (origin) {
        null -> null
        Origin.Library -> stringResource(R.string.nav_library)
        is Origin.Search -> stringResource(R.string.origin_search, origin.text)
        is Origin.Album -> origin.name ?: stringResource(R.string.no_album)
        is Origin.Artist -> origin.name ?: stringResource(R.string.no_artist)
        is Origin.Playlist -> playlists.find { it.id == origin.id }?.name
        is Origin.Source -> sources.find { it.root == origin.root }?.name
    }
}

/** Opens the page of what plays, closing the player. */
fun AppController.openOrigin(origin: Origin) = when (origin) {
    Origin.Library -> showTracks(filters)
    is Origin.Search -> open(Page.Search)
    is Origin.Album -> open(Page.Album(origin.name, origin.artist))
    is Origin.Artist -> open(Page.Artist(origin.name))
    is Origin.Playlist -> open(Page.Playlist(origin.id))
    is Origin.Source -> open(Page.Source(origin.root))
}
