package com.enn3developer.n_music.ui.playlists

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.VisibilityThreshold
import androidx.compose.foundation.BorderStroke
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
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.LineBreak
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Filter
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.PlaylistRow
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.core.SourceRow
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.core.defaultSourceName
import com.enn3developer.n_music.key
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.LocalBottomSpace
import com.enn3developer.n_music.ui.Origin
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.PlaylistTrack
import com.enn3developer.n_music.ui.Selection
import com.enn3developer.n_music.ui.Snack
import com.enn3developer.n_music.ui.bottomPadding
import com.enn3developer.n_music.ui.components.ButtonSurface
import com.enn3developer.n_music.ui.components.EmptyState
import com.enn3developer.n_music.ui.components.MenuDivider
import com.enn3developer.n_music.ui.components.MenuItem
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.NMenu
import com.enn3developer.n_music.ui.components.PageBar
import com.enn3developer.n_music.ui.components.PlayShuffle
import com.enn3developer.n_music.ui.components.SelectionBar
import com.enn3developer.n_music.ui.components.SortControl
import com.enn3developer.n_music.ui.components.SwipeToRemove
import com.enn3developer.n_music.ui.components.Tab
import com.enn3developer.n_music.ui.components.TrackItem
import com.enn3developer.n_music.ui.components.barSwap
import com.enn3developer.n_music.ui.dialogs.AppDialog
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.enqueue
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.formatDuration
import com.enn3developer.n_music.ui.formatLength
import com.enn3developer.n_music.ui.formatWhen
import com.enn3developer.n_music.ui.library.FilterField
import com.enn3developer.n_music.ui.library.NowPlaying
import com.enn3developer.n_music.ui.library.TrackFilters
import com.enn3developer.n_music.ui.library.TrackOrder
import com.enn3developer.n_music.ui.library.TrackSort
import com.enn3developer.n_music.ui.library.label
import com.enn3developer.n_music.ui.library.rememberNowPlaying
import com.enn3developer.n_music.ui.playsCount
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.rememberLibrary
import com.enn3developer.n_music.ui.rememberLibraryRead
import com.enn3developer.n_music.ui.removeFromPlaylist
import com.enn3developer.n_music.ui.sheets.Sheet
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.NShapes
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import com.enn3developer.n_music.ui.tracksCount

/** The order a playlist's tracks show in: its own, or by default the newest or most played first. */
fun playlistOrder(playlist: PlaylistRow): TrackOrder =
    if (playlist.sort.isEmpty() && playlist.rule != null) TrackOrder(TrackSort.MOST_PLAYED) else TrackOrder.of(playlist.sort)

/**
 * A playlist's page: its tile, name and details, Play and Shuffle, and its tracks in its own
 * order. A smart one shows its rules as chips, or listed when only the desktop can edit them; a
 * plain one lets a track be swiped out of it.
 */
@Composable
fun PlaylistPage(page: Page.Playlist) {
    val app = LocalApp.current
    val resources = LocalResources.current
    val playlist = rememberLibrary<PlaylistRow?>(null, page.id) { CoreRepository.playlist(page.id) }
    val order = playlist?.let(::playlistOrder) ?: TrackOrder(TrackSort.ADDED)
    val query = Query(Filter.Playlist(page.id), order.keys(page.id))
    val read = rememberLibraryRead(query) { CoreRepository.tracks(query) }
    // Tracks swiped out leave at once, though the core hears of it once their snackbar goes.
    val tracks = read?.let { (rows, version) ->
        rows.filterNot { app.removals.hides(PlaylistTrack(page.id, it.locator), version) }
    }
    val origin = Origin.Playlist(page.id)
    val selection = app.selection
    BackHandler(selection != null) { app.endSelection() }

    fun queue(next: Boolean) {
        val all = tracks.orEmpty().map { it.locator }
        if (all.isEmpty()) return
        val undo = enqueue(all, next)
        val count = all.size
        app.snack(
            Snack(
                resources.getQuantityString(if (next) R.plurals.queued_next else R.plurals.queued, quantity(count), formatCount(count)),
                resources.getString(R.string.undo),
                undo,
            )
        )
    }

    PlaylistContent(
        playlist = playlist,
        tracks = tracks,
        order = order,
        nowPlaying = rememberNowPlaying(),
        selection = selection,
        held = (app.sheet as? Sheet.TrackActions)?.track,
        sources = CoreRepository.sources.collectAsStateWithLifecycle().value,
        onBack = app::back,
        onRename = { playlist?.let { app.show(AppDialog.RenamePlaylist(it.id, it.name)) } },
        onRules = { field -> playlist?.let { app.show(Sheet.SmartPlaylist(it.id, field)) } },
        onPlay = { app.play(query, origin, it.locator) },
        onPlayAll = { shuffle -> app.play(query, origin, shuffle = shuffle) },
        onSort = { app.show(Sheet.PlaylistSort(page.id)) },
        onSelect = { app.select(it.locator) },
        onEndSelection = app::endSelection,
        // A plain playlist's tracks can leave it from their menu too.
        onMore = { app.show(Sheet.TrackActions(it.locator, page.id.takeIf { playlist?.rule == null })) },
        onRemove = { app.removeFromPlaylist(page.id, it, resources) },
        onLibrary = { app.navigator.home(Tab.LIBRARY) },
        menu = { close ->
            if (!tracks.isNullOrEmpty()) {
                MenuItem(stringResource(R.string.play_next), NIcons.PlayNext, { close(); queue(next = true) })
                MenuItem(stringResource(R.string.add_to_queue), NIcons.AddToQueue, { close(); queue(next = false) })
                MenuDivider()
            }
            MenuItem(stringResource(R.string.delete_playlist), NIcons.Remove, {
                close()
                playlist?.let { app.show(AppDialog.DeletePlaylist(it.id, it.name)) }
            }, danger = true)
        },
    )
}

/** The playlist page itself, for [playlist] as it is read; [menu] fills its ⋮ menu. */
@Composable
fun PlaylistContent(
    playlist: PlaylistRow?,
    tracks: List<TrackRow>?,
    order: TrackOrder,
    nowPlaying: NowPlaying,
    selection: Selection?,
    held: Locator?,
    sources: List<SourceRow>,
    onBack: () -> Unit,
    onRename: () -> Unit,
    onRules: (FilterField?) -> Unit,
    onPlay: (TrackRow) -> Unit,
    onPlayAll: (shuffle: Boolean) -> Unit,
    onSort: () -> Unit,
    onSelect: (TrackRow) -> Unit,
    onEndSelection: () -> Unit,
    onMore: (TrackRow) -> Unit,
    onRemove: (TrackRow) -> Unit,
    onLibrary: () -> Unit,
    menu: @Composable (close: () -> Unit) -> Unit,
) {
    val haptics = LocalHapticFeedback.current
    val rows = tracks.orEmpty()
    val bottom = bottomPadding(LocalBottomSpace.current)
    val smart = playlist?.rule != null
    // Rules the filter sections can show, or null when only the desktop edits them.
    val rules = remember(playlist?.rule) { playlist?.rule?.let(::rulesOf) }
    var menuOpen by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize()) {
        AnimatedContent(selection != null, transitionSpec = { barSwap(targetState) }, label = "bar") { selecting ->
            if (selecting) {
                SelectionBar(selection?.count ?: 0, rows.size, onEndSelection, { selection?.addAll(rows.map { it.locator }) })
            } else {
                PageBar(onBack) {
                    Box {
                        NIconButton(NIcons.More, stringResource(R.string.more_playlist_actions), { menuOpen = true }, tint = colors.onSurface)
                        NMenu(menuOpen, { menuOpen = false }) { menu { menuOpen = false } }
                    }
                }
            }
        }
        // With nothing in it, what to do about that takes the room under the header.
        if (playlist != null && tracks != null && rows.isEmpty()) {
            Header(playlist, smart, onRename)
            if (smart) Rules(playlist, rules, sources, onRules)
            if (smart) {
                EmptyState(
                    NIcons.Filter,
                    stringResource(R.string.smart_empty),
                    stringResource(R.string.smart_empty_hint),
                    Modifier.weight(1f),
                    bottom = 60.dp,
                )
            } else {
                EmptyState(
                    NIcons.Playlist,
                    stringResource(R.string.playlist_empty),
                    stringResource(R.string.playlist_empty_hint),
                    Modifier.weight(1f),
                    action = stringResource(R.string.go_to_library),
                    tonal = true,
                    bottom = 60.dp,
                    onAction = onLibrary,
                )
            }
            return@Column
        }
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = bottom)) {
            item(key = "header") {
                if (playlist != null) Header(playlist, smart, onRename)
            }
            if (playlist != null && smart) {
                item(key = "rules") { Rules(playlist, rules, sources, onRules) }
            }
            if (rows.isNotEmpty()) {
                item(key = "play") {
                    PlayShuffle(
                        onPlay = { onPlayAll(false) },
                        onShuffle = { onPlayAll(true) },
                        modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = if (smart) 14.dp else 16.dp),
                    )
                }
                item(key = "sort") {
                    Row(
                        Modifier.padding(start = 6.dp, end = 16.dp, top = if (smart) 6.dp else 8.dp, bottom = if (smart) 4.dp else 2.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        SortControl(stringResource(order.sort.label), onSort)
                    }
                }
            }
            items(rows, key = { it.locator.key }) { track ->
                val state = nowPlaying.state(track).copy(
                    selected = selection?.contains(track.locator) == true,
                    held = track.locator == held,
                )
                val item: @Composable () -> Unit = {
                    TrackItem(
                        track,
                        state,
                        onClick = { if (selection != null) onSelect(track) else onPlay(track) },
                        onLongClick = {
                            if (selection == null) haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                            onSelect(track)
                        },
                        onMore = { onMore(track) },
                        line = track.artist.ifEmpty { stringResource(R.string.unknown_artist) },
                        trailing = if (order.sort == TrackSort.MOST_PLAYED) playsCount(track.plays) else formatLength(track.length),
                    )
                }
                Box(Modifier.animateItem(fadeInSpec = null, placementSpec = NMotion.spatialDefault(IntOffset.VisibilityThreshold))) {
                    if (smart || selection != null) {
                        item()
                    } else {
                        SwipeToRemove({ onRemove(track) }, label = stringResource(R.string.remove_from_playlist), content = item)
                    }
                }
            }
        }
    }
}

/** A smart playlist's [rules] as chips, or listed when only the desktop app can edit them. */
@Composable
private fun Rules(playlist: PlaylistRow, rules: TrackFilters?, sources: List<SourceRow>, onRules: (FilterField?) -> Unit) {
    if (rules != null) RuleChips(rules, sources, onRules) else LockedRules(playlist.rule!!)
}

/** The tile, kind, name with its pencil, and what the playlist holds. */
@Composable
private fun Header(playlist: PlaylistRow, smart: Boolean, onRename: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(start = 16.dp, end = 16.dp, top = 4.dp),
        verticalAlignment = Alignment.Bottom,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        PlaylistTile(playlist, 104.dp)
        Column(
            Modifier
                .weight(1f)
                .padding(bottom = 2.dp)
        ) {
            Text(
                stringResource(if (smart) R.string.smart_playlist_kind else R.string.playlist_kind),
                style = text(13, FontWeight.Bold),
                color = colors.onSurfaceVariant,
            )
            Row(
                Modifier.padding(top = 2.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                // 30 sp apart on two lines; on one, the pencil's 40 dp sets the row's height.
                Text(
                    playlist.name,
                    style = text(26, FontWeight.ExtraBold, 30.sp, (-0.4).sp).copy(lineBreak = LineBreak.Heading),
                    color = colors.onSurface,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f, fill = false),
                )
                NIconButton(
                    NIcons.Rename,
                    stringResource(R.string.rename_playlist),
                    onRename,
                    size = 40.dp,
                    iconSize = 18.dp,
                    tint = colors.onSurfaceVariant,
                )
            }
            Text(
                listOf(
                    if (playlist.tracks == 0u) {
                        stringResource(R.string.playlist_no_tracks)
                    } else {
                        dotted(tracksCount(playlist.tracks), formatDuration(playlist.length))
                    },
                    when {
                        smart -> stringResource(R.string.smart_updates)
                        playlist.tracks == 0u -> stringResource(R.string.playlist_made, formatWhen(playlist.created))
                        else -> stringResource(R.string.playlist_changed, formatWhen(playlist.modified))
                    },
                ).joinToString("\n"),
                style = text(13, lineHeight = 18.sp, tabular = true),
                color = colors.onSurfaceVariant,
                modifier = Modifier.padding(top = 2.dp),
            )
        }
    }
}

/**
 * A smart playlist's rules as chips, each opening the rules at its section: how many are set,
 * then the set ones, then the rest, but for the artist, picked from a list, and the source while
 * there is one.
 */
@Composable
private fun RuleChips(rules: TrackFilters, sources: List<SourceRow>, onOpen: (FilterField?) -> Unit) {
    val active = rules.active
    val unset = FilterField.entries.filter {
        it !in active && it != FilterField.ARTIST && (it != FilterField.SOURCE || sources.size > 1)
    }
    val sourceName = { root: Locator -> sources.find { it.root == root }?.name ?: defaultSourceName(root) }
    val description = pluralStringResource(R.plurals.edit_rules, active.size, active.size)
    LazyRow(
        Modifier
            .fillMaxWidth()
            .padding(top = 16.dp),
        contentPadding = PaddingValues(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        item(key = "count") {
            ButtonSurface(
                onClick = { onOpen(null) },
                shape = NShapes.chip,
                container = colors.secondaryContainer,
                content = colors.onSecondaryContainer,
                modifier = Modifier
                    .height(32.dp)
                    .semantics { contentDescription = description },
                padding = PaddingValues(start = 8.dp, end = 10.dp),
                arrangement = Arrangement.spacedBy(4.dp),
            ) {
                NIcon(NIcons.Filter, size = 18.dp)
                Text(active.size.toString(), style = text(14, FontWeight.Bold, tabular = true))
            }
        }
        items(active, key = { "set " + it.name }) { field ->
            RuleChip(rules.label(field, sourceName), set = true) { onOpen(field) }
        }
        items(unset, key = { "unset " + it.name }) { field ->
            RuleChip(stringResource(field.label), set = false) { onOpen(field) }
        }
    }
}

/** A rule's chip: filled while set, outlined while not, with the chevron of a picker. */
@Composable
private fun RuleChip(label: String, set: Boolean, onClick: () -> Unit) {
    ButtonSurface(
        onClick = onClick,
        shape = NShapes.chip,
        container = if (set) colors.secondaryContainer else Color.Transparent,
        content = if (set) colors.onSecondaryContainer else colors.onSurface,
        border = if (set) null else BorderStroke(1.dp, colors.outlineVariant),
        modifier = Modifier.height(32.dp),
        padding = PaddingValues(start = 12.dp, end = 6.dp),
        arrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(label, style = text(14, if (set) FontWeight.Bold else FontWeight.SemiBold), maxLines = 1)
        NIcon(NIcons.Collapse, size = 18.dp, tint = if (set) colors.onSecondaryContainer else colors.onSurfaceVariant)
    }
}

/** Rules only the desktop app can edit, listed in words with a lock. */
@Composable
private fun LockedRules(rule: Filter) {
    Column(
        Modifier
            .fillMaxWidth()
            .padding(start = 16.dp, end = 16.dp, top = 16.dp)
            .background(colors.surfaceLow, RoundedCornerShape(20.dp))
            .padding(start = 16.dp, end = 16.dp, top = 14.dp, bottom = 14.dp)
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(
                stringResource(if (matchesAll(rule)) R.string.rules_match_all else R.string.rules_match_any),
                style = text(14, FontWeight.Bold),
                color = colors.onSurface,
                modifier = Modifier.weight(1f),
            )
            NIcon(NIcons.Locked, size = 18.dp, tint = colors.onSurfaceVariant)
        }
        Column(Modifier.padding(top = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            for (line in ruleLines(rule)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Box(
                        Modifier
                            .size(6.dp)
                            .background(colors.primary, CircleShape)
                    )
                    Text(line, style = text(15, lineHeight = 20.sp), color = colors.onSurface)
                }
            }
        }
        Text(
            stringResource(R.string.rules_locked),
            style = text(13, lineHeight = 18.sp),
            color = colors.onSurfaceVariant,
            modifier = Modifier.padding(top = 10.dp),
        )
    }
}
