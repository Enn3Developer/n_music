package com.enn3developer.n_music.ui.settings

import androidx.appcompat.app.AppCompatDelegate
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.core.os.LocaleListCompat
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.components.RadioMark
import com.enn3developer.n_music.ui.components.SheetFrame
import com.enn3developer.n_music.ui.sheets.SheetTitle
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import java.util.Locale

/** The languages N Music speaks, as in `locales_config.xml`. */
private val LANGUAGES = listOf("en")

/**
 * The app's language, on Android 11 and 12, which have no screen of their own for it: Android's
 * own, or one N Music speaks, each named in itself.
 */
@Composable
fun LanguageSheet(open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val title = stringResource(R.string.language)
    val picked = AppCompatDelegate.getApplicationLocales()[0]?.language
    SheetFrame(open, title, onDismissRequest, onGone, header = { SheetTitle(title) }) {
        Column(Modifier.selectableGroup().padding(bottom = 16.dp)) {
            Option(stringResource(R.string.language_system), picked == null) {
                AppCompatDelegate.setApplicationLocales(LocaleListCompat.getEmptyLocaleList())
                onDismissRequest()
            }
            for (tag in LANGUAGES) {
                val locale = Locale.forLanguageTag(tag)
                val name = locale.getDisplayName(locale).replaceFirstChar { it.titlecase(locale) }
                Option(name, picked == locale.language) {
                    AppCompatDelegate.setApplicationLocales(LocaleListCompat.forLanguageTags(tag))
                    onDismissRequest()
                }
            }
        }
    }
}

@Composable
private fun Option(label: String, chosen: Boolean, onClick: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .height(56.dp)
            .selectable(chosen, role = Role.RadioButton, onClick = onClick)
            .padding(horizontal = 24.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        RadioMark(chosen)
        Text(label, style = text(16, if (chosen) FontWeight.Bold else FontWeight.Medium), color = colors.onSurface)
    }
}
