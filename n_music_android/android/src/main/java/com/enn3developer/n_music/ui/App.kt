package com.enn3developer.n_music.ui

import android.content.res.Resources
import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.expandHorizontally
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.shrinkHorizontally
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.core.net.toUri
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.PlayingFrom
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ScanState
import com.enn3developer.n_music.SleepTimer
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.core.PlaylistRow
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.ui.components.BottomFade
import com.enn3developer.n_music.ui.components.DrawerWidth
import com.enn3developer.n_music.ui.components.FabItem
import com.enn3developer.n_music.ui.components.ExtendedFab
import com.enn3developer.n_music.ui.components.FabMenu
import com.enn3developer.n_music.ui.components.MiniPlayer
import com.enn3developer.n_music.ui.components.ModalDrawer
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.NavActions
import com.enn3developer.n_music.ui.components.NavBar
import com.enn3developer.n_music.ui.components.NavDrawer
import com.enn3developer.n_music.ui.components.NavPlace
import com.enn3developer.n_music.ui.components.NavRail
import com.enn3developer.n_music.ui.components.PlaybackActions
import com.enn3developer.n_music.ui.components.PlaybackUi
import com.enn3developer.n_music.ui.components.SelectionActions
import com.enn3developer.n_music.ui.components.SnackbarHost
import com.enn3developer.n_music.ui.components.Tab
import com.enn3developer.n_music.ui.components.rememberPlaybackSeconds
import com.enn3developer.n_music.ui.dialogs.AppDialog
import com.enn3developer.n_music.ui.dialogs.DialogHost
import com.enn3developer.n_music.ui.library.AlbumPage
import com.enn3developer.n_music.ui.library.ArtistPage
import com.enn3developer.n_music.ui.library.LibraryPage
import com.enn3developer.n_music.ui.library.SearchPage
import com.enn3developer.n_music.ui.library.TrackFilters
import com.enn3developer.n_music.ui.player.NowPlayingPane
import com.enn3developer.n_music.ui.player.PlayerHost
import com.enn3developer.n_music.ui.player.PlayerTransition
import com.enn3developer.n_music.ui.playlists.PlaylistPage
import com.enn3developer.n_music.ui.playlists.PlaylistsPage
import com.enn3developer.n_music.ui.settings.LicencePage
import com.enn3developer.n_music.ui.settings.SettingsPage
import com.enn3developer.n_music.ui.sources.SourcePage
import com.enn3developer.n_music.ui.sources.SourcesPage
import com.enn3developer.n_music.ui.sources.WelcomePage
import com.enn3developer.n_music.ui.sheets.Sheet
import com.enn3developer.n_music.ui.sheets.SheetHost
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.NTheme
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.delayed
import kotlinx.coroutines.CoroutineScope

/** What the app needs from its activity: Android's pickers and browser. */
interface AppHost {
    /** Opens Android's folder picker; [onPicked] gets the folder, readable from then on. */
    fun pickFolder(onPicked: (Uri) -> Unit)

    /** Gives back the permission to read [folder] that [pickFolder] kept. */
    fun releaseFolder(folder: Uri)

    fun openLink(url: String)

    /** Opens Android's output switcher, to play on another speaker or headphones. */
    fun openOutputSwitcher()

    /** Offers the logs to the apps that take text, for a bug report. */
    fun shareLogs()

    /** Opens Android's screen for the app's language; `false` where there is none. */
    fun openLanguageSettings(): Boolean
}

/** The app's controller: navigation, and playing what a page asks for. */
private class Controller(
    override val navigator: Navigator,
    override val scope: CoroutineScope,
    private val host: AppHost,
) : AppController {
    override val player = PlayerTransition(scope)

    /** How the window lays the app out: a tablet has no player to open, for its pane shows it. */
    var layout = WindowLayout.PHONE

    /** Going to a page closes the player, which shows over every page. */
    private fun leavePlayer() {
        if (player.isOpen) player.close()
    }

    override fun open(page: Page) {
        leavePlayer()
        navigator.open(page)
    }

    override fun back() {
        navigator.back()
    }

    override fun play(query: Query, origin: Origin, start: Locator?, shuffle: Boolean?) {
        PlayingFrom.set(origin)
        if (shuffle != null) CoreRepository.send(Command.SetShuffle(shuffle))
        CoreRepository.send(Command.PlayFrom(query, start))
    }

    override fun openPlayer() {
        if (layout != WindowLayout.TABLET) player.open()
    }

    override val playback = object : PlaybackActions by CorePlayback {
        override fun openOutput() = host.openOutputSwitcher()

        override fun openSleepTimer() = show(Sheet.SleepTimer)
    }

    override var filters by mutableStateOf(TrackFilters())

    override var tracksShown by mutableIntStateOf(0)
        private set

    override fun showTracks(filters: TrackFilters) {
        leavePlayer()
        this.filters = filters
        navigator.home(Tab.LIBRARY)
        tracksShown++
    }

    override var sheet by mutableStateOf<Sheet?>(null)
        private set

    override fun show(sheet: Sheet) {
        this.sheet = sheet
    }

    override fun closeSheet() {
        sheet = null
    }

    override var dialog by mutableStateOf<AppDialog?>(null)
        private set

    override fun show(dialog: AppDialog) {
        this.dialog = dialog
    }

    override fun closeDialog() {
        dialog = null
    }

    override var selection by mutableStateOf<Selection?>(null)
        private set

    override fun select(track: Locator) {
        val selection = selection
        if (selection == null) {
            this.selection = Selection(track)
        } else {
            selection.toggle(track)
            // Letting go of the last one ends selecting.
            if (selection.count == 0) this.selection = null
        }
    }

    override fun endSelection() {
        selection = null
    }

    private var snacks = 0L

    override var snack by mutableStateOf<Pair<Long, Snack>?>(null)
        private set

    override fun snack(snack: Snack) {
        val replaced = this.snack?.second
        this.snack = ++snacks to snack
        replaced?.onGone?.invoke()
    }

    override fun dismissSnack(id: Long) {
        val (shown, gone) = snack ?: return
        if (shown != id) return
        snack = null
        gone.onGone?.invoke()
    }

    override fun snackAction(id: Long) {
        val (shown, acted) = snack ?: return
        if (shown != id) return
        snack = null
        acted.onAction?.invoke()
    }

    override val removals = Removals()

    override fun pickFolder(onPicked: (Locator) -> Unit) =
        host.pickFolder { onPicked(Locator.DocumentTree(it.toString())) }

    override fun releaseFolder(root: Locator) {
        if (root is Locator.DocumentTree) host.releaseFolder(root.v1.toUri())
    }

    override fun openLink(url: String) = host.openLink(url)

    override fun shareLogs() = host.shareLogs()

    override fun openLanguage() {
        if (!host.openLanguageSettings()) show(Sheet.Language)
    }
}

/** Playback's controls, sent to the core; the app opens the output and the sleep timer. */
object CorePlayback : PlaybackActions {
    override fun togglePause() = CoreRepository.send(Command.TogglePause)
    override fun previous() = CoreRepository.send(Command.PlayPrevious)
    override fun next() = CoreRepository.send(Command.PlayNext)
    override fun toggleShuffle() = CoreRepository.send(Command.ToggleShuffle)
    // Off, all, one, like the notification's button.
    override fun cycleRepeat() = CoreRepository.send(
        Command.SetLoopStatus(
            when (CoreRepository.loopStatus.value) {
                LoopStatus.OFF -> LoopStatus.PLAYLIST
                LoopStatus.PLAYLIST -> LoopStatus.FILE
                LoopStatus.FILE -> LoopStatus.OFF
            }
        )
    )
    override fun openOutput() {}
    override fun openSleepTimer() {}
}

/** The whole app: its pages, and the mini player and navigation over them. */
@Composable
fun NMusicApp(host: AppHost) {
    val ui by UiPreferences.settings.collectAsStateWithLifecycle()
    NTheme(ui.theme, ui.accent) {
        val navigator = rememberNavigator()
        val scope = rememberCoroutineScope()
        val controller = remember(navigator, host) { Controller(navigator, scope, host) }
        BackHandler(navigator.canGoBack) { navigator.back() }
        // Leaving the screen lets the snackbar go, and sends what it held back.
        LifecycleEventEffect(Lifecycle.Event.ON_STOP) {
            controller.snack?.let { controller.dismissSnack(it.first) }
        }
        val resources = LocalResources.current
        LaunchedEffect(controller) {
            CoreRepository.rejections.collect {
                controller.snack(Snack(resources.getString(R.string.playlist_rejected)))
            }
        }
        val roots by CoreRepository.roots.collectAsStateWithLifecycle()
        // The first run asks where the music is; it waits for the core to tell its sources.
        val screen = when {
            ui.welcomed -> Screen.APP
            roots == null -> Screen.WAITING
            roots.orEmpty().isEmpty() -> Screen.WELCOME
            else -> Screen.APP
        }
        CompositionLocalProvider(LocalApp provides controller) {
            WindowLayoutBox {
                Crossfade(screen, animationSpec = NMotion.effectsSlow(), label = "screen") { shown ->
                    when (shown) {
                        Screen.WAITING -> Box(Modifier.fillMaxSize().background(colors.background))
                        Screen.WELCOME -> Box(if (controller.dialog != null) Modifier.clearAndSetSemantics {} else Modifier) {
                            WelcomePage()
                        }
                        Screen.APP -> AppLayers(navigator, controller)
                    }
                }
                DialogHost(controller.dialog, controller::closeDialog)
            }
        }
    }
}

/** What the app shows: the first run's welcome, or the app itself. */
private enum class Screen { WAITING, WELCOME, APP }

/** The app's pages, with the player, the sheets and their messages over them. */
@Composable
private fun AppLayers(navigator: Navigator, controller: Controller) {
    val layout = LocalWindowLayout.current
    SideEffect { controller.layout = layout }
    // A tablet shows what plays in its pane; the player opened elsewhere closes there.
    LaunchedEffect(layout) {
        if (layout == WindowLayout.TABLET && controller.player.isOpen) controller.player.close()
    }
    Box(Modifier.fillMaxSize()) {
        // Accessibility services see only the top layer: a sheet or a dialog over
        // everything, else the open player over the app.
        val covered = controller.sheet != null || controller.dialog != null
        Box(if (covered || controller.player.isOpen) Modifier.clearAndSetSemantics {} else Modifier) {
            MainLayout(navigator)
        }
        if (layout != WindowLayout.TABLET) {
            Box(if (covered) Modifier.clearAndSetSemantics {} else Modifier) {
                PlayerHost(controller.player)
            }
        }
        SheetHost(controller.sheet, controller::closeSheet)
        // Over a sheet, so its own messages, like the queue's Undo, show.
        SnackbarHost(
            controller.snack.takeIf { controller.sheet != null },
            controller::dismissSnack,
            controller::snackAction,
            Modifier
                .align(Alignment.BottomCenter)
                .windowInsetsPadding(WindowInsets.navigationBars)
                .padding(bottom = 8.dp),
        )
    }
}

/** A phone held sideways has its cutout and buttons on a side, which the app holds off. */
private val SideInsets: WindowInsets
    @Composable get() = WindowInsets.displayCutout.union(WindowInsets.navigationBars)

/**
 * The pages with what goes around them: the bottom bar on phones, the rail or a tablet's drawer
 * beside them elsewhere, and the mini player over them, where there is no now playing pane.
 */
@Composable
private fun MainLayout(navigator: Navigator) {
    val layout = LocalWindowLayout.current
    val ui by UiPreferences.settings.collectAsStateWithLifecycle()
    val current by CoreRepository.current.collectAsStateWithLifecycle()
    val playing by CoreRepository.playing.collectAsStateWithLifecycle()
    val shuffle by CoreRepository.shuffle.collectAsStateWithLifecycle()
    val loop by CoreRepository.loopStatus.collectAsStateWithLifecycle()
    val position by CoreRepository.position.collectAsStateWithLifecycle()
    val scan by CoreRepository.scanState.collectAsStateWithLifecycle()
    val playlists by CoreRepository.playlists.collectAsStateWithLifecycle()
    val origin by PlayingFrom.origin.collectAsStateWithLifecycle()
    val sleep by SleepTimer.state.collectAsStateWithLifecycle()
    val seconds = rememberPlaybackSeconds(position, playing)
    val app = LocalApp.current
    val resources = LocalResources.current

    val page = navigator.current.page
    val rail = layout.rail
    val tablet = layout == WindowLayout.TABLET
    // Phones' bottom bar; the rail stays beside every page.
    val navigation = page.navigation && !rail
    val selection = app.selection
    // Selecting belongs to a page: another page, or another tab, ends it.
    LaunchedEffect(navigator.tab, navigator.current.id) { app.endSelection() }
    val actions = selection != null && page.navigation
    // A tablet's pane shows what plays instead.
    val miniPlayer = page.miniPlayer && current != null && selection == null && !tablet
    // Whether it stepped aside for the selection's actions, which it fades out of the way of
    // and back from, rather than sliding.
    val forActions = remember { booleanArrayOf(false) }
    if (!miniPlayer) forActions[0] = actions
    val miniShown = remember { MutableTransitionState(miniPlayer) }
    miniShown.targetState = miniPlayer
    val miniHeight = if (rail) 72.dp else 64.dp
    val density = LocalDensity.current
    val navInset = with(density) { WindowInsets.navigationBars.getBottom(this).toDp() }
    val playerShown by remember(app) { derivedStateOf { app.player.shown } }
    val pull = remember(app, density) {
        app.player.pullGesture(with(density) { 400.dp.toPx() }) { app.player.miniBounds?.top ?: 1f }
    }
    val bottomSpace = (if (navigation) 64.dp + navInset else navInset) +
        (if (miniPlayer || actions) miniHeight + 8.dp else 0.dp)
    // Playlists' button for a new one, plain or smart; open, its scrim covers all the rest.
    val onPlaylists = page == Page.Playlists
    var newOpen by rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(onPlaylists) { if (!onPlaylists) newOpen = false }
    val covered = if (newOpen && onPlaylists) Modifier.clearAndSetSemantics {} else Modifier
    var snackHeight by remember { mutableStateOf(0.dp) }
    // The drawer over the pages, beside a foldable's rail.
    val drawer = rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(rail, tablet) { if (!rail || tablet) drawer.value = false }
    val place = NavPlace(navigator.tab, settings = page == Page.Settings || page == Page.Licence)
    val navActions = remember(app, navigator) { NavigationActions(app, navigator) { drawer.value = false } }
    val playingFrom = (origin as? Origin.Playlist)?.id

    Box(
        Modifier
            .fillMaxSize()
            .background(colors.background)
    ) {
        Row(Modifier.fillMaxSize().windowInsetsPadding(SideInsets.only(WindowInsetsSides.End))) {
            if (tablet) {
                TabletNavigation(
                    expanded = ui.drawer,
                    onExpand = UiPreferences::setDrawer,
                    place = place,
                    playlists = playlists,
                    playingFrom = playingFrom,
                    playing = playing,
                    scan = scan,
                    actions = navActions,
                    modifier = covered,
                )
            } else if (rail) {
                NavRail(place, scan != null, navActions, onMenu = { drawer.value = true }, modifier = covered)
            }
            Box(
                Modifier
                    .weight(1f)
                    .fillMaxHeight()
            ) {
                Box(covered) {
                    // The rail's side follows the drawer as it widens.
                    val start by animateDpAsState(
                        when {
                            !rail -> 16.dp
                            tablet && !ui.drawer -> 0.dp
                            else -> 24.dp
                        },
                        NMotion.spatialDefault(),
                        label = "margin",
                    )
                    val margins = PageMargins(start, if (rail) 24.dp else 16.dp)
                    CompositionLocalProvider(LocalBottomSpace provides bottomSpace, LocalPageMargins provides margins) {
                        PageHost(navigator)
                    }
                }
                Column(
                    Modifier
                        .align(Alignment.BottomCenter)
                        .fillMaxWidth()
                        .then(covered)
                ) {
                    Box(Modifier.fillMaxWidth()) {
                        // The plain AnimatedVisibility, not the Column's, for these sit in the Box.
                        androidx.compose.animation.AnimatedVisibility(
                            miniPlayer || actions,
                            Modifier.align(Alignment.BottomCenter),
                            enter = fadeIn(NMotion.effectsDefault()),
                            exit = fadeOut(NMotion.effectsFast()),
                        ) {
                            BottomFade(
                                when {
                                    actions -> 110.dp
                                    rail -> 130.dp
                                    navigation -> 100.dp
                                    else -> 120.dp
                                },
                                solidFrom = when {
                                    actions -> 0.70f
                                    rail -> 0.60f
                                    navigation -> 0.72f
                                    else -> 0.45f
                                },
                            )
                        }
                        // Behind the mini player, so a snackbar rises out from under it.
                        val snackBottom by animateDpAsState(
                            (if (miniPlayer) miniHeight + 8.dp else 0.dp) + 8.dp + (if (navigation) 0.dp else navInset),
                            NMotion.spatialDefault(),
                            label = "snack",
                        )
                        // An action's snackbar waits for the mini player coming back to rise from.
                        val behindReturn = forActions[0] && miniShown.targetState && !miniShown.isIdle
                        SnackbarHost(
                            if (app.player.isOpen || app.sheet != null) null else app.snack,
                            app::dismissSnack,
                            app::snackAction,
                            enterDelay = if (behindReturn) SNACK_AFTER_ACTIONS_MS else 0L,
                            modifier = Modifier
                                .align(Alignment.BottomCenter)
                                .padding(bottom = snackBottom)
                                .onSizeChanged { snackHeight = with(density) { it.height.toDp() } },
                        )
                        // The plain AnimatedVisibility, not the Column's, for these sit in the Box.
                        androidx.compose.animation.AnimatedVisibility(
                            miniShown,
                            Modifier.align(Alignment.BottomCenter),
                            // For the actions it shrinks a little as it fades, once the covers
                            // have turned, and comes back once they have started leaving.
                            enter = if (forActions[0]) {
                                scaleIn(NMotion.spatialDefault<Float>().delayed(MINI_BACK_MS), initialScale = MINI_ASIDE_SCALE) +
                                    fadeIn(NMotion.effectsDefault<Float>().delayed(MINI_BACK_MS))
                            } else {
                                slideInVertically(NMotion.spatialDefault()) { it } + fadeIn(NMotion.effectsDefault())
                            },
                            exit = if (forActions[0]) {
                                scaleOut(NMotion.spatialDefault<Float>().delayed(MINI_ASIDE_MS), targetScale = MINI_ASIDE_SCALE) +
                                    fadeOut(NMotion.effectsFast<Float>().delayed(MINI_ASIDE_MS))
                            } else {
                                slideOutVertically(NMotion.spatialDefault()) { it } + fadeOut(NMotion.effectsFast())
                            },
                        ) {
                            MiniPlayer(
                                ui = PlaybackUi(current?.track, playing, shuffle, loop, sleep != null),
                                progress = {
                                    val length = position.length.takeIf { it > 0 } ?: current?.track?.length ?: 0.0
                                    if (length > 0) (seconds.value / length).toFloat() else 0f
                                },
                                buttons = ui.miniButtons,
                                actions = app.playback,
                                onOpen = app::openPlayer,
                                pull = pull,
                                bar = rail,
                                modifier = Modifier
                                    .padding(
                                        start = if (rail) 12.dp else 8.dp,
                                        end = if (rail) 16.dp else 8.dp,
                                        bottom = if (navigation) 8.dp else 8.dp + navInset,
                                    )
                                    .onGloballyPositioned { app.player.miniBounds = it.boundsInRoot() }
                                    // The player draws it while any of the player shows.
                                    .graphicsLayer { alpha = if (playerShown) 0f else 1f },
                            ) {
                                if (rail) {
                                    NIconButton(
                                        NIcons.PlayNext,
                                        stringResource(R.string.open_queue),
                                        { app.show(Sheet.Queue) },
                                        tint = colors.onSurfaceVariant,
                                    )
                                }
                            }
                        }
                        // The selection's actions outlive it while they leave.
                        val acting = remember { arrayOfNulls<Selection>(1) }
                        if (selection != null) acting[0] = selection
                        // The plain AnimatedVisibility, not the Column's, for these sit in the Box.
                        androidx.compose.animation.AnimatedVisibility(
                            actions,
                            Modifier.align(Alignment.BottomCenter),
                            enter = EnterTransition.None,
                            exit = ExitTransition.None,
                        ) {
                            val picked = acting[0]
                            SelectionActions(
                                count = picked?.count ?: 0,
                                onPlayNext = { picked?.let { queue(app, resources, it, next = true) } },
                                onQueue = { picked?.let { queue(app, resources, it, next = false) } },
                                onAddToPlaylist = { picked?.let { app.show(Sheet.AddToPlaylist(it.tracks)) } },
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(start = 16.dp, end = 16.dp, bottom = 20.dp + if (navigation) 0.dp else navInset),
                            )
                        }
                    }
                    if (navigation) {
                        NavBar(
                            selected = navigator.tab,
                            onSelect = navigator::select,
                            sourcesBusy = scan != null,
                            // It slides away as the player opens.
                            modifier = Modifier.graphicsLayer {
                                translationY = app.player.expand.value.coerceIn(0f, 1f) * size.height
                            },
                        )
                    }
                }
                // A snackbar pushes it up.
                val fabBottom by animateDpAsState(
                    bottomSpace + 16.dp + if (snackHeight > 0.dp) snackHeight + 8.dp else 0.dp,
                    NMotion.spatialDefault(),
                    label = "fab",
                )
                FabMenu(
                    visible = onPlaylists,
                    open = newOpen && onPlaylists,
                    onOpenChange = { newOpen = it },
                    description = stringResource(R.string.new_playlist_menu),
                    items = listOf(
                        FabItem(NIcons.Playlist, stringResource(R.string.new_playlist)) {
                            app.show(AppDialog.NewPlaylist(emptyList()))
                        },
                        // The library's filters carry over.
                        FabItem(NIcons.Filter, stringResource(R.string.new_smart_playlist)) {
                            app.show(Sheet.SmartPlaylist(null, start = app.filters))
                        },
                    ),
                    bottom = fabBottom,
                    modifier = Modifier.graphicsLayer { alpha = 1f - app.player.expand.value.coerceIn(0f, 1f) },
                )
                // Sources' button for a new one.
                androidx.compose.animation.AnimatedVisibility(
                    page == Page.Sources,
                    Modifier
                        .align(Alignment.BottomEnd)
                        .padding(end = 16.dp, bottom = fabBottom)
                        .graphicsLayer { alpha = 1f - app.player.expand.value.coerceIn(0f, 1f) },
                    enter = scaleIn(NMotion.spatialDefault(), initialScale = 0.6f, transformOrigin = TransformOrigin(1f, 1f)) +
                        fadeIn(NMotion.effectsDefault()),
                    exit = scaleOut(NMotion.effectsFast(), targetScale = 0.6f, transformOrigin = TransformOrigin(1f, 1f)) +
                        fadeOut(NMotion.effectsFast()),
                ) {
                    ExtendedFab(NIcons.Add, stringResource(R.string.add_source), { app.show(Sheet.AddSource) })
                }
            }
            // A tablet shows what plays beside the pages, but for Settings.
            androidx.compose.animation.AnimatedVisibility(
                visible = tablet && current != null && !place.settings,
                // It slides in from the screen's edge as the pages make room.
                enter = expandHorizontally(NMotion.spatialDefault(), Alignment.End) + fadeIn(NMotion.effectsDefault()),
                exit = shrinkHorizontally(NMotion.spatialDefault(), Alignment.End) + fadeOut(NMotion.effectsFast()),
            ) {
                NowPlayingPane(narrow = ui.drawer, modifier = covered)
            }
        }
        if (rail && !tablet) {
            ModalDrawer(drawer.value, { drawer.value = false }) {
                NavDrawer(
                    place = place,
                    playlists = playlists,
                    playingFrom = playingFrom,
                    playing = playing,
                    scan = scan,
                    actions = navActions,
                    onClose = { drawer.value = false },
                    closeLabel = stringResource(R.string.close_navigation),
                )
            }
        }
    }
}

/**
 * A tablet's navigation: the rail, which ☰ widens into the drawer, [expanded] beside the pages
 * until it is folded back.
 */
@Composable
private fun TabletNavigation(
    expanded: Boolean,
    onExpand: (Boolean) -> Unit,
    place: NavPlace,
    playlists: List<PlaylistRow>,
    playingFrom: Long?,
    playing: Boolean,
    scan: ScanState?,
    actions: NavActions,
    modifier: Modifier = Modifier,
) {
    val width by animateDpAsState(if (expanded) DrawerWidth else 96.dp, NMotion.spatialDefault(), label = "drawer")
    val fill by animateColorAsState(
        if (expanded) colors.surfaceLow else colors.surfaceLow.copy(alpha = 0f),
        NMotion.effectsDefault(),
        label = "drawerFill",
    )
    Box(
        modifier
            .fillMaxHeight()
            .background(fill)
            .windowInsetsPadding(SideInsets.only(WindowInsetsSides.Start))
            .width(width)
            .clipToBounds()
    ) {
        Crossfade(expanded, animationSpec = NMotion.effectsDefault(), label = "drawerContent") { open ->
            // The drawer keeps its width while the rail widens into it.
            Box(Modifier.wrapContentWidth(Alignment.Start, unbounded = true)) {
                if (open) {
                    NavDrawer(place, playlists, playingFrom, playing, scan, actions, onClose = { onExpand(false) })
                } else {
                    NavRail(place, scan != null, actions, onMenu = { onExpand(true) }, tablet = true)
                }
            }
        }
    }
}

/**
 * What the rail and the drawer do: going to a place leaves Settings, and closes the drawer over
 * the pages.
 */
private class NavigationActions(
    private val app: AppController,
    private val navigator: Navigator,
    private val closeDrawer: () -> Unit,
) : NavActions {
    override fun select(tab: Tab) {
        closeDrawer()
        // From Settings, a tab goes back to what it showed.
        val fromSettings = navigator.current.page.let { it == Page.Settings || it == Page.Licence }
        navigator.closeSettings()
        if (!fromSettings || tab != navigator.tab) navigator.select(tab)
    }

    override fun openSettings() {
        closeDrawer()
        when (navigator.current.page) {
            Page.Settings -> {}
            Page.Licence -> navigator.back()
            else -> app.open(Page.Settings)
        }
    }

    override fun openPlaylist(id: Long) {
        closeDrawer()
        navigator.closeSettings()
        navigator.home(Tab.PLAYLISTS)
        app.open(Page.Playlist(id))
    }

    override fun newPlaylist() {
        closeDrawer()
        app.show(AppDialog.NewPlaylist(emptyList()))
    }

    override fun openScan() {
        closeDrawer()
        navigator.closeSettings()
        navigator.home(Tab.SOURCES)
    }
}

/** Queues what [selection] holds, ends selecting and offers to undo it. */
private fun queue(app: AppController, resources: Resources, selection: Selection, next: Boolean) {
    val tracks = selection.tracks
    val undo = enqueue(tracks, next)
    app.endSelection()
    val count = tracks.size
    app.snack(
        Snack(
            resources.getQuantityString(
                if (next) R.plurals.queued_next else R.plurals.queued,
                quantity(count),
                formatCount(count),
            ),
            resources.getString(R.string.undo),
            undo,
        )
    )
}

/** How long after selecting starts the mini player steps aside for the actions. */
private const val MINI_ASIDE_MS = 100L

/** How long after the selection ends the mini player comes back. */
private const val MINI_BACK_MS = 140L

/** How far the mini player shrinks as it steps aside. */
private const val MINI_ASIDE_SCALE = 0.96f

/** How long after the selection ends an action's snackbar rises from behind the mini player. */
private const val SNACK_AFTER_ACTIONS_MS = 240L

/**
 * The current page. Another tab fades through; a page opened over another slides in from the
 * end and back out when it closes.
 */
@Composable
private fun PageHost(navigator: Navigator) {
    val holder = rememberSaveableStateHolder()
    // Forgets the state of pages that left every stack.
    LaunchedEffect(navigator) {
        val seen = mutableSetOf<Long>()
        snapshotFlow { navigator.liveIds() }.collect { live ->
            for (id in seen - live) holder.removeState(id)
            seen.clear()
            seen.addAll(live)
        }
    }
    AnimatedContent(
        targetState = navigator.tab to navigator.current,
        contentKey = { it.second.id },
        transitionSpec = {
            val (fromTab, from) = initialState
            val (toTab, to) = targetState
            when {
                fromTab != toTab -> fadeIn(NMotion.effectsDefault()) togetherWith fadeOut(NMotion.effectsFast())
                to.id > from.id ->
                    (slideInHorizontally(NMotion.noBounce()) { it / 4 } + fadeIn(NMotion.effectsDefault()))
                        .togetherWith(slideOutHorizontally(NMotion.noBounce()) { -it / 8 } + fadeOut(NMotion.effectsFast()))

                else ->
                    (slideInHorizontally(NMotion.noBounce()) { -it / 8 } + fadeIn(NMotion.effectsDefault()))
                        .togetherWith(slideOutHorizontally(NMotion.noBounce()) { it / 4 } + fadeOut(NMotion.effectsFast()))
            }
        },
        label = "page",
    ) { (_, entry) ->
        holder.SaveableStateProvider(entry.id) {
            Box(Modifier.fillMaxSize().background(colors.background)) {
                PageContent(entry.page)
            }
        }
    }
}

@Composable
private fun PageContent(page: Page) {
    when (page) {
        Page.Library -> LibraryPage()
        is Page.Album -> AlbumPage(page)
        is Page.Artist -> ArtistPage(page)
        is Page.Search -> SearchPage(page.text)
        Page.Playlists -> PlaylistsPage()
        is Page.Playlist -> PlaylistPage(page)
        Page.Sources -> SourcesPage()
        is Page.Source -> SourcePage(page)
        Page.Settings -> SettingsPage()
        Page.Licence -> LicencePage()
    }
}
