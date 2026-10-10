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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ScanState
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.SourceRow
import com.enn3developer.n_music.core.TelegramStatus
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.LocalBottomSpace
import com.enn3developer.n_music.ui.LocalPageMargins
import com.enn3developer.n_music.ui.LocalWindowLayout
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.bottomPadding
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.EmptyState
import com.enn3developer.n_music.ui.components.MenuDivider
import com.enn3developer.n_music.ui.components.MenuItem
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.NMenu
import com.enn3developer.n_music.ui.components.WavyProgress
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.dialogs.AppDialog
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.rememberLibrary
import com.enn3developer.n_music.ui.sheets.Sheet
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NType
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import com.enn3developer.n_music.ui.tracksCount

/** What a source's actions ask of the library. */
interface SourceActions {
    fun open(source: SourceRow)

    /** Reads [root] again, or every source while `null`; [reload] reads every file's tags again. */
    fun scan(root: Locator?, reload: Boolean)

    /** Asks for another name for [source]. */
    fun rename(source: SourceRow)

    fun remove(source: SourceRow)
}

/**
 * The library's sources, local folders, web playlists then Telegram chats, with the scan's
 * progress over them while one runs.
 */
@Composable
fun SourcesPage() {
    val app = LocalApp.current
    val sources by CoreRepository.sources.collectAsStateWithLifecycle()
    val library by CoreRepository.library.collectAsStateWithLifecycle()
    val scan by CoreRepository.scanState.collectAsStateWithLifecycle()
    val telegram by CoreRepository.telegram.collectAsStateWithLifecycle()
    // What a source that can't be reached listed and lacks, for how many tracks it has.
    val down = sources.filter { !it.reachable }.map { it.root }
    val missing = rememberLibrary(emptyMap(), down) { down.associateWith { CoreRepository.missingTracks(it).size } }
    val actions = remember(app) {
        object : SourceActions {
            override fun open(source: SourceRow) = app.open(Page.Source(source.root))

            override fun scan(root: Locator?, reload: Boolean) =
                CoreRepository.send(Command.ScanRequested(root, checkCache = !reload))

            override fun rename(source: SourceRow) = app.show(AppDialog.RenameSource(source.root, source.name))

            override fun remove(source: SourceRow) = app.show(AppDialog.RemoveSource(source.root, source.title, source.tracks))
        }
    }
    // Beside a rail, Settings is on it.
    val settings = { app.open(Page.Settings) }.takeUnless { LocalWindowLayout.current.rail }
    SourcesContent(sources, library.tracks, scan, missing, actions, onSettings = settings, telegram = telegram?.status)
}

/**
 * The page itself: [sources], [tracks] in the whole library, and [scan] while one runs; [missing]
 * counts the tracks unreachable sources listed and can't play. [telegram] is where signing in to
 * Telegram is, `null` without Telegram.
 */
@Composable
fun SourcesContent(
    sources: List<SourceRow>,
    tracks: UInt,
    scan: ScanState?,
    missing: Map<Locator, Int>,
    actions: SourceActions,
    onSettings: (() -> Unit)?,
    telegram: TelegramStatus? = null,
) {
    var menuOpen by remember { mutableStateOf(false) }
    val local = sources.filter { it.root.isLocal }
    val chats = sources.filter { it.root is Locator.TelegramChat }
    val web = sources.filter { !it.root.isLocal && it.root !is Locator.TelegramChat }
    val margins = LocalPageMargins.current
    LazyColumn(
        Modifier.fillMaxSize(),
        // The last row scrolls clear of the add button.
        contentPadding = PaddingValues(bottom = bottomPadding(LocalBottomSpace.current) + 72.dp),
    ) {
        item(key = "bar") {
            Row(
                Modifier
                    .windowInsetsPadding(WindowInsets.statusBars)
                    .fillMaxWidth()
                    .height(64.dp)
                    .padding(horizontal = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.End,
            ) {
                Box {
                    NIconButton(NIcons.More, stringResource(R.string.library_actions), { menuOpen = true }, tint = colors.onSurfaceVariant)
                    NMenu(menuOpen, { menuOpen = false }) {
                        MenuItem(stringResource(R.string.update_library), NIcons.Update, {
                            menuOpen = false
                            actions.scan(null, reload = false)
                        })
                        MenuItem(stringResource(R.string.reload_all_metadata), NIcons.Tag, {
                            menuOpen = false
                            actions.scan(null, reload = true)
                        })
                    }
                }
                if (onSettings != null) {
                    NIconButton(NIcons.Settings, stringResource(R.string.settings), onSettings, tint = colors.onSurfaceVariant)
                }
            }
        }
        item(key = "title") {
            Column(Modifier.padding(start = margins.start, end = margins.end, bottom = 12.dp)) {
                Text(stringResource(R.string.sources_title), style = NType.headline, color = colors.onSurface)
                Text(
                    dotted(
                        pluralStringResource(R.plurals.sources_count, quantity(sources.size), formatCount(sources.size)),
                        tracksCount(tracks),
                    ),
                    style = text(14, tabular = true),
                    color = colors.onSurfaceVariant,
                    modifier = Modifier.padding(top = 2.dp),
                )
            }
        }
        if (scan != null) {
            item(key = "scan") { UpdatingCard(scan, Modifier.padding(start = margins.start, end = margins.end)) }
        }
        if (sources.isEmpty()) {
            item(key = "empty") {
                EmptyState(
                    NIcons.Sources,
                    stringResource(R.string.sources_empty),
                    stringResource(if (telegram != null) R.string.sources_empty_hint_telegram else R.string.sources_empty_hint),
                    Modifier.fillParentMaxHeight(0.6f),
                )
            }
        }
        section(R.string.local_folders, local, scan, missing, telegram, actions)
        section(R.string.web_playlists, web, scan, missing, telegram, actions)
        section(R.string.telegram_chats, chats, scan, missing, telegram, actions)
    }
}

/** A kind of source: its name and how many, then each, grouped as one card. */
private fun LazyListScope.section(
    title: Int,
    sources: List<SourceRow>,
    scan: ScanState?,
    missing: Map<Locator, Int>,
    telegram: TelegramStatus?,
    actions: SourceActions,
) {
    if (sources.isEmpty()) return
    item(key = "section $title") {
        val margins = LocalPageMargins.current
        Row(
            Modifier.padding(start = margins.start + 4.dp, end = margins.end + 4.dp, top = 18.dp, bottom = 8.dp),
            verticalAlignment = Alignment.Bottom,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(stringResource(title), style = text(14, FontWeight.Bold), color = colors.onSurface)
            Text(formatCount(sources.size), style = text(13, tabular = true), color = colors.onSurfaceVariant)
        }
    }
    itemsIndexed(sources, key = { _, source -> "source " + source.root.hashCode() }) { index, source ->
        val margins = LocalPageMargins.current
        SourceItem(
            source = source,
            shape = groupShape(index, sources.size),
            updating = scan?.libraries?.contains(source.root) == true,
            signedOut = signedOut(source.root, telegram),
            tracks = source.tracks.toLong() + (missing[source.root] ?: 0),
            actions = actions,
            modifier = Modifier.padding(start = margins.start, end = margins.end, top = if (index > 0) 2.dp else 0.dp),
        )
    }
}

/** A card in a group of [count]: round at the group's ends, nearly square between. */
fun groupShape(index: Int, count: Int, outer: Dp = 20.dp, inner: Dp = 4.dp): RoundedCornerShape =
    RoundedCornerShape(
        topStart = if (index == 0) outer else inner,
        topEnd = if (index == 0) outer else inner,
        bottomStart = if (index == count - 1) outer else inner,
        bottomEnd = if (index == count - 1) outer else inner,
    )

/** The scan's card: Updating library, how far it got, and its wave. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun UpdatingCard(scan: ScanState, modifier: Modifier = Modifier) {
    Column(
        modifier
            .fillMaxWidth()
            .background(colors.surfaceLow, RoundedCornerShape(20.dp))
            .padding(horizontal = 16.dp, vertical = 14.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            LoadingIndicator(Modifier.size(36.dp), color = colors.primary)
            Text(
                stringResource(R.string.updating_library),
                style = text(15, FontWeight.Bold),
                color = colors.onSurface,
                modifier = Modifier.weight(1f),
            )
            Text(
                stringResource(R.string.read_of_found, formatCount(scan.read), formatCount(scan.found)),
                style = text(13, tabular = true),
                color = colors.onSurfaceVariant,
            )
        }
        WavyProgress({ scan.progress }, Modifier.fillMaxWidth().height(12.dp))
    }
}

/**
 * A source's card: its cover, name with what it is going through, how many tracks it has and
 * where it is, and its actions under ⋮. One that can't be reached [signedOut] of Telegram says
 * that instead.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun SourceItem(
    source: SourceRow,
    shape: RoundedCornerShape,
    updating: Boolean,
    signedOut: Boolean,
    tracks: Long,
    actions: SourceActions,
    modifier: Modifier = Modifier,
) {
    var menuOpen by remember { mutableStateOf(false) }
    val name = source.title
    Row(
        modifier
            .fillMaxWidth()
            .height(76.dp)
            .clip(shape)
            .background(colors.surfaceLow)
            .padding(end = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Row(
            Modifier
                .weight(1f)
                .height(76.dp)
                .tappable({ actions.open(source) })
                .padding(start = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            SourceTile(source, 52.dp, Modifier.graphicsLayer { alpha = if (source.reachable) 1f else 0.5f })
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        name,
                        style = text(16, FontWeight.Bold),
                        color = colors.onSurface,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.weight(1f, fill = false),
                    )
                    when {
                        updating -> StatusChip(stringResource(R.string.source_updating), colors.secondaryContainer, colors.onSecondaryContainer) {
                            LoadingIndicator(Modifier.size(16.dp), color = colors.onSecondaryContainer)
                        }
                        !source.reachable -> StatusChip(
                            stringResource(if (signedOut) R.string.source_signed_out_short else R.string.source_unreachable),
                            colors.errorContainer,
                            colors.onErrorContainer,
                        ) {
                            NIcon(NIcons.AlertBold, size = 14.dp, tint = colors.onErrorContainer)
                        }
                    }
                }
                Text(
                    tracksCount(tracks),
                    style = text(13, tabular = true),
                    color = colors.onSurfaceVariant,
                    maxLines = 1,
                    modifier = Modifier.padding(top = 2.dp),
                )
                Text(
                    sourcePlace(source.root),
                    style = text(13),
                    color = colors.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        Box {
            NIconButton(
                NIcons.More,
                stringResource(R.string.more_for_source, name),
                { menuOpen = true },
                size = 44.dp,
                iconSize = 20.dp,
                tint = colors.onSurfaceVariant,
            )
            NMenu(menuOpen, { menuOpen = false }) {
                // The list has no pencil by the name, as a source's page has: its menu renames it.
                SourceMenu(source, actions, rename = true) { menuOpen = false }
            }
        }
    }
}

/** What ⋮ offers for one source: read it again, [rename] it, or take it out of the library. */
@Composable
fun SourceMenu(source: SourceRow, actions: SourceActions, rename: Boolean = false, close: () -> Unit) {
    MenuItem(stringResource(R.string.update_now), NIcons.Update, {
        close()
        actions.scan(source.root, reload = false)
    })
    MenuItem(stringResource(R.string.reload_metadata), NIcons.Tag, {
        close()
        actions.scan(source.root, reload = true)
    })
    if (rename) {
        MenuItem(stringResource(R.string.rename), NIcons.Rename, {
            close()
            actions.rename(source)
        })
    }
    MenuDivider()
    MenuItem(stringResource(R.string.remove_from_library), NIcons.RemoveSource, {
        close()
        actions.remove(source)
    }, danger = true)
}

/** A small pill by a source's name telling what it is going through. */
@Composable
private fun StatusChip(label: String, container: Color, content: Color, icon: @Composable () -> Unit) {
    Row(
        Modifier
            .height(22.dp)
            .background(container, RoundedCornerShape(11.dp))
            .padding(start = 6.dp, end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        icon()
        Text(label, style = text(12, FontWeight.Bold), color = content, maxLines = 1)
    }
}

/** A source's tile at [size]: its cover, or what kind of source it is while it has none. */
@Composable
fun SourceTile(source: SourceRow, size: Dp, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(if (size > 64.dp) 20.dp else 12.dp)
    if (source.cover != null) {
        Cover(source.cover, modifier.size(size), shape = shape)
    } else {
        Box(
            modifier
                .size(size)
                .background(colors.surfaceHigh, shape),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(
                sourceIcon(source.root),
                size = if (size > 64.dp) 44.dp else 24.dp,
                tint = colors.onSurfaceVariant,
            )
        }
    }
}
