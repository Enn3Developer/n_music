package com.enn3developer.n_music.ui

import android.content.Context
import android.util.Log
import androidx.compose.runtime.staticCompositionLocalOf
import org.json.JSONObject
import java.util.Locale

/** A bundled translation, `lang/it_Italiano.json` for example. */
data class Language(val denominator: String, val name: String)

/** The interface's texts in one language; keys it lacks come in English, like in the Slint app. */
class Strings(private val texts: Map<String, String>, private val english: Map<String, String>) {
    private fun text(key: String): String = texts[key] ?: english[key] ?: key

    val settings get() = text("settings")
    val search get() = text("search")
    val theme get() = text("theme")
    val musicPath get() = text("music_path")
    val language get() = text("language")
    val themeSystem get() = text("theme_system")
    val themeLight get() = text("theme_light")
    val themeDark get() = text("theme_dark")
    val credits get() = text("credits")
    val license get() = text("license")
    val rescan get() = text("rescan")
}

val LocalStrings = staticCompositionLocalOf { Strings(emptyMap(), emptyMap()) }

/**
 * The translations in the app's `lang` assets, shared in format with the desktop app: one JSON
 * file per language, named after its denominator and its own name for itself.
 */
object Localizations {
    private const val DIRECTORY = "lang"
    private const val ENGLISH = "en"

    fun languages(context: Context): List<Language> =
        (context.assets.list(DIRECTORY) ?: emptyArray())
            .filter { it.endsWith(".json") }
            .mapNotNull { file ->
                val parts = file.substringBefore('.').split('_')
                if (parts.size < 2) null else Language(parts[0], parts[1])
            }
            .sortedBy { it.name }

    /** The language [locale] picks, or the system's when it is `null`. */
    fun denominator(locale: String?): String =
        locale ?: Locale.getDefault().language.ifEmpty { ENGLISH }

    /** What the language picker shows for [denominator]: English for one with no translation. */
    fun name(languages: List<Language>, denominator: String): String =
        languages.find { it.denominator == denominator }?.name ?: "English"

    fun strings(context: Context, denominator: String): Strings {
        val languages = languages(context)
        return Strings(
            texts = read(context, languages.find { it.denominator == denominator }),
            english = read(context, languages.find { it.denominator == ENGLISH }),
        )
    }

    private fun read(context: Context, language: Language?): Map<String, String> {
        language ?: return emptyMap()
        val file = "$DIRECTORY/${language.denominator}_${language.name}.json"
        return runCatching {
            val json = JSONObject(context.assets.open(file).bufferedReader().use { it.readText() })
            json.keys().asSequence().associateWith { json.getString(it) }
        }.onFailure { Log.e("n_music", "Could not read the translation $file", it) }
            .getOrDefault(emptyMap())
    }
}
