package com.enn3developer.n_music.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.BuildConfig
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.Theme
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.core.Command

private const val CONTRIBUTORS = "https://github.com/Enn3Developer/n_music/graphs/contributors"

/** The Slint app's settings: theme, music folder, rescan, language, and who made it. */
@Composable
fun SettingsScreen(onBack: () -> Unit, onPickFolder: () -> Unit, onOpenLink: (String) -> Unit) {
    val strings = LocalStrings.current
    val ui by UiPreferences.settings.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val languages = remember { Localizations.languages(context) }
    val colors = MaterialTheme.colorScheme
    BackHandler(onBack = onBack)

    Surface(color = colors.surface) {
        Column(
            Modifier
                .fillMaxSize()
                .windowInsetsPadding(WindowInsets.safeDrawing)
                .padding(start = 10.dp, end = 10.dp, bottom = 10.dp)
        ) {
            Row(
                Modifier
                    .fillMaxWidth()
                    .height(56.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    strings.settings,
                    fontSize = 24.sp,
                    color = colors.onSurface,
                    modifier = Modifier.weight(1f),
                )
                FilledIconButton(onClick = onBack) {
                    Icon(painterResource(R.drawable.ic_back), stringResource(R.string.back))
                }
            }
            Column(
                Modifier
                    .verticalScroll(rememberScrollState())
                    .padding(top = 8.dp, bottom = 24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Setting(strings.theme) {
                    val themes = listOf(strings.themeSystem, strings.themeLight, strings.themeDark)
                    ComboBox(
                        options = themes,
                        selected = themes[ui.theme.ordinal],
                        label = strings.theme,
                        onSelect = { UiPreferences.setTheme(Theme.entries[it]) },
                    )
                }
                Setting(strings.musicPath) {
                    FilledTonalButton(onClick = onPickFolder) {
                        Icon(painterResource(R.drawable.ic_folder), strings.musicPath)
                    }
                }
                Setting(strings.rescan) {
                    FilledTonalButton(onClick = {
                        CoreRepository.send(Command.ScanRequested(library = null, checkCache = false))
                    }) {
                        Icon(painterResource(R.drawable.ic_shuffle), strings.rescan)
                    }
                }
                Setting(strings.language) {
                    val denominator = Localizations.denominator(ui.locale)
                    ComboBox(
                        options = languages.map { it.name },
                        selected = Localizations.name(languages, denominator),
                        label = strings.language,
                        onSelect = { UiPreferences.setLocale(languages[it].denominator) },
                        maxVisibleItems = 3,
                    )
                }
                Column(
                    Modifier
                        .fillMaxWidth()
                        .padding(top = 30.dp),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(
                        "N Music v${BuildConfig.VERSION_NAME}",
                        style = MaterialTheme.typography.labelMedium,
                        color = colors.onSurfaceVariant,
                    )
                    TextButton(onClick = { onOpenLink(CONTRIBUTORS) }) {
                        Text(strings.credits)
                    }
                    Text(
                        "${strings.license}: GPL-3",
                        style = MaterialTheme.typography.labelSmall,
                        color = colors.outline,
                    )
                }
            }
        }
    }
}
