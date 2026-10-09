package com.enn3developer.n_music.ui

import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
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
import androidx.compose.ui.res.stringResource
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NType
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.Query
import com.enn3developer.n_music.ui.components.BottomFade
import com.enn3developer.n_music.ui.components.MiniPlayer
import com.enn3developer.n_music.ui.components.NavBar
import com.enn3developer.n_music.ui.components.PlaybackActions
import com.enn3developer.n_music.ui.components.PlaybackUi
import com.enn3developer.n_music.ui.components.rememberPlaybackSeconds
import com.enn3developer.n_music.ui.library.LibraryPage
import com.enn3developer.n_music.ui.library.TrackFilters
import com.enn3developer.n_music.ui.sheets.Sheet
import com.enn3developer.n_music.ui.sheets.SheetHost
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.NTheme
import com.enn3developer.n_music.ui.theme.colors

/** What the app needs from its activity: Android's pickers and browser. */
interface AppHost {
    /** Opens Android's folder picker; [onPicked] gets the folder, readable from then on. */
    fun pickFolder(onPicked: (Uri) -> Unit)

    fun openLink(url: String)
}

/** The app's controller: navigation, and playing what a page asks for. */
private class Controller(override val navigator: Navigator) : AppController {
    var playerOpen by mutableStateOf(false)

    override fun open(page: Page) = navigator.open(page)

    override fun back() {
        navigator.back()
    }

    override fun play(query: Query, origin: Origin, start: Locator?, shuffle: Boolean?) {
        if (shuffle != null) CoreRepository.send(Command.SetShuffle(shuffle))
        CoreRepository.send(Command.PlayFrom(query, start))
    }

    override fun openPlayer() {
        playerOpen = true
    }

    override var filters by mutableStateOf(TrackFilters())

    override var sheet by mutableStateOf<Sheet?>(null)
        private set

    override fun show(sheet: Sheet) {
        this.sheet = sheet
    }

    override fun closeSheet() {
        sheet = null
    }
}

/** Playback's controls, sent to the core. */
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
        val controller = remember(navigator) { Controller(navigator) }
        BackHandler(navigator.canGoBack) { navigator.back() }
        CompositionLocalProvider(LocalApp provides controller) {
            Box(Modifier.fillMaxSize()) {
                // Under a sheet, only the sheet is there for accessibility services.
                Box(if (controller.sheet != null) Modifier.clearAndSetSemantics {} else Modifier) {
                    PhoneLayout(navigator)
                }
                SheetHost(controller.sheet, controller::closeSheet)
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
    val seconds = rememberPlaybackSeconds(position, playing)
    val app = LocalApp.current

    val page = navigator.current.page
    val navigation = page.navigation
    val miniPlayer = page.miniPlayer && current != null
    val navInset = with(LocalDensity.current) { WindowInsets.navigationBars.getBottom(this).toDp() }
    val bottomSpace = (if (navigation) 64.dp + navInset else navInset) +
        (if (miniPlayer) 64.dp + 8.dp else 0.dp)

    Box(
        Modifier
            .fillMaxSize()
            .background(colors.background)
    ) {
        CompositionLocalProvider(LocalBottomSpace provides bottomSpace) {
            PageHost(navigator)
        }
        Column(Modifier.align(Alignment.BottomCenter).fillMaxWidth()) {
            AnimatedVisibility(
                miniPlayer,
                enter = slideInVertically(NMotion.spatialDefault()) { it } + fadeIn(NMotion.effectsDefault()),
                exit = slideOutVertically(NMotion.spatialDefault()) { it } + fadeOut(NMotion.effectsFast()),
            ) {
                Box {
                    BottomFade(
                        if (navigation) 100.dp else 120.dp,
                        Modifier.align(Alignment.BottomCenter),
                        solidFrom = if (navigation) 0.72f else 0.45f,
                    )
                    MiniPlayer(
                        ui = PlaybackUi(current?.track, playing, shuffle, loop),
                        progress = {
                            val length = position.length.takeIf { it > 0 } ?: current?.track?.length ?: 0.0
                            if (length > 0) (seconds.value / length).toFloat() else 0f
                        },
                        buttons = ui.miniButtons,
                        actions = CorePlayback,
                        onOpen = app::openPlayer,
                        modifier = Modifier.align(Alignment.BottomCenter).padding(
                            start = 8.dp,
                            end = 8.dp,
                            bottom = if (navigation) 8.dp else 8.dp + navInset,
                        ),
                    )
                }
            }
            if (navigation) {
                NavBar(
                    selected = navigator.tab,
                    onSelect = navigator::select,
                    sourcesBusy = scan != null,
                )
            }
        }
    }
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
