package com.enn3developer.n_music

import com.enn3developer.n_music.ui.Origin
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONObject

/**
 * Where what plays came from, for the player's "Playing from". The core keeps the play session
 * between launches but not where it came from, so it is kept here, next to the interface's
 * preferences in the core's settings file.
 */
object PlayingFrom {
    private const val KEY = "android.playing"

    private val _origin = MutableStateFlow<Origin?>(null)
    val origin: StateFlow<Origin?> = _origin.asStateFlow()

    /** Reads the stored origin; the core must be running. */
    fun load() {
        _origin.value = CoreRepository.setting(KEY)?.let { runCatching { decode(JSONObject(it)) }.getOrNull() }
    }

    fun set(origin: Origin) {
        if (origin == _origin.value) return
        _origin.value = origin
        CoreRepository.setSetting(KEY, encode(origin).toString())
    }

    private fun encode(origin: Origin): JSONObject = JSONObject().apply {
        when (origin) {
            Origin.Library -> put("kind", "library")
            is Origin.Search -> put("kind", "search").put("text", origin.text)
            is Origin.Album -> put("kind", "album").put("name", origin.name).put("artist", origin.artist)
            is Origin.Artist -> put("kind", "artist").put("name", origin.name)
            is Origin.Playlist -> put("kind", "playlist").put("id", origin.id)
            is Origin.Source -> put("kind", "source").put("root", origin.root.encode())
        }
    }

    private fun decode(json: JSONObject): Origin? = when (json.optString("kind")) {
        "library" -> Origin.Library
        "search" -> Origin.Search(json.optString("text"))
        "album" -> Origin.Album(json.text("name"), json.text("artist"))
        "artist" -> Origin.Artist(json.text("name"))
        "playlist" -> if (json.has("id")) Origin.Playlist(json.getLong("id")) else null
        "source" -> json.text("root")?.let(::decodeLocator)?.let(Origin::Source)
        else -> null
    }

    /** The text under [key]; `null` when it is missing, as `put` leaves out a null. */
    private fun JSONObject.text(key: String): String? = if (isNull(key)) null else optString(key)
}
