package com.enn3developer.n_music.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.decodeLocator
import com.enn3developer.n_music.encode
import com.enn3developer.n_music.ui.components.Tab

/** A page of the app. */
sealed interface Page {
    data object Library : Page
    data object Playlists : Page
    data object Sources : Page
    data object Search : Page
    data object Settings : Page
    data object Licence : Page
    data class Album(val name: String?, val artist: String?) : Page
    data class Artist(val name: String?) : Page
    data class Playlist(val id: Long) : Page
    data class Source(val root: Locator) : Page

    /** Shows the navigation bar and the mini player under it. */
    val navigation: Boolean
        get() = this !is Search && this !is Settings && this !is Licence

    /** Shows the mini player: Settings hides playback controls. */
    val miniPlayer: Boolean
        get() = this !is Settings && this !is Licence
}

/** A page on a back stack; [id] tells two visits of the same page apart. */
data class Entry(val page: Page, val id: Long)

/**
 * Where the app is: a back stack for each [Tab], so each keeps its pages while another shows.
 * The tab's own page is at the bottom of its stack.
 */
@Stable
class Navigator(tab: Tab, saved: Map<Tab, List<Entry>>, private var nextId: Long) {
    var tab by mutableStateOf(tab)
        private set

    private val stacks = Tab.entries.associateWith { tab ->
        mutableStateListOf<Entry>().apply {
            addAll(saved[tab].orEmpty().ifEmpty { listOf(Entry(root(tab), 0)) })
        }
    }

    val stack: List<Entry> get() = stacks.getValue(tab)

    val current: Entry get() = stack.last()

    /** Can go back within the app: a page over a tab's own, or a tab other than Library. */
    val canGoBack: Boolean get() = stack.size > 1 || tab != Tab.LIBRARY

    /** Opens [page] over the current one. */
    fun open(page: Page) {
        if (current.page == page) return
        stacks.getValue(tab).add(Entry(page, nextId++))
    }

    /** Shows [tab]; the one showing goes back to its own page. */
    fun select(tab: Tab) {
        if (tab == this.tab) {
            val stack = stacks.getValue(tab)
            while (stack.size > 1) stack.removeAt(stack.lastIndex)
        } else {
            this.tab = tab
        }
    }

    /** Goes back a page, or to Library from another tab's own page; false when there is none. */
    fun back(): Boolean {
        val stack = stacks.getValue(tab)
        return when {
            stack.size > 1 -> {
                stack.removeAt(stack.lastIndex)
                true
            }

            tab != Tab.LIBRARY -> {
                tab = Tab.LIBRARY
                true
            }

            else -> false
        }
    }

    /** The pages on any stack. */
    fun liveIds(): Set<Long> = stacks.values.flatMapTo(mutableSetOf()) { stack -> stack.map { it.id } }

    companion object {
        fun root(tab: Tab): Page = when (tab) {
            Tab.LIBRARY -> Page.Library
            Tab.PLAYLISTS -> Page.Playlists
            Tab.SOURCES -> Page.Sources
        }

        /** Keeps the stacks as text through a configuration change or the process's death. */
        val Saver: Saver<Navigator, List<String>> = Saver(
            save = { navigator ->
                buildList {
                    add(navigator.tab.name)
                    add(navigator.nextId.toString())
                    for ((tab, stack) in navigator.stacks) {
                        for (entry in stack) add("${tab.name}\u0000${entry.id}\u0000${entry.page.encode()}")
                    }
                }
            },
            restore = { saved ->
                val tab = Tab.entries.find { it.name == saved.getOrNull(0) } ?: Tab.LIBRARY
                val nextId = saved.getOrNull(1)?.toLongOrNull() ?: 1
                val stacks = saved.drop(2).mapNotNull { line ->
                    val parts = line.split('\u0000', limit = 3)
                    val owner = Tab.entries.find { it.name == parts.getOrNull(0) } ?: return@mapNotNull null
                    val id = parts.getOrNull(1)?.toLongOrNull() ?: return@mapNotNull null
                    val page = parts.getOrNull(2)?.let(::decodePage) ?: return@mapNotNull null
                    owner to Entry(page, id)
                }.groupBy({ it.first }, { it.second })
                Navigator(tab, stacks, nextId)
            },
        )
    }
}

@Composable
fun rememberNavigator(): Navigator =
    rememberSaveable(saver = Navigator.Saver) { Navigator(Tab.LIBRARY, emptyMap(), 1) }

private fun Page.encode(): String = when (this) {
    Page.Library -> "library"
    Page.Playlists -> "playlists"
    Page.Sources -> "sources"
    Page.Search -> "search"
    Page.Settings -> "settings"
    Page.Licence -> "licence"
    is Page.Album -> "album\u0001${name.orEmpty()}\u0001${artist.orEmpty()}\u0001${name != null}\u0001${artist != null}"
    is Page.Artist -> "artist\u0001${name.orEmpty()}\u0001${name != null}"
    is Page.Playlist -> "playlist\u0001$id"
    is Page.Source -> "source\u0001${root.encode()}"
}

private fun decodePage(text: String): Page? {
    val parts = text.split('\u0001')
    return when (parts[0]) {
        "library" -> Page.Library
        "playlists" -> Page.Playlists
        "sources" -> Page.Sources
        "search" -> Page.Search
        "settings" -> Page.Settings
        "licence" -> Page.Licence
        "album" -> if (parts.size == 5) {
            Page.Album(parts[1].takeIf { parts[3] == "true" }, parts[2].takeIf { parts[4] == "true" })
        } else {
            null
        }
        "artist" -> if (parts.size == 3) Page.Artist(parts[1].takeIf { parts[2] == "true" }) else null
        "playlist" -> parts.getOrNull(1)?.toLongOrNull()?.let(Page::Playlist)
        "source" -> parts.getOrNull(1)?.let(::decodeLocator)?.let(Page::Source)
        else -> null
    }
}
