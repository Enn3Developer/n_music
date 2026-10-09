package com.enn3developer.n_music.ui.components

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ScanState
import com.enn3developer.n_music.core.PlaylistRow
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** The drawer's width, beside the pages or over them. */
val DrawerWidth = 248.dp

/** The rail and the drawer hold off the cutout and the buttons of a phone held sideways. */
private val StartInsets: WindowInsets
    @Composable get() = WindowInsets.displayCutout.union(WindowInsets.navigationBars).only(WindowInsetsSides.Start)

/** Where the app is, for the rail and the drawer to mark: one of the [Tab]s, or Settings. */
data class NavPlace(val tab: Tab, val settings: Boolean)

/** What the rail and the drawer ask for. */
interface NavActions {
    fun select(tab: Tab)

    fun openSettings()

    fun openPlaylist(id: Long)

    fun newPlaylist()

    /** Opens the sources, from the scan's note. */
    fun openScan()
}

/**
 * The rail of foldables and tablets: ☰, the [Tab]s, and Settings at its foot. A foldable's sits
 * on a surface of its own; a tablet's on the page.
 */
@Composable
fun NavRail(
    place: NavPlace,
    sourcesBusy: Boolean,
    actions: NavActions,
    onMenu: () -> Unit,
    modifier: Modifier = Modifier,
    tablet: Boolean = false,
) {
    Column(
        modifier
            .fillMaxHeight()
            .then(if (tablet) Modifier else Modifier.background(colors.surfaceLow))
            .windowInsetsPadding(StartInsets)
            .windowInsetsPadding(WindowInsets.statusBars)
            .windowInsetsPadding(WindowInsets.navigationBars.only(WindowInsetsSides.Bottom))
            .width(if (tablet) 96.dp else 88.dp)
            .padding(top = if (tablet) 8.dp else 16.dp, bottom = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(if (tablet) 14.dp else 12.dp),
    ) {
        val menu = stringResource(R.string.open_navigation)
        Box(
            Modifier
                .padding(bottom = if (tablet) 10.dp else 12.dp)
                .width(56.dp)
                .height(56.dp)
                .clip(RoundedCornerShape(16.dp))
                .clickable(role = Role.Button, onClick = onMenu)
                .semantics { contentDescription = menu },
            contentAlignment = Alignment.Center,
        ) {
            NIcon(NIcons.Menu, tint = colors.onSurfaceVariant)
        }
        for (tab in Tab.entries) {
            NavItem(
                tab.icon,
                stringResource(tab.label),
                selected = !place.settings && place.tab == tab,
                onClick = { actions.select(tab) },
                badge = tab == Tab.SOURCES && sourcesBusy,
                modifier = Modifier.fillMaxWidth(),
            )
        }
        Spacer(Modifier.weight(1f))
        NavItem(
            NIcons.Settings,
            stringResource(R.string.settings),
            selected = place.settings,
            onClick = actions::openSettings,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

/**
 * The drawer: the app's name, the [Tab]s, the [playlists] with the one [playingFrom] marked,
 * the [scan]'s progress while one runs, and Settings. [onClose] folds it back into the rail, or
 * closes it over the pages.
 */
@Composable
fun NavDrawer(
    place: NavPlace,
    playlists: List<PlaylistRow>,
    playingFrom: Long?,
    playing: Boolean,
    scan: ScanState?,
    actions: NavActions,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
    closeLabel: String = stringResource(R.string.collapse_navigation),
) {
    Column(
        modifier
            .fillMaxHeight()
            .windowInsetsPadding(StartInsets)
            .windowInsetsPadding(WindowInsets.statusBars)
            .windowInsetsPadding(WindowInsets.navigationBars.only(WindowInsetsSides.Bottom))
            .width(DrawerWidth)
            .padding(horizontal = 12.dp),
    ) {
        Row(
            Modifier
                .height(56.dp)
                .padding(start = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            NIconButton(NIcons.MenuOpen, closeLabel, onClose, tint = colors.onSurfaceVariant)
            Text(
                stringResource(R.string.app_name),
                style = text(18, FontWeight.ExtraBold, letterSpacing = (-0.2).sp),
                color = colors.onSurface,
            )
        }
        Column(Modifier.padding(top = 8.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            for (tab in Tab.entries) {
                DrawerItem(
                    tab.icon,
                    stringResource(tab.label),
                    selected = !place.settings && place.tab == tab,
                    onClick = { actions.select(tab) },
                )
            }
        }
        Row(
            Modifier
                .fillMaxWidth()
                .padding(top = 18.dp, bottom = 4.dp, start = 16.dp, end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                stringResource(R.string.nav_playlists),
                style = text(13, FontWeight.Bold),
                color = colors.onSurfaceVariant,
                modifier = Modifier.weight(1f),
            )
            NIconButton(NIcons.Add, stringResource(R.string.new_playlist), actions::newPlaylist, size = 40.dp, tint = colors.onSurfaceVariant)
        }
        // The playlists scroll between the destinations and Settings when they run long.
        Column(
            Modifier
                .weight(1f)
                .verticalScroll(rememberScrollState())
        ) {
            for (playlist in playlists) {
                PlaylistLink(playlist, playing = playlist.id == playingFrom, animate = playing) {
                    actions.openPlaylist(playlist.id)
                }
            }
        }
        AnimatedVisibility(
            scan != null,
            enter = expandVertically(NMotion.spatialDefault()) + fadeIn(NMotion.effectsDefault()),
            exit = shrinkVertically(NMotion.spatialDefault()) + fadeOut(NMotion.effectsFast()),
        ) {
            scan?.let { ScanNote(it, actions::openScan, Modifier.padding(top = 8.dp)) }
        }
        DrawerItem(
            NIcons.Settings,
            stringResource(R.string.settings),
            selected = place.settings,
            onClick = actions::openSettings,
            modifier = Modifier.padding(top = 8.dp),
            height = 48,
        )
    }
}

/** One of the drawer's destinations: a pill, filled while [selected]. */
@Composable
private fun DrawerItem(
    icon: ImageVector,
    label: String,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    height: Int = 52,
) {
    val fill by animateColorAsState(
        if (selected) colors.secondaryContainer else Color.Transparent,
        NMotion.effectsDefault(),
        label = "fill",
    )
    val ink = if (selected) colors.onSecondaryContainer else colors.onSurfaceVariant
    Row(
        modifier
            .fillMaxWidth()
            .height(height.dp)
            .clip(RoundedCornerShape((height / 2).dp))
            .background(fill)
            .selectable(selected = selected, role = Role.Tab, onClick = onClick)
            .padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        NIcon(icon, tint = ink)
        Text(label, style = text(15, if (selected) FontWeight.Bold else FontWeight.SemiBold), color = ink, maxLines = 1)
    }
}

/** A playlist in the drawer; the one playing is in the accent, with its bars. */
@Composable
private fun PlaylistLink(playlist: PlaylistRow, playing: Boolean, animate: Boolean, onClick: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .height(44.dp)
            .clip(RoundedCornerShape(22.dp))
            .tappable(onClick)
            .padding(start = 16.dp, end = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        NIcon(if (playlist.rule != null) NIcons.Filter else NIcons.Playlist, size = 20.dp, tint = colors.onSurfaceVariant)
        Text(
            playlist.name,
            style = text(14, FontWeight.Medium),
            color = if (playing) colors.primary else colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        if (playing) PlayingBars(color = colors.primary, height = 12.dp, animate = animate)
    }
}

/** The scan's progress at the drawer's foot, opening the sources. */
@Composable
private fun ScanNote(scan: ScanState, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val description = stringResource(R.string.updating_library)
    Column(
        modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .background(colors.surfaceHigh)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = description }
            .padding(horizontal = 14.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Row {
            Text(description, style = text(12), color = colors.onSurface, modifier = Modifier.weight(1f), maxLines = 1)
            Text(
                stringResource(R.string.read_slash_found, formatCount(scan.read), formatCount(scan.found)),
                style = text(12, tabular = true),
                color = colors.onSurfaceVariant,
                maxLines = 1,
            )
        }
        Box(
            Modifier
                .fillMaxWidth()
                .height(4.dp)
                .background(colors.outlineVariant, RoundedCornerShape(2.dp))
        ) {
            Box(
                Modifier
                    .fillMaxWidth(scan.progress)
                    .height(4.dp)
                    .background(colors.primary, RoundedCornerShape(2.dp))
            )
        }
    }
}

/**
 * The drawer over the pages of foldables and phones held sideways, on its scrim: it slides in
 * from the start while [open], and the scrim or back closes it.
 */
@Composable
fun ModalDrawer(open: Boolean, onClose: () -> Unit, content: @Composable () -> Unit) {
    BackHandler(open, onClose)
    Box(Modifier.fillMaxSize()) {
        AnimatedVisibility(open, enter = fadeIn(NMotion.effectsDefault()), exit = fadeOut(NMotion.effectsDefault())) {
            val close = stringResource(R.string.close_navigation)
            Box(
                Modifier
                    .fillMaxSize()
                    .background(colors.scrim)
                    .clickable(interactionSource = null, indication = null, onClick = onClose)
                    .semantics { contentDescription = close }
            )
        }
        AnimatedVisibility(
            open,
            enter = slideInHorizontally(NMotion.spatialDefault()) { -it },
            exit = slideOutHorizontally(NMotion.noBounce()) { -it },
        ) {
            val shape = RoundedCornerShape(topEnd = 28.dp, bottomEnd = 28.dp)
            val title = stringResource(R.string.navigation)
            Box(
                Modifier
                    .fillMaxHeight()
                    .floating(shape)
                    .background(colors.surfaceLow, shape)
                    .semantics { paneTitle = title }
            ) {
                content()
            }
        }
    }
}
