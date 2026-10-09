package com.enn3developer.n_music

import com.enn3developer.n_music.ui.theme.Accent
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONArray
import org.json.JSONObject

/** The theme the user picked; [SYSTEM] follows Android's. */
enum class Theme(val stored: String) {
    SYSTEM("System"),
    LIGHT("Light"),
    DARK("Dark"),
}

/** The library's tabs. */
enum class LibraryTab(val stored: String) {
    TRACKS("tracks"),
    ALBUMS("albums"),
    ARTISTS("artists"),
    GENRES("genres"),
}

/** How a library tab lays its items out. */
enum class ViewMode(val stored: String) {
    LIST("list"),
    GRID("grid"),
}

/** A button the mini player can show. */
enum class MiniButton(val stored: String) {
    SHUFFLE("shuffle"),
    PREVIOUS("previous"),
    PLAY_PAUSE("playPause"),
    NEXT("next"),
    REPEAT("repeat"),
    OUTPUT("output"),
    SLEEP_TIMER("sleepTimer"),
}

/** The interface's preferences, in the core's settings file. */
data class UiSettings(
    val theme: Theme = Theme.SYSTEM,
    val accent: Accent = Accent.AMBER,
    val compactRows: Boolean = false,
    /** The mini player's buttons, in order. */
    val miniButtons: List<MiniButton> = DEFAULT_MINI_BUTTONS,
    /** Each library tab's own view: Tracks and Artists start as lists, Albums and Genres as grids. */
    val views: Map<LibraryTab, ViewMode> = DEFAULT_VIEWS,
    /** Each list's sort, by the name of the list. */
    val sorts: Map<String, String> = emptyMap(),
    /** Welcome was gone through: an empty library no longer shows it. */
    val welcomed: Boolean = false,
    /** A tablet's rail is widened into the drawer, which stays open beside the pages. */
    val drawer: Boolean = false,
) {
    fun view(tab: LibraryTab): ViewMode = views[tab] ?: DEFAULT_VIEWS.getValue(tab)

    companion object {
        val DEFAULT_MINI_BUTTONS = listOf(MiniButton.PREVIOUS, MiniButton.PLAY_PAUSE, MiniButton.NEXT)
        val DEFAULT_VIEWS = mapOf(
            LibraryTab.TRACKS to ViewMode.LIST,
            LibraryTab.ALBUMS to ViewMode.GRID,
            LibraryTab.ARTISTS to ViewMode.LIST,
            LibraryTab.GENRES to ViewMode.GRID,
        )
    }
}

/**
 * Keeps [UiSettings] in the core's settings file, in the `android.ui` section where the Slint app
 * kept its theme, so the choice made there carries over. The language is Android's per-app
 * language setting.
 */
object UiPreferences {
    private const val KEY = "android.ui"

    private val _settings = MutableStateFlow(UiSettings())
    val settings: StateFlow<UiSettings> = _settings.asStateFlow()

    /** Reads the stored preferences; the core must be running. */
    fun load() {
        val section = stored() ?: return
        _settings.value = UiSettings(
            theme = Theme.entries.find { it.stored == section.optString("theme") } ?: Theme.SYSTEM,
            accent = Accent.entries.find { it.stored == section.optString("accent") } ?: Accent.AMBER,
            compactRows = section.optBoolean("compactRows", false),
            miniButtons = section.optJSONArray("miniButtons")?.let { array ->
                (0 until array.length()).mapNotNull { index ->
                    MiniButton.entries.find { it.stored == array.optString(index) }
                }.distinct()
            } ?: UiSettings.DEFAULT_MINI_BUTTONS,
            views = section.optJSONObject("views")?.let { views ->
                LibraryTab.entries.associateWith { tab ->
                    ViewMode.entries.find { it.stored == views.optString(tab.stored) }
                        ?: UiSettings.DEFAULT_VIEWS.getValue(tab)
                }
            } ?: UiSettings.DEFAULT_VIEWS,
            sorts = section.optJSONObject("sorts")?.let { sorts ->
                sorts.keys().asSequence().associateWith { sorts.optString(it) }
            } ?: emptyMap(),
            welcomed = section.optBoolean("welcomed", false),
            drawer = section.optBoolean("drawer", false),
        )
    }

    fun setTheme(theme: Theme) = update { it.copy(theme = theme) }

    fun setAccent(accent: Accent) = update { it.copy(accent = accent) }

    fun setCompactRows(compact: Boolean) = update { it.copy(compactRows = compact) }

    fun setMiniButtons(buttons: List<MiniButton>) = update { it.copy(miniButtons = buttons) }

    fun setView(tab: LibraryTab, view: ViewMode) =
        update { it.copy(views = it.views + (tab to view)) }

    fun setSort(list: String, sort: String) = update { it.copy(sorts = it.sorts + (list to sort)) }

    fun setWelcomed() = update { it.copy(welcomed = true) }

    fun setDrawer(open: Boolean) = update { it.copy(drawer = open) }

    private fun update(change: (UiSettings) -> UiSettings) {
        val settings = change(_settings.value)
        if (settings == _settings.value) return
        _settings.value = settings
        // What else the section holds, like the Slint app's window size, stays as it was.
        val section = stored() ?: JSONObject()
        section.put("theme", settings.theme.stored)
        section.put("accent", settings.accent.stored)
        section.put("compactRows", settings.compactRows)
        section.put("miniButtons", JSONArray(settings.miniButtons.map { it.stored }))
        section.put("views", JSONObject(settings.views.entries.associate { it.key.stored to it.value.stored }))
        section.put("sorts", JSONObject(settings.sorts))
        section.put("welcomed", settings.welcomed)
        section.put("drawer", settings.drawer)
        CoreRepository.setSetting(KEY, section.toString())
    }

    private fun stored(): JSONObject? =
        CoreRepository.setting(KEY)?.let { runCatching { JSONObject(it) }.getOrNull() }
}
