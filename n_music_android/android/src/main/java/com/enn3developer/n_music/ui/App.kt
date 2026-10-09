package com.enn3developer.n_music.ui

import android.content.res.Resources
import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.PlayingFrom
import com.enn3developer.n_music.R
import com.enn3developer.n_music.SleepTimer
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.ui.components.BottomFade
import com.enn3developer.n_music.ui.components.MiniPlayer
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.NavBar
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
import com.enn3developer.n_music.ui.player.PlayerHost
import com.enn3developer.n_music.ui.player.PlayerTransition
import com.enn3developer.n_music.ui.sheets.Sheet
import com.enn3developer.n_music.ui.sheets.SheetHost
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.NTheme
import com.enn3developer.n_music.ui.theme.NType
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.delayed
import kotlinx.coroutines.CoroutineScope

/** What the app needs from its activity: Android's pickers and browser. */
interface AppHost {
    /** Opens Android's folder picker; [onPicked] gets the folder, readable from then on. */
    fun pickFolder(onPicked: (Uri) -> Unit)

    fun openLink(url: String)

    /** Opens Android's output switcher, to play on another speaker or headphones. */
    fun openOutputSwitcher()
}

/** The app's controller: navigation, and playing what a page asks for. */
private class Controller(
    override val navigator: Navigator,
    override val scope: CoroutineScope,
    private val host: AppHost,
) : AppController {
    override val player = PlayerTransition(scope)

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

    override fun openPlayer() = player.open()

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
        this.snack = ++snacks to snack
    }

    override fun dismissSnack(id: Long) {
        if (snack?.first == id) snack = null
    }
}

/** Playback's controls, sent to the core; the app opens the output and the sleep timer. */
object CorePlayback : PlaybackActions {
    override fun togglePause() = CoreRepository.send(Command.TogglePause)
    override fun previous() = CoreRepository.send(Command.PlayPrevious)
    override fun next() = CoreRepository.send(Command.PlayNext)
    override fun toggleShuffle() = CoreRepository.send(Command.ToggleShuffle)
    override fun cycleRepeat() = CoreRepository.send(Command.ToggleRepeat)
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
        CompositionLocalProvider(LocalApp provides controller) {
            Box(Modifier.fillMaxSize()) {
                // Accessibility services see only the top layer: a sheet or a dialog over
                // everything, else the open player over the app.
                val covered = controller.sheet != null || controller.dialog != null
                Box(if (covered || controller.player.isOpen) Modifier.clearAndSetSemantics {} else Modifier) {
                    PhoneLayout(navigator)
                }
                Box(if (covered) Modifier.clearAndSetSemantics {} else Modifier) {
                    PlayerHost(controller.player)
                }
                SheetHost(controller.sheet, controller::closeSheet)
                DialogHost(controller.dialog, controller::closeDialog)
            }
        }
    }
}

@Composable
private fun PhoneLayout(navigator: Navigator) {
    val ui by UiPreferences.settings.collectAsStateWithLifecycle()
    val current by CoreRepository.current.collectAsStateWithLifecycle()
    val playing by CoreRepository.playing.collectAsStateWithLifecycle()
    val shuffle by CoreRepository.shuffle.collectAsStateWithLifecycle()
    val loop by CoreRepository.loopStatus.collectAsStateWithLifecycle()
    val position by CoreRepository.position.collectAsStateWithLifecycle()
    val scan by CoreRepository.scanState.collectAsStateWithLifecycle()
    val sleep by SleepTimer.state.collectAsStateWithLifecycle()
    val seconds = rememberPlaybackSeconds(position, playing)
    val app = LocalApp.current
    val resources = LocalResources.current

    val page = navigator.current.page
    val navigation = page.navigation
    val selection = app.selection
    // Selecting belongs to a page: another page, or another tab, ends it.
    LaunchedEffect(navigator.tab, navigator.current.id) { app.endSelection() }
    val actions = selection != null && navigation
    val miniPlayer = page.miniPlayer && current != null && selection == null
    val density = LocalDensity.current
    val navInset = with(density) { WindowInsets.navigationBars.getBottom(this).toDp() }
    val playerShown by remember(app) { derivedStateOf { app.player.shown } }
    val pull = remember(app, density) {
        app.player.pullGesture(with(density) { 400.dp.toPx() }) { app.player.miniBounds?.top ?: 1f }
    }
    val bottomSpace = (if (navigation) 64.dp + navInset else navInset) +
        (if (miniPlayer || actions) 64.dp + 8.dp else 0.dp)

    Box(
        Modifier
            .fillMaxSize()
            .background(colors.background)
    ) {
        CompositionLocalProvider(LocalBottomSpace provides bottomSpace) {
            PageHost(navigator)
        }
        Column(Modifier.align(Alignment.BottomCenter).fillMaxWidth()) {
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
                            navigation -> 100.dp
                            else -> 120.dp
                        },
                        solidFrom = when {
                            actions -> 0.70f
                            navigation -> 0.72f
                            else -> 0.45f
                        },
                    )
                }
                // Behind the mini player, so a snackbar rises out from under it.
                val snackBottom by animateDpAsState(
                    (if (miniPlayer) 72.dp else 0.dp) + 8.dp + (if (navigation) 0.dp else navInset),
                    NMotion.spatialDefault(),
                    label = "snack",
                )
                SnackbarHost(
                    if (app.player.isOpen) null else app.snack,
                    app::dismissSnack,
                    Modifier
                        .align(Alignment.BottomCenter)
                        .padding(bottom = snackBottom),
                )
                // The plain AnimatedVisibility, not the Column's, for these sit in the Box.
                androidx.compose.animation.AnimatedVisibility(
                    miniPlayer,
                    Modifier.align(Alignment.BottomCenter),
                    // Back from selecting, it waits for the actions to leave.
                    enter = slideInVertically(NMotion.spatialDefault<IntOffset>().delayed(2 * NMotion.STAGGER_MS)) { it } +
                        fadeIn(NMotion.effectsDefault<Float>().delayed(2 * NMotion.STAGGER_MS)),
                    exit = slideOutVertically(NMotion.spatialDefault()) { it } + fadeOut(NMotion.effectsFast()),
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
                        modifier = Modifier
                            .padding(
                                start = 8.dp,
                                end = 8.dp,
                                bottom = if (navigation) 8.dp else 8.dp + navInset,
                            )
                            .onGloballyPositioned { app.player.miniBounds = it.boundsInRoot() }
                            // The player draws it while any of the player shows.
                            .graphicsLayer { alpha = if (playerShown) 0f else 1f },
                    )
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
                            .padding(start = 16.dp, end = 16.dp, bottom = 20.dp),
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
        Page.Search -> SearchPage()
        else -> ComingPage(page)
    }
}

/** A page the app does not draw yet: its name and the way back. */
@Composable
private fun ComingPage(page: Page) {
    val app = LocalApp.current
    Column(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.statusBars).padding(4.dp)) {
        NIconButton(NIcons.Back, stringResource(R.string.back), app::back)
        Text(
            page::class.simpleName.orEmpty(),
            style = NType.headline,
            color = colors.onSurface,
            modifier = Modifier.padding(horizontal = 12.dp),
        )
    }
}
