package com.enn3developer.n_music

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONObject

/** The theme the user picked; [SYSTEM] follows Android's. */
enum class Theme(val stored: String) {
    SYSTEM("System"),
    LIGHT("Light"),
    DARK("Dark"),
}

/** The interface's preferences. [locale] is a translation's denominator, `null` for the system's. */
data class UiSettings(val theme: Theme = Theme.SYSTEM, val locale: String? = null)

/**
 * Keeps [UiSettings] in the core's settings file, in the `android.ui` section where the Slint app
 * kept them, so the choices made there carry over.
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
            locale = if (section.isNull("locale")) null else section.optString("locale"),
        )
    }

    fun setTheme(theme: Theme) = update { it.copy(theme = theme) }

    fun setLocale(denominator: String) = update { it.copy(locale = denominator) }

    private fun update(change: (UiSettings) -> UiSettings) {
        val settings = change(_settings.value)
        _settings.value = settings
        // What else the section holds, like the Slint app's window size, stays as it was.
        val section = stored() ?: JSONObject()
        section.put("theme", settings.theme.stored)
        section.put("locale", settings.locale ?: JSONObject.NULL)
        CoreRepository.setSetting(KEY, section.toString())
    }

    private fun stored(): JSONObject? =
        CoreRepository.setting(KEY)?.let { runCatching { JSONObject(it) }.getOrNull() }
}
