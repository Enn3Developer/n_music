package com.enn3developer.n_music.ui.sources

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.PillButton
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.dialogs.AppDialog
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.theme.AppLogo
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/**
 * The first run: where the music is. Sources picked here wait on the screen, and Build my
 * library hands them to the library, which reads them while it shows.
 */
@Composable
fun WelcomePage() {
    val app = LocalApp.current
    val telegram by CoreRepository.telegram.collectAsStateWithLifecycle()
    WelcomeContent(
        sources = WelcomeDraft.sources,
        onFolder = { app.pickFolder { WelcomeDraft.add(it) } },
        onWeb = { app.show(AppDialog.AddWebPlaylist(draft = true)) },
        onTelegram = { app.show(AppDialog.Telegram(draft = true)) }.takeIf { telegram != null },
        onRemove = { source ->
            WelcomeDraft.remove(source.root)
            app.releaseFolder(source.root)
        },
        onBuild = {
            val picked = WelcomeDraft.sources.toList()
            CoreRepository.build(picked.map { it.root })
            // The core takes them in order, so each name follows its source.
            for (source in picked) {
                if (source.name != null) CoreRepository.send(Command.RenameLibrary(source.root, source.name))
            }
            WelcomeDraft.sources.clear()
            UiPreferences.setWelcomed()
        },
    )
}

/** The screen itself, with [sources] picked so far; a Telegram chat to pick unless [onTelegram] is `null`. */
@Composable
fun WelcomeContent(
    sources: List<DraftSource>,
    onFolder: () -> Unit,
    onWeb: () -> Unit,
    onRemove: (DraftSource) -> Unit,
    onBuild: () -> Unit,
    onTelegram: (() -> Unit)? = null,
) {
    Column(
        Modifier
            .fillMaxSize()
            .background(colors.background)
            .windowInsetsPadding(WindowInsets.statusBars)
            .windowInsetsPadding(WindowInsets.navigationBars)
    ) {
        Column(
            Modifier
                .weight(1f)
                .verticalScroll(rememberScrollState())
                .padding(start = 20.dp, end = 20.dp, top = 40.dp, bottom = 16.dp)
        ) {
            AppLogo(64.dp)
            Text(
                stringResource(R.string.welcome_title),
                style = text(32, FontWeight.ExtraBold, 38.sp, (-0.6).sp),
                color = colors.onSurface,
                modifier = Modifier.padding(top = 24.dp),
            )
            Text(
                stringResource(R.string.welcome_hint),
                style = text(15, lineHeight = 22.sp),
                color = colors.onSurfaceVariant,
                modifier = Modifier.padding(top = 12.dp),
            )
            Text(
                stringResource(R.string.add_a_source),
                style = text(14, FontWeight.Bold),
                color = colors.onSurface,
                modifier = Modifier.padding(top = 28.dp),
            )
            Column(Modifier.padding(top = 10.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                val kinds = if (onTelegram != null) 3 else 2
                Option(NIcons.Sources, R.string.local_folder, R.string.local_folder_hint, groupShape(0, kinds), onFolder)
                Option(NIcons.Web, R.string.web_playlist, R.string.web_playlist_hint, groupShape(1, kinds), onWeb)
                if (onTelegram != null) {
                    Option(NIcons.Telegram, R.string.telegram_chat, R.string.telegram_hint, groupShape(2, kinds), onTelegram)
                }
            }
            if (sources.isNotEmpty()) {
                Row(
                    Modifier.padding(top = 30.dp),
                    verticalAlignment = Alignment.Bottom,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text(stringResource(R.string.added), style = text(14, FontWeight.Bold), color = colors.onSurface)
                    Text(formatCount(sources.size), style = text(13, tabular = true), color = colors.onSurfaceVariant)
                }
                Column(Modifier.padding(top = 10.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    sources.forEachIndexed { index, source ->
                        Added(source, groupShape(index, sources.size)) { onRemove(source) }
                    }
                }
            }
        }
        PillButton(
            stringResource(R.string.build_my_library),
            onBuild,
            modifier = Modifier
                .padding(start = 20.dp, end = 20.dp, bottom = 16.dp)
                .fillMaxWidth(),
            height = 56.dp,
            textStyle = text(16, FontWeight.Bold),
            padding = PaddingValues(horizontal = 24.dp),
            enabled = sources.isNotEmpty(),
        )
    }
}

/** A kind of source to add: its icon, name and what it is, as one card of a group. */
@Composable
private fun Option(icon: ImageVector, name: Int, detail: Int, shape: RoundedCornerShape, onClick: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .height(72.dp)
            .clip(shape)
            .background(colors.surfaceLow)
            .tappable(onClick)
            .padding(horizontal = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Box(
            Modifier
                .size(48.dp)
                .background(colors.secondaryContainer, RoundedCornerShape(14.dp)),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(icon, tint = colors.onSecondaryContainer)
        }
        Column(Modifier.weight(1f)) {
            Text(stringResource(name), style = text(16, FontWeight.SemiBold), color = colors.onSurface)
            Text(
                stringResource(detail),
                style = text(14),
                color = colors.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(top = 2.dp),
            )
        }
        NIcon(NIcons.Open, tint = colors.onSurfaceVariant)
    }
}

/** A source picked so far: its name, where it is, and × to take it back out. */
@Composable
private fun Added(source: DraftSource, shape: RoundedCornerShape, onRemove: () -> Unit) {
    val name = source.name ?: defaultName(source.root)
    Row(
        Modifier
            .fillMaxWidth()
            .height(72.dp)
            .background(colors.surfaceLow, shape)
            .padding(start = 12.dp, end = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Box(
            Modifier
                .size(48.dp)
                .background(colors.surfaceHigh, RoundedCornerShape(14.dp)),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(sourceIcon(source.root), tint = colors.onSurfaceVariant)
        }
        Column(Modifier.weight(1f)) {
            Text(
                name,
                style = text(16, FontWeight.SemiBold),
                color = colors.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                sourcePlace(source.root),
                style = text(14),
                color = colors.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(top = 2.dp),
            )
        }
        NIconButton(NIcons.Close, stringResource(R.string.remove_source_named, name), onRemove, tint = colors.onSurfaceVariant)
    }
}
