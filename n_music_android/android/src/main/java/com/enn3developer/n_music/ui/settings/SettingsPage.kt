package com.enn3developer.n_music.ui.settings

import android.os.Build
import androidx.appcompat.app.AppCompatDelegate
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.systemBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.BuildConfig
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.MiniButton
import com.enn3developer.n_music.R
import com.enn3developer.n_music.SleepTimer
import com.enn3developer.n_music.StreamCache
import com.enn3developer.n_music.Theme
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.UiSettings
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.PlaybackOptions
import com.enn3developer.n_music.core.ReplayGainMode
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.LocalPageMargins
import com.enn3developer.n_music.ui.LocalWindowLayout
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.PageMargins
import com.enn3developer.n_music.ui.WindowLayout
import com.enn3developer.n_music.ui.components.MenuItem
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.NMenu
import com.enn3developer.n_music.ui.components.NSwitch
import com.enn3developer.n_music.ui.components.PillButton
import com.enn3developer.n_music.ui.components.PillStyle
import com.enn3developer.n_music.ui.components.PlaybackUi
import com.enn3developer.n_music.ui.components.Segmented
import com.enn3developer.n_music.ui.components.rememberPlaybackSeconds
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.player.fadeThrough
import com.enn3developer.n_music.ui.sources.groupShape
import com.enn3developer.n_music.ui.theme.Accent
import com.enn3developer.n_music.ui.theme.AppLogo
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NType
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import kotlin.math.abs
import kotlin.math.roundToInt

/** Where the source code lives. */
private const val SOURCE_URL = "https://github.com/Enn3Developer/n_music"

/** The longest crossfade, in seconds, and the step the slider takes. */
private const val CROSSFADE_MAX = 12
private const val CROSSFADE_STEP = 2

/** What the cache of web tracks may take, in bytes. */
private val CACHE_LIMITS = listOf(256L, 512L, 1024L, 2048L, 5120L, 10240L).map { it * 1024 * 1024 }

/** What settings change, and where they lead. */
interface SettingsActions {
    fun back()
    fun setReplayGain(mode: ReplayGainMode)
    fun setCrossfade(seconds: Int)
    fun setResume(resume: Boolean)
    fun setCache(enabled: Boolean, limit: Long)
    fun setTheme(theme: Theme)
    fun setAccent(accent: Accent)
    fun setCompactRows(compact: Boolean)
    fun setMiniButtons(buttons: List<MiniButton>)
    fun openLanguage()
    fun shareLogs()
    fun openSource()
    fun openLicence()
}

/** The settings as they stand, for [SettingsContent]. */
data class SettingsState(
    val ui: UiSettings,
    val options: PlaybackOptions,
    val cache: StreamCache?,
    /** The language picked for the app, in itself; `null` while it follows Android's. */
    val language: String?,
    val version: String,
    /** Android colours things after the wallpaper, from Android 12. */
    val wallpaper: Boolean,
    /** What plays, for the mini player's preview. */
    val preview: PlaybackUi,
    /** How far it has played, from 0 to 1, for the preview's line. */
    val progress: () -> Float = { 0f },
)

/** Playback, appearance and what N Music is, each in its cards. */
@Composable
fun SettingsPage() {
    val app = LocalApp.current
    val ui by UiPreferences.settings.collectAsStateWithLifecycle()
    val options by CoreRepository.options.collectAsStateWithLifecycle()
    val cache by CoreRepository.streamCache.collectAsStateWithLifecycle()
    val current by CoreRepository.current.collectAsStateWithLifecycle()
    val playing by CoreRepository.playing.collectAsStateWithLifecycle()
    val shuffle by CoreRepository.shuffle.collectAsStateWithLifecycle()
    val loop by CoreRepository.loopStatus.collectAsStateWithLifecycle()
    val sleep by SleepTimer.state.collectAsStateWithLifecycle()
    val position by CoreRepository.position.collectAsStateWithLifecycle()
    val seconds = rememberPlaybackSeconds(position, playing)
    val actions = remember(app) {
        object : SettingsActions {
            override fun back() = app.back()
            override fun setReplayGain(mode: ReplayGainMode) = CoreRepository.setReplayGain(mode)
            override fun setCrossfade(seconds: Int) = CoreRepository.setCrossfade(seconds.toDouble())
            override fun setResume(resume: Boolean) = CoreRepository.setResume(resume)
            override fun setCache(enabled: Boolean, limit: Long) =
                CoreRepository.send(Command.SetStreamCache(enabled, limit.toULong()))
            override fun setTheme(theme: Theme) = UiPreferences.setTheme(theme)
            override fun setAccent(accent: Accent) = UiPreferences.setAccent(accent)
            override fun setCompactRows(compact: Boolean) = UiPreferences.setCompactRows(compact)
            override fun setMiniButtons(buttons: List<MiniButton>) = UiPreferences.setMiniButtons(buttons)
            override fun openLanguage() = app.openLanguage()
            override fun shareLogs() = app.shareLogs()
            override fun openSource() = app.openLink(SOURCE_URL)
            override fun openLicence() = app.open(Page.Licence)
        }
    }
    val locales = AppCompatDelegate.getApplicationLocales()
    SettingsContent(
        SettingsState(
            ui = ui,
            options = options,
            cache = cache,
            language = locales[0]?.let { it.getDisplayName(it).replaceFirstChar { first -> first.titlecase(it) } },
            version = BuildConfig.VERSION_NAME,
            wallpaper = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S,
            preview = PlaybackUi(current?.track, playing, shuffle, loop, sleep != null),
            progress = {
                val length = position.length.takeIf { it > 0 } ?: current?.track?.length ?: 0.0
                if (length > 0) (seconds.value / length).toFloat() else 0f
            },
        ),
        actions,
    )
}

/** The page itself, for [state]. */
@Composable
fun SettingsContent(state: SettingsState, actions: SettingsActions, list: LazyListState = rememberLazyListState()) {
    if (LocalWindowLayout.current == WindowLayout.TABLET) {
        TabletSettings(state, actions)
        return
    }
    val margins = LocalPageMargins.current
    var miniOpen by rememberSaveable { mutableStateOf(false) }
    // Once the big title has gone up, the bar carries it.
    val scrolled by remember(list) {
        derivedStateOf { list.firstVisibleItemIndex > 0 || list.firstVisibleItemScrollOffset > 0 }
    }
    val titled by remember(list) { derivedStateOf { list.firstVisibleItemIndex > 0 } }
    val bar by animateColorAsState(if (scrolled) colors.surface else colors.background, label = "bar")
    Box(Modifier.fillMaxSize()) {
        LazyColumn(
            Modifier.fillMaxSize(),
            state = list,
            contentPadding = PaddingValues(
                top = 64.dp,
                bottom = 40.dp,
            ),
        ) {
            item(key = "title") {
                Text(
                    stringResource(R.string.settings),
                    style = NType.headline,
                    color = colors.onSurface,
                    modifier = Modifier
                        .windowInsetsPadding(WindowInsets.statusBars)
                        .padding(start = margins.start, end = margins.end, top = 4.dp)
                        .semantics { heading() },
                )
            }
            item(key = "playback") { Heading(R.string.settings_playback, first = true) }
            item(key = "replay gain") { ReplayGainCard(state.options.replayGain, actions::setReplayGain) }
            item(key = "crossfade") { CrossfadeCard(state.options.crossfade, actions::setCrossfade) }
            item(key = "resume") {
                SwitchCard(R.string.resume, R.string.resume_hint, state.options.resume, actions::setResume)
            }
            if (state.cache != null) {
                item(key = "cache") { CacheCards(state.cache, actions::setCache) }
            }
            item(key = "appearance") { Heading(R.string.settings_appearance) }
            item(key = "theme") { ThemeCard(state.ui.theme, actions::setTheme) }
            item(key = "accent") { AccentCard(state.ui.accent, state.wallpaper, actions::setAccent) }
            item(key = "compact") {
                SwitchCard(R.string.compact_rows, R.string.compact_rows_hint, state.ui.compactRows, actions::setCompactRows)
            }
            item(key = "mini player") {
                MiniPlayerCard(state.ui.miniButtons, state.preview, state.progress, miniOpen, { miniOpen = !miniOpen }, actions::setMiniButtons)
            }
            item(key = "language") {
                RowCard(NIcons.Language, stringResource(R.string.language), state.language ?: stringResource(R.string.language_system), actions::openLanguage) {
                    NIcon(NIcons.Open, tint = colors.onSurfaceVariant)
                }
            }
            item(key = "about") { Heading(R.string.settings_about) }
            item(key = "app") { AppCard(state.version) }
            item(key = "logs") { LogsCard(actions::shareLogs) }
            item(key = "links") { Links(actions::openSource, actions::openLicence) }
            item(key = "made by") {
                Text(
                    stringResource(R.string.made_by),
                    style = text(13),
                    color = colors.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                    modifier = Modifier
                        .fillMaxWidth()
                        .windowInsetsPadding(WindowInsets.navigationBars)
                        .padding(top = 24.dp),
                )
            }
        }
        // The bar over the page: the way back, and the title once the page's own scrolled away.
        Row(
            Modifier
                .fillMaxWidth()
                .background(bar)
                .windowInsetsPadding(WindowInsets.statusBars)
                .height(64.dp)
                .padding(start = margins.startLess(12.dp), end = margins.endLess(12.dp)),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            NIconButton(NIcons.Back, stringResource(R.string.back), actions::back, tint = colors.onSurface)
            Text(
                stringResource(R.string.settings),
                style = text(22, FontWeight.Bold, 28.sp),
                color = colors.onSurface,
                modifier = Modifier.graphicsLayer { alpha = if (titled) 1f else 0f },
            )
        }
    }
}

/**
 * A tablet's settings beside its rail: the title over two columns, Playback in one and Appearance
 * and About in the other. The pane stands in for the mini player there, so it has no buttons to
 * pick.
 */
@Composable
private fun TabletSettings(state: SettingsState, actions: SettingsActions) {
    val margins = LocalPageMargins.current
    Column(
        Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .windowInsetsPadding(WindowInsets.systemBars.only(WindowInsetsSides.Vertical))
            .padding(start = margins.start + 8.dp, end = margins.end + 8.dp, top = 4.dp, bottom = 24.dp),
    ) {
        Text(
            stringResource(R.string.settings),
            style = text(32, FontWeight.ExtraBold, 40.sp, (-0.6).sp),
            color = colors.onSurface,
            modifier = Modifier
                .padding(8.dp)
                .semantics { heading() },
        )
        // The columns keep the page's margins; their cards fill them.
        CompositionLocalProvider(LocalPageMargins provides PageMargins(0.dp, 0.dp)) {
            Row(horizontalArrangement = Arrangement.spacedBy(24.dp)) {
                Column(Modifier.weight(1f)) {
                    Heading(R.string.settings_playback, first = true)
                    ReplayGainCard(state.options.replayGain, actions::setReplayGain)
                    CrossfadeCard(state.options.crossfade, actions::setCrossfade)
                    SwitchCard(R.string.resume, R.string.resume_hint, state.options.resume, actions::setResume)
                    if (state.cache != null) CacheCards(state.cache, actions::setCache)
                }
                Column(Modifier.weight(1f)) {
                    Heading(R.string.settings_appearance, first = true)
                    ThemeCard(state.ui.theme, actions::setTheme)
                    AccentCard(state.ui.accent, state.wallpaper, actions::setAccent)
                    SwitchCard(R.string.compact_rows, R.string.compact_rows_hint, state.ui.compactRows, actions::setCompactRows)
                    RowCard(NIcons.Language, stringResource(R.string.language), state.language ?: stringResource(R.string.language_system), actions::openLanguage) {
                        NIcon(NIcons.Open, tint = colors.onSurfaceVariant)
                    }
                    Heading(R.string.settings_about)
                    AboutCard(state.version, actions)
                }
            }
        }
    }
}

/** A tablet's one card about the app: its icon, name, version and makers, then the logs, code and licence. */
@Composable
private fun AboutCard(version: String, actions: SettingsActions) {
    Card {
        Column(Modifier.padding(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                AppLogo(48.dp, corner = 14.dp)
                Column(Modifier.weight(1f)) {
                    Text(stringResource(R.string.app_name), style = text(17, FontWeight.ExtraBold), color = colors.onSurface)
                    Text(
                        stringResource(R.string.app_version_by, version),
                        style = text(14, tabular = true),
                        color = colors.onSurfaceVariant,
                        modifier = Modifier.padding(top = 1.dp),
                    )
                }
            }
            FlowRow(
                Modifier.padding(top = 14.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                AboutButton(stringResource(R.string.share_logs_short), NIcons.Share, actions::shareLogs)
                AboutButton(stringResource(R.string.source_code_short), NIcons.Code, actions::openSource)
                AboutButton(stringResource(R.string.licence_named), NIcons.Info, actions::openLicence)
            }
        }
    }
}

@Composable
private fun AboutButton(label: String, icon: ImageVector, onClick: () -> Unit) {
    PillButton(
        label,
        onClick,
        style = PillStyle.OUTLINED,
        height = 40.dp,
        icon = icon,
        textStyle = text(14, FontWeight.Bold),
        padding = PaddingValues(start = 12.dp, end = 14.dp),
        outline = colors.outlineVariant,
    )
}

/**
 * A part of the settings: Playback, Appearance, About. A tablet's sit a little in from their
 * cards, and closer under them.
 */
@Composable
private fun Heading(title: Int, first: Boolean = false) {
    val margins = LocalPageMargins.current
    val tablet = LocalWindowLayout.current == WindowLayout.TABLET
    val inset = if (tablet) 8.dp else 0.dp
    Text(
        stringResource(title),
        style = text(14, FontWeight.Bold),
        color = colors.primary,
        modifier = Modifier
            // Cards keep 12 dp below them already.
            .padding(
                start = margins.start + inset,
                end = margins.end + inset,
                top = if (first || tablet) 12.dp else 20.dp,
                bottom = 12.dp,
            )
            .semantics { heading() },
    )
}

/** A card of settings, 12 dp from the next. */
@Composable
private fun Card(
    modifier: Modifier = Modifier,
    shape: RoundedCornerShape = RoundedCornerShape(24.dp),
    gap: Boolean = true,
    content: @Composable ColumnScope.() -> Unit,
) {
    val margins = LocalPageMargins.current
    Column(
        modifier
            .padding(start = margins.start, end = margins.end, bottom = if (gap) 12.dp else 0.dp)
            .fillMaxWidth()
            .clip(shape)
            .background(colors.surfaceLow),
        content = content,
    )
}

@Composable
private fun CardTitle(text: String, modifier: Modifier = Modifier) {
    Text(text, style = text(16, FontWeight.Bold), color = colors.onSurface, modifier = modifier)
}

@Composable
private fun CardText(text: String, modifier: Modifier = Modifier, size: Int = 14) {
    Text(
        text,
        style = text(size, lineHeight = if (size == 14) 20.sp else 18.sp),
        color = colors.onSurfaceVariant,
        modifier = modifier,
    )
}

/** Volume levelling: off, per track or per album, and what the one picked does. */
@Composable
private fun ReplayGainCard(mode: ReplayGainMode, onPick: (ReplayGainMode) -> Unit) {
    Card {
        Column(Modifier.padding(16.dp)) {
            CardTitle(stringResource(R.string.replay_gain))
            CardText(stringResource(R.string.replay_gain_hint), Modifier.padding(top = 4.dp))
            Segmented(
                options = listOf(ReplayGainMode.OFF, ReplayGainMode.TRACK, ReplayGainMode.ALBUM),
                selected = mode,
                label = {
                    stringResource(
                        when (it) {
                            ReplayGainMode.OFF -> R.string.replay_gain_off
                            ReplayGainMode.TRACK -> R.string.replay_gain_track
                            ReplayGainMode.ALBUM -> R.string.replay_gain_album
                        }
                    )
                },
                onSelect = onPick,
                modifier = Modifier
                    .padding(top = 14.dp)
                    .selectableGroup(),
                weights = listOf(1f, 1.2f, 1.45f),
            )
            CardText(
                stringResource(
                    when (mode) {
                        ReplayGainMode.OFF -> R.string.replay_gain_off_hint
                        ReplayGainMode.TRACK -> R.string.replay_gain_track_hint
                        ReplayGainMode.ALBUM -> R.string.replay_gain_album_hint
                    }
                ),
                Modifier.padding(top = 10.dp),
                size = 13,
            )
        }
    }
}

/** Crossfade, from off to 12 seconds in steps of 2, with how long it is now. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun CrossfadeCard(seconds: Double, onChange: (Int) -> Unit) {
    var dragged by remember { mutableStateOf<Float?>(null) }
    val shown = (dragged ?: seconds.toFloat()).roundToStep()
    val off = stringResource(R.string.crossfade_off)
    val value = if (shown == 0) off else stringResource(R.string.crossfade_seconds, shown)
    val spoken = if (shown == 0) off else pluralStringResource(R.plurals.crossfade_value, shown, shown)
    val active = colors.primary
    val inactive = colors.secondaryContainer
    val activeDot = colors.onPrimary
    val inactiveDot = colors.onSecondaryContainer
    Card {
        Column(Modifier.padding(16.dp)) {
            Row(verticalAlignment = Alignment.Bottom) {
                CardTitle(stringResource(R.string.crossfade), Modifier.weight(1f))
                Text(value, style = text(16, FontWeight.ExtraBold, tabular = true), color = colors.primary)
            }
            CardText(stringResource(R.string.crossfade_hint), Modifier.padding(top = 4.dp))
            // Material's slider takes the touches and speaks the value; the track is drawn as the
            // design has it, the thumb at its share of the whole width.
            Slider(
                value = dragged ?: seconds.toFloat(),
                onValueChange = { dragged = it },
                onValueChangeFinished = {
                    dragged?.let { onChange(it.roundToStep()) }
                    dragged = null
                },
                valueRange = 0f..CROSSFADE_MAX.toFloat(),
                steps = CROSSFADE_MAX / CROSSFADE_STEP - 1,
                thumb = { Spacer(Modifier.size(4.dp, 44.dp)) },
                track = { Spacer(Modifier.fillMaxWidth().height(16.dp)) },
                modifier = Modifier
                    .padding(top = 10.dp)
                    .height(44.dp)
                    .drawBehind {
                        val fraction = (dragged ?: seconds.toFloat()) / CROSSFADE_MAX
                        drawCrossfade(fraction, CROSSFADE_MAX / CROSSFADE_STEP, active, inactive, activeDot, inactiveDot)
                    }
                    .semantics { stateDescription = spoken },
            )
            Row(Modifier.padding(top = 4.dp)) {
                Text(off, style = text(12, tabular = true), color = colors.onSurfaceVariant)
                Spacer(Modifier.weight(1f))
                Text(stringResource(R.string.crossfade_seconds, CROSSFADE_MAX), style = text(12, tabular = true), color = colors.onSurfaceVariant)
            }
        }
    }
}

private fun Float.roundToStep(): Int = (this / CROSSFADE_STEP).roundToInt() * CROSSFADE_STEP

/**
 * The crossfade's track at [fraction] of the way: the active part, a gap, the thumb, a gap and the
 * rest, with a dot at each of the [steps] but the first. The last dot keeps clear of the rounded
 * end; the one under the thumb hides.
 */
private fun DrawScope.drawCrossfade(
    fraction: Float,
    steps: Int,
    active: Color,
    inactive: Color,
    activeDot: Color,
    inactiveDot: Color,
) {
    val width = size.width
    val thumb = 2.dp.toPx()
    // Half the thumb and 5 dp of space each side of it.
    val gap = 7.dp.toPx()
    // The slider takes 48 dp to be touched; the 44 dp thumb and 16 dp track keep to its middle.
    val middle = size.height / 2
    val top = middle - 8.dp.toPx()
    val bottom = middle + 8.dp.toPx()
    val outer = CornerRadius(8.dp.toPx())
    val inner = CornerRadius(3.dp.toPx())
    val rtl = layoutDirection == LayoutDirection.Rtl
    scale(if (rtl) -1f else 1f, 1f) {
        val at = (fraction.coerceIn(0f, 1f) * width).coerceIn(thumb, width - thumb)
        if (at - gap > 0) {
            val part = RoundRect(Rect(0f, top, at - gap, bottom), topLeft = outer, bottomLeft = outer, topRight = inner, bottomRight = inner)
            drawPath(Path().apply { addRoundRect(part) }, active)
        }
        if (at + gap < width) {
            val part = RoundRect(Rect(at + gap, top, width, bottom), topLeft = inner, bottomLeft = inner, topRight = outer, bottomRight = outer)
            drawPath(Path().apply { addRoundRect(part) }, inactive)
        }
        for (step in 1..steps) {
            val x = if (step == steps) width - 8.dp.toPx() else step.toFloat() / steps * width
            if (abs(x - at) < gap) continue
            drawCircle(if (x < at) activeDot else inactiveDot, 2.dp.toPx(), Offset(x, middle))
        }
        drawRoundRect(active, Offset(at - thumb, middle - 22.dp.toPx()), Size(2 * thumb, 44.dp.toPx()), CornerRadius(thumb))
    }
}

/** A setting that is on or off: its name and what it does beside its switch. */
@Composable
private fun SwitchCard(title: Int, detail: Int, checked: Boolean, onChange: (Boolean) -> Unit) {
    Card {
        SwitchRow(stringResource(title), stringResource(detail), checked, onChange)
    }
}

@Composable
private fun SwitchRow(title: String, detail: String, checked: Boolean, onChange: (Boolean) -> Unit, detailSize: Int = 14) {
    Row(
        Modifier
            .fillMaxWidth()
            .toggleable(checked, role = Role.Switch, onValueChange = onChange)
            .padding(16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Column(Modifier.weight(1f)) {
            CardTitle(title)
            CardText(detail, Modifier.padding(top = 4.dp), size = detailSize)
        }
        // The row toggles; the switch only shows it.
        NSwitch(checked, null)
    }
}

/** Caching web tracks, and how much room the copies may take. */
@Composable
private fun CacheCards(cache: StreamCache, onChange: (Boolean, Long) -> Unit) {
    Column(Modifier.padding(bottom = 12.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Card(shape = groupShape(0, 2, 24.dp, 4.dp), gap = false) {
            SwitchRow(
                stringResource(R.string.cache_web),
                stringResource(if (LocalWindowLayout.current == WindowLayout.TABLET) R.string.cache_web_hint_tablet else R.string.cache_web_hint),
                cache.enabled,
                { onChange(it, cache.limit) },
                detailSize = 13,
            )
        }
        Card(shape = groupShape(1, 2, 24.dp, 4.dp), gap = false) {
            var open by remember { mutableStateOf(false) }
            val limit = sizeText(cache.limit)
            Row(
                Modifier
                    .fillMaxWidth()
                    .graphicsLayer { alpha = if (cache.enabled) 1f else 0.38f }
                    .padding(horizontal = 16.dp, vertical = 14.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Column(Modifier.weight(1f)) {
                    CardTitle(stringResource(R.string.cache_size))
                    Text(
                        stringResource(R.string.cache_used, sizeText(cache.used), limit),
                        style = text(14, tabular = true),
                        color = colors.onSurfaceVariant,
                        modifier = Modifier.padding(top = 2.dp),
                    )
                }
                Box {
                    val description = stringResource(R.string.cache_size_description, limit)
                    Row(
                        Modifier
                            .height(40.dp)
                            .clip(RoundedCornerShape(12.dp))
                            .background(colors.surfaceHigh)
                            .clickable(enabled = cache.enabled, role = Role.DropdownList) { open = true }
                            .semantics { contentDescription = description }
                            .padding(start = 14.dp, end = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Text(limit, style = text(15, FontWeight.Bold, tabular = true), color = colors.onSurface)
                        NIcon(NIcons.Collapse, size = 20.dp, tint = colors.onSurfaceVariant)
                    }
                    NMenu(open, { open = false }) {
                        for (option in CACHE_LIMITS) {
                            MenuItem(sizeText(option), null, {
                                open = false
                                onChange(cache.enabled, option)
                            }, checked = option == cache.limit)
                        }
                    }
                }
            }
        }
    }
}

/** [bytes] the way the design writes sizes: 312 MB, 1 GB, 1.5 GB. */
@Composable
private fun sizeText(bytes: Long): String {
    val locale = LocalConfiguration.current.locales[0]
    val megabytes = bytes / (1024.0 * 1024.0)
    return if (megabytes >= 1024) {
        val gigabytes = megabytes / 1024
        val shown = if (gigabytes % 1.0 < 0.05) gigabytes.roundToInt().toString() else String.format(locale, "%.1f", gigabytes)
        stringResource(R.string.size_gb, shown)
    } else {
        stringResource(R.string.size_mb, megabytes.roundToInt().toString())
    }
}

/** The theme: Android's, light or dark. */
@Composable
private fun ThemeCard(theme: Theme, onPick: (Theme) -> Unit) {
    Card {
        Column(Modifier.padding(16.dp)) {
            CardTitle(stringResource(R.string.theme))
            Segmented(
                options = listOf(Theme.SYSTEM, Theme.LIGHT, Theme.DARK),
                selected = theme,
                label = {
                    stringResource(
                        when (it) {
                            Theme.SYSTEM -> R.string.theme_system
                            Theme.LIGHT -> R.string.theme_light
                            Theme.DARK -> R.string.theme_dark
                        }
                    )
                },
                onSelect = onPick,
                modifier = Modifier
                    .padding(top = 12.dp)
                    .selectableGroup(),
                height = 44.dp,
                weights = listOf(1.2f, 1f, 1f),
            )
        }
    }
}

/** An accent's name. */
@Composable
private fun accentName(accent: Accent): String = stringResource(
    when (accent) {
        Accent.WALLPAPER -> R.string.accent_wallpaper
        Accent.AMBER -> R.string.accent_amber
        Accent.GREEN -> R.string.accent_green
        Accent.TEAL -> R.string.accent_teal
        Accent.BLUE -> R.string.accent_blue
        Accent.VIOLET -> R.string.accent_violet
        Accent.ROSE -> R.string.accent_rose
    }
)

/** The accent colours as swatches, the wallpaper's first where Android has it, and the one picked named. */
@Composable
private fun AccentCard(accent: Accent, wallpaper: Boolean, onPick: (Accent) -> Unit) {
    val accents = if (wallpaper) Accent.entries else Accent.entries - Accent.WALLPAPER
    Card {
        Column(Modifier.padding(16.dp)) {
            CardTitle(stringResource(R.string.accent))
            Row(
                Modifier
                    .padding(top = 16.dp)
                    .fillMaxWidth()
                    .padding(horizontal = 2.dp)
                    .selectableGroup(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                for (option in accents) Swatch(option, option == accent) { onPick(option) }
            }
            Text(
                accentName(accent),
                style = text(14, FontWeight.Bold),
                color = colors.onSurface,
                modifier = Modifier.padding(top = 16.dp),
            )
            if (wallpaper) CardText(stringResource(R.string.accent_hint), Modifier.padding(top = 2.dp), size = 13)
        }
    }
}

/** One accent's round swatch; the picked one is ringed and checked. */
@Composable
private fun Swatch(accent: Accent, selected: Boolean, onClick: () -> Unit) {
    val ring = colors.onSurface
    val card = colors.surfaceLow
    val quarters = if (accent == Accent.WALLPAPER) wallpaperQuarters() else null
    val name = accentName(accent)
    Box(
        Modifier
            .size(40.dp)
            .selectable(selected, role = Role.RadioButton, onClick = onClick)
            .semantics { contentDescription = name },
        contentAlignment = Alignment.Center,
    ) {
        Canvas(Modifier.size(40.dp)) {
            if (selected) {
                drawCircle(ring, radius = size.minDimension / 2 + 5.dp.toPx())
                drawCircle(card, radius = size.minDimension / 2 + 3.dp.toPx())
            }
            if (quarters == null) {
                drawCircle(accent.fill)
            } else {
                // Clockwise from the top: the wallpaper's colours a quarter each.
                quarters.forEachIndexed { index, color ->
                    drawArc(color, startAngle = -90f + 90f * index, sweepAngle = 90f, useCenter = true, size = Size(size.width, size.height), topLeft = Offset.Zero)
                }
            }
        }
        if (selected) NIcon(NIcons.CheckBold, size = 20.dp, tint = accent.onFill)
    }
}

/** Four colours of the wallpaper's palette, as the design's swatch shows them. */
@Composable
private fun wallpaperQuarters(): List<Color> {
    val context = LocalContext.current
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
        return listOf(Color(0xFF7FA4C9), Color(0xFFC9D7A8), Color(0xFFE7C08C), Color(0xFF4B6C8A))
    }
    val dark = remember(context) { dynamicDarkColorScheme(context) }
    val light = remember(context) { dynamicLightColorScheme(context) }
    return listOf(dark.primary, dark.tertiary, dark.secondary, light.primary)
}

/** A card that leads somewhere: its icon, name and value, and [trailing] at its end. */
@Composable
private fun RowCard(icon: ImageVector, title: String, detail: String?, onClick: () -> Unit, trailing: @Composable RowScope.() -> Unit) {
    Card {
        Row(
            Modifier
                .fillMaxWidth()
                .tappable(onClick)
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            NIcon(icon, tint = colors.onSurfaceVariant)
            Column(Modifier.weight(1f)) {
                CardTitle(title)
                if (detail != null) {
                    Text(detail, style = text(14), color = colors.onSurfaceVariant, modifier = Modifier.padding(top = 2.dp))
                }
            }
            trailing()
        }
    }
}

/** The app: its icon, name and version. */
@Composable
private fun AppCard(version: String) {
    Card {
        Row(
            Modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            AppLogo(56.dp, corner = 16.dp)
            Column {
                Text(
                    stringResource(R.string.app_name),
                    style = text(18, FontWeight.ExtraBold, letterSpacing = (-0.2).sp),
                    color = colors.onSurface,
                )
                Text(
                    stringResource(R.string.app_version, version),
                    style = text(14, tabular = true),
                    color = colors.onSurfaceVariant,
                    modifier = Modifier.padding(top = 2.dp),
                )
            }
        }
    }
}

/** The logs, with a way to share them for a bug report. */
@Composable
private fun LogsCard(onShare: () -> Unit) {
    Card {
        Row(
            Modifier.padding(start = 16.dp, end = 12.dp, top = 14.dp, bottom = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            NIcon(NIcons.Logs, tint = colors.onSurfaceVariant)
            Column(Modifier.weight(1f)) {
                CardTitle(stringResource(R.string.logs))
                Text(stringResource(R.string.logs_hint), style = text(14), color = colors.onSurfaceVariant, modifier = Modifier.padding(top = 2.dp))
            }
            val description = stringResource(R.string.share_logs)
            PillButton(
                stringResource(R.string.share),
                onShare,
                modifier = Modifier.semantics { contentDescription = description },
                style = PillStyle.OUTLINED,
                height = 40.dp,
                icon = NIcons.Share,
                textStyle = text(14, FontWeight.Bold),
                padding = PaddingValues(start = 10.dp, end = 14.dp),
            )
        }
    }
}

/** The source code on GitHub and the licence, as one card of two. */
@Composable
private fun Links(onSource: () -> Unit, onLicence: () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Card(shape = groupShape(0, 2, 24.dp, 4.dp), gap = false) {
            Row(
                Modifier
                    .fillMaxWidth()
                    .tappable(onSource)
                    .padding(16.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(14.dp),
            ) {
                NIcon(NIcons.Code, tint = colors.onSurfaceVariant)
                Text(stringResource(R.string.source_code), style = text(16, FontWeight.SemiBold), color = colors.onSurface, modifier = Modifier.weight(1f))
                NIcon(NIcons.External, size = 20.dp, tint = colors.onSurfaceVariant)
            }
        }
        Card(shape = groupShape(1, 2, 24.dp, 4.dp), gap = false) {
            Row(
                Modifier
                    .fillMaxWidth()
                    .tappable(onLicence)
                    .padding(horizontal = 16.dp, vertical = 14.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(14.dp),
            ) {
                NIcon(NIcons.Info, tint = colors.onSurfaceVariant)
                Column(Modifier.weight(1f)) {
                    Text(stringResource(R.string.licence), style = text(16, FontWeight.SemiBold), color = colors.onSurface)
                    Text(stringResource(R.string.licence_hint), style = text(14), color = colors.onSurfaceVariant, modifier = Modifier.padding(top = 2.dp))
                }
                NIcon(NIcons.Open, size = 20.dp, tint = colors.onSurfaceVariant)
            }
        }
    }
}

/** The mini player's buttons: what the bar holds, and the editor while [open]. */
@Composable
private fun MiniPlayerCard(
    buttons: List<MiniButton>,
    preview: PlaybackUi,
    progress: () -> Float,
    open: Boolean,
    onToggle: () -> Unit,
    onChange: (List<MiniButton>) -> Unit,
) {
    val state = stringResource(if (open) R.string.expanded else R.string.collapsed)
    Card {
        Row(
            Modifier
                .fillMaxWidth()
                .tappable(onToggle)
                .semantics { stateDescription = state }
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            NIcon(NIcons.MiniPlayer, tint = colors.onSurfaceVariant)
            Column(Modifier.weight(1f)) {
                CardTitle(stringResource(R.string.mini_player_buttons))
                // It changes over as buttons go in, the way they went: on as one is added, back
                // as one is taken out.
                val shift = with(LocalDensity.current) { 8.dp.roundToPx() }
                AnimatedContent(
                    buttons,
                    Modifier.padding(top = 2.dp),
                    transitionSpec = { fadeThrough(if (targetState.size < initialState.size) -1 else 1, shift) },
                    label = "summary",
                ) { shown ->
                    Text(buttonsLine(shown), style = text(14), color = colors.onSurfaceVariant)
                }
            }
            NIcon(if (open) NIcons.Expand else NIcons.Collapse, tint = colors.onSurfaceVariant)
        }
        if (open) MiniPlayerEditor(buttons, preview, progress, onChange)
    }
}

/** A mini player button's name. */
@Composable
fun buttonName(button: MiniButton): String = stringResource(
    when (button) {
        MiniButton.SHUFFLE -> R.string.mini_shuffle
        MiniButton.PREVIOUS -> R.string.mini_previous
        MiniButton.PLAY_PAUSE -> R.string.mini_play_pause
        MiniButton.NEXT -> R.string.mini_next
        MiniButton.REPEAT -> R.string.mini_repeat
        MiniButton.OUTPUT -> R.string.mini_output
        MiniButton.SLEEP_TIMER -> R.string.mini_sleep_timer
    }
)

/** The buttons in a sentence: Previous, play/pause and next. */
@Composable
private fun buttonsLine(buttons: List<MiniButton>): String {
    if (buttons.isEmpty()) return stringResource(R.string.mini_none)
    val locale = LocalConfiguration.current.locales[0]
    val names = buttons.mapIndexed { index, button ->
        val name = buttonName(button)
        if (index == 0) name else name.lowercase(locale)
    }
    if (names.size == 1) return names[0]
    val pair = stringResource(R.string.list_pair, "\u0000", "\u0001")
    val more = stringResource(R.string.list_more, "\u0000", "\u0001")
    val start = names.dropLast(1).reduce { joined, name -> more.replace("\u0000", joined).replace("\u0001", name) }
    return pair.replace("\u0000", start).replace("\u0001", names.last())
}
