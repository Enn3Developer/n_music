package com.enn3developer.n_music.ui.sources

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.LineBreak
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ScanState
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Filter
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.core.SourceRow
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.key
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.LocalBottomSpace
import com.enn3developer.n_music.ui.LocalPageMargins
import com.enn3developer.n_music.ui.Origin
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.bottomPadding
import com.enn3developer.n_music.ui.components.ButtonSurface
import com.enn3developer.n_music.ui.components.EmptyState
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.NMenu
import com.enn3developer.n_music.ui.components.PageBar
import com.enn3developer.n_music.ui.components.PlayShuffle
import com.enn3developer.n_music.ui.components.SortControl
import com.enn3developer.n_music.ui.components.TrackItem
import com.enn3developer.n_music.ui.components.trackLine
import com.enn3developer.n_music.ui.dialogs.AppDialog
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.formatDuration
import com.enn3developer.n_music.ui.library.NowPlaying
import com.enn3developer.n_music.ui.library.SortedList
import com.enn3developer.n_music.ui.library.TrackOrder
import com.enn3developer.n_music.ui.library.label
import com.enn3developer.n_music.ui.library.rememberNowPlaying
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.rememberLibrary
import com.enn3developer.n_music.ui.sheets.Sheet
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import com.enn3developer.n_music.ui.tracksCount

/**
 * A source's page: its cover, kind, name and where it is, then its tracks. A web playlist that
 * can't be reached says so, plays what is saved on this phone, and lists the rest greyed.
 */
@Composable
fun SourcePage(page: Page.Source) {
    val app = LocalApp.current
    val ui by UiPreferences.settings.collectAsStateWithLifecycle()
    val sources by CoreRepository.sources.collectAsStateWithLifecycle()
    val scan by CoreRepository.scanState.collectAsStateWithLifecycle()
    val source = sources.find { it.root == page.root }
    val order = SortedList.SOURCE_TRACKS.trackOrder(ui.sorts)
    val query = Query(Filter.Library(page.root), order.keys())
    val tracks = rememberLibrary<List<TrackRow>?>(null, query) { CoreRepository.tracks(query) }
    val down = source?.reachable == false
    val missing = rememberLibrary(emptyList(), page.root, down) {
        if (down) CoreRepository.missingTracks(page.root) else emptyList()
    }
    val origin = Origin.Source(page.root)
    val actions = remember(app) {
        object : SourceActions {
            override fun open(source: SourceRow) {}

            override fun scan(root: Locator?, reload: Boolean) =
                CoreRepository.send(Command.ScanRequested(root, checkCache = !reload))

            override fun remove(source: SourceRow) = app.show(AppDialog.RemoveSource(source.root, source.title, source.tracks))
        }
    }
    SourceContent(
        source = source,
        tracks = tracks,
        missing = missing,
        order = order,
        nowPlaying = rememberNowPlaying(),
        scan = scan?.takeIf { page.root in it.libraries },
        held = (app.sheet as? Sheet.TrackActions)?.track,
        actions = actions,
        onBack = app::back,
        onRename = { source?.let { app.show(AppDialog.RenameSource(it.root, it.name)) } },
        onPlay = { app.play(query, origin, it.locator) },
        onPlayAll = { shuffle -> app.play(query, origin, shuffle = shuffle) },
        onSort = { app.show(Sheet.Sort(SortedList.SOURCE_TRACKS)) },
        onMore = { app.show(Sheet.TrackActions(it.locator)) },
        compact = ui.compactRows,
    )
}

/**
 * The page itself, for [source] as the library has it: [tracks] it plays, and [missing], those a
 * web playlist that can't be reached listed and can't play; [scan] while one reads it.
 */
@Composable
fun SourceContent(
    source: SourceRow?,
    tracks: List<TrackRow>?,
    missing: List<TrackRow>,
    order: TrackOrder,
    nowPlaying: NowPlaying,
    scan: ScanState?,
    held: Locator?,
    actions: SourceActions,
    onBack: () -> Unit,
    onRename: () -> Unit,
    onPlay: (TrackRow) -> Unit,
    onPlayAll: (shuffle: Boolean) -> Unit,
    onSort: () -> Unit,
    onMore: (TrackRow) -> Unit,
    compact: Boolean = false,
) {
    val rows = tracks.orEmpty()
    val margins = LocalPageMargins.current
    val down = source?.reachable == false
    var menuOpen by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize()) {
        PageBar(onBack) {
            if (source != null) {
                Box {
                    NIconButton(
                        NIcons.More,
                        stringResource(R.string.more_for_source, source.title),
                        { menuOpen = true },
                        tint = colors.onSurface,
                    )
                    NMenu(menuOpen, { menuOpen = false }) { SourceMenu(source, actions) { menuOpen = false } }
                }
            }
        }
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = bottomPadding(LocalBottomSpace.current))) {
            if (source == null) return@LazyColumn
            // What it listed: saved tracks and missing ones alike, for one that can't be reached.
            item(key = "header") { Header(source, source.tracks.toLong() + missing.size, onRename) }
            if (down) {
                item(key = "down") {
                    Unreachable(source.root, rows.size) { actions.scan(source.root, reload = false) }
                }
            } else if (scan != null && rows.isEmpty()) {
                item(key = "scan") { UpdatingCard(scan, Modifier.padding(start = margins.start, end = margins.end, top = 16.dp)) }
            }
            if (rows.isNotEmpty()) {
                item(key = "play") {
                    val count = rows.size.toLong()
                    PlayShuffle(
                        onPlay = { onPlayAll(false) },
                        onShuffle = { onPlayAll(true) },
                        modifier = Modifier.padding(start = margins.start, end = margins.end, top = if (down) 14.dp else 16.dp),
                        playDescription = if (down) {
                            pluralStringResource(R.plurals.play_saved, quantity(count), formatCount(count))
                        } else {
                            null
                        },
                        shuffleDescription = if (down) {
                            pluralStringResource(R.plurals.shuffle_saved, quantity(count), formatCount(count))
                        } else {
                            null
                        },
                    )
                }
                if (down) {
                    item(key = "gap") { Box(Modifier.padding(top = 6.dp)) }
                } else {
                    item(key = "sort") {
                        Row(Modifier.padding(start = margins.startLess(10.dp), end = margins.end, top = 8.dp, bottom = 2.dp)) {
                            SortControl(stringResource(order.sort.label), onSort)
                        }
                    }
                }
            } else if (tracks != null && missing.isEmpty() && scan == null) {
                item(key = "empty") {
                    EmptyState(
                        if (source.root.isLocal) NIcons.Sources else NIcons.Web,
                        stringResource(R.string.source_empty),
                        stringResource(R.string.source_empty_hint),
                        Modifier.fillParentMaxHeight(0.6f),
                    )
                }
            }
            items(rows, key = { it.locator.key }) { track ->
                TrackItem(
                    track,
                    nowPlaying.state(track).copy(held = track.locator == held),
                    onClick = { onPlay(track) },
                    onLongClick = null,
                    onMore = { onMore(track) },
                    line = if (down) dotted(artist(track), stringResource(R.string.saved_on_phone)) else trackLine(track),
                    quiet = down,
                    compact = compact,
                )
            }
            items(missing, key = { "missing " + it.locator.key }) { track ->
                // Not in the library while away, it has nothing to play or open.
                TrackItem(
                    track,
                    nowPlaying.state(track),
                    onClick = {},
                    onLongClick = null,
                    onMore = null,
                    line = dotted(artist(track), stringResource(R.string.not_saved)),
                    muted = true,
                    compact = compact,
                    cover = { modifier ->
                        Box(
                            modifier
                                .size(if (compact) 40.dp else 48.dp)
                                .background(colors.surfaceHigh, RoundedCornerShape(8.dp)),
                            contentAlignment = Alignment.Center,
                        ) {
                            NIcon(NIcons.Later, size = 22.dp, tint = colors.onSurfaceQuiet)
                        }
                    },
                )
            }
        }
    }
}

@Composable
private fun artist(track: TrackRow): String = track.artist.ifEmpty { stringResource(R.string.unknown_artist) }

/** The tile, kind, name with its pencil, how much it holds and where it is. */
@Composable
private fun Header(source: SourceRow, tracks: Long, onRename: () -> Unit) {
    val margins = LocalPageMargins.current
    val down = !source.reachable
    Row(
        Modifier
            .fillMaxWidth()
            .padding(start = margins.start, end = margins.end, top = 4.dp),
        verticalAlignment = Alignment.Bottom,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        SourceTile(source, 104.dp)
        Column(Modifier.weight(1f)) {
            Text(
                stringResource(if (source.root.isLocal) R.string.local_folder else R.string.web_playlist),
                style = text(13, FontWeight.Bold),
                color = colors.onSurfaceVariant,
            )
            Row(
                Modifier.padding(top = 2.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Text(
                    source.title,
                    style = text(26, FontWeight.ExtraBold, 30.sp, (-0.4).sp).copy(lineBreak = LineBreak.Heading),
                    color = colors.onSurface,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f, fill = false),
                )
                NIconButton(
                    NIcons.Rename,
                    stringResource(R.string.rename_source),
                    onRename,
                    size = 40.dp,
                    iconSize = 18.dp,
                    tint = colors.onSurfaceVariant,
                )
            }
            // One that can't be reached has no length to tell: its address is all.
            val count = tracksCount(tracks)
            Text(
                if (down) {
                    dotted(count, sourcePlace(source.root))
                } else {
                    dotted(count, formatDuration(source.length)) + "\n" + sourcePlace(source.root)
                },
                style = text(13, lineHeight = 18.sp, tabular = true),
                color = colors.onSurfaceVariant,
                maxLines = if (down) 1 else 2,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(top = 2.dp),
            )
        }
    }
}

/**
 * What a web playlist that can't be reached says: who didn't answer, what still plays, and a way
 * to try again.
 */
@Composable
private fun Unreachable(root: Locator, saved: Int, onRetry: () -> Unit) {
    val margins = LocalPageMargins.current
    val host = (root as? Locator.Web)?.v1?.let(::webHost) ?: sourcePlace(root)
    Column(
        Modifier
            .padding(start = margins.start, end = margins.end, top = 16.dp)
            .fillMaxWidth()
            .background(colors.errorContainer, RoundedCornerShape(20.dp))
            .semantics { liveRegion = LiveRegionMode.Polite }
            .padding(horizontal = 16.dp, vertical = 14.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            NIcon(NIcons.Info, size = 20.dp, tint = colors.onErrorContainer)
            Text(stringResource(R.string.source_unreachable), style = text(15, FontWeight.Bold), color = colors.onErrorContainer)
        }
        Text(
            if (saved == 0) {
                stringResource(R.string.source_unreachable_none, host)
            } else {
                pluralStringResource(R.plurals.source_unreachable_hint, saved, host, formatCount(saved))
            },
            style = text(14, lineHeight = 20.sp),
            color = colors.onErrorContainer,
            modifier = Modifier.padding(top = 6.dp),
        )
        ButtonSurface(
            onClick = onRetry,
            shape = RoundedCornerShape(20.dp),
            container = colors.onErrorContainer,
            content = colors.errorContainer,
            modifier = Modifier
                .padding(top = 12.dp)
                .height(40.dp),
            padding = PaddingValues(horizontal = 16.dp),
        ) {
            Text(stringResource(R.string.try_again), style = text(14, FontWeight.Bold))
        }
    }
}
