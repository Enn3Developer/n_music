package com.enn3developer.n_music.ui.player

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.ContentTransform
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.AnimationVector1D
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitVerticalTouchSlopOrCancellation
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsBottomHeight
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.FloatState
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.MotionDurationScale
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.unit.toSize
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.core.TrackDetails
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.ui.LocalWindowLayout
import com.enn3developer.n_music.ui.WindowLayout
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.CoverPlaceholder
import com.enn3developer.n_music.ui.components.InfoChip
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.PlaybackActions
import com.enn3developer.n_music.ui.components.floating
import com.enn3developer.n_music.ui.components.margins
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.NType
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.delayed
import com.enn3developer.n_music.ui.theme.text
import com.enn3developer.n_music.ui.trackFormat

/** The player's parts, in the order they come in as it opens. */
enum class PlayerGroup { TOP_BAR, TITLE, SEEK, CONTROLS, CHIPS, UP_NEXT }

/** What the player asks for beyond playback's controls. */
interface PlayerActions : PlaybackActions {
    fun close()

    /** Opens the page of what plays, its "Playing from". */
    fun openOrigin()

    /** Opens the playing track's actions. */
    fun openMore()

    fun openArtist()

    fun openAlbum()

    fun seek(seconds: Double)

    fun openQueue()
}

/** What the player shows of playback. */
@Immutable
data class PlayerUi(
    /** The play session's item playing: a skip changes it, the same track met again does not. */
    val item: ULong,
    val track: TrackRow,
    val details: TrackDetails?,
    val playing: Boolean,
    val shuffle: Boolean,
    val loop: LoopStatus,
    /** What it plays from, as "Playing from" names it; `null` when that is not known. */
    val origin: String?,
    /** What plays after it; `null` at the end of the queue. */
    val next: TrackRow?,
    /** Where it plays: this phone, or the headphones it plays on. */
    val output: String,
    /** A running sleep timer, as its chip says it; `null` while none runs. */
    val sleep: Sleep?,
)

/** A running sleep timer as the player's chip says it: [label] on it, [description] aloud. */
@Immutable
data class Sleep(val label: String, val description: String)

/** The covers' corners on the player. */
val PlayerCoverShape = RoundedCornerShape(28.dp)

/** How long new text waits for the old to leave when it fades through. */
private const val FADE_THROUGH_MS = 90L

/** How big the player's text is and how far apart its parts sit, which the window decides. */
@Immutable
internal data class PlayerMetrics(
    /** Space on both sides of the title, the position and the controls. */
    val side: Dp,
    val title: TextStyle,
    val titleTop: Dp,
    /** The artist and album. */
    val line: TextStyle,
    val chipsTop: Dp,
    val seekTop: Dp,
    val controlsTop: Dp,
    val controls: ControlSizes,
    val lineTop: Dp = 4.dp,
    /** Smaller chips, and a smaller position bar, as a tablet's pane has them. */
    val small: Boolean = false,
) {
    companion object {
        val Phone = PlayerMetrics(
            32.dp, NType.titleLarge, 24.dp, text(16, lineHeight = 22.sp), 14.dp, 20.dp, 20.dp, ControlSizes.Phone,
        )
        val Fold = PlayerMetrics(
            24.dp, text(24, FontWeight.ExtraBold, 30.sp, (-0.3).sp), 20.dp, text(15, lineHeight = 18.sp), 12.dp, 18.dp, 18.dp,
            ControlSizes.Fold,
        )
        val Landscape = PlayerMetrics(
            0.dp, NType.titleLarge, 14.dp, text(15, lineHeight = 18.sp), 12.dp, 16.dp, 12.dp, ControlSizes.Landscape,
        )
        val Pane = PlayerMetrics(
            0.dp, text(22, FontWeight.ExtraBold, 28.sp), 16.dp, text(14), 10.dp, 12.dp, 10.dp, ControlSizes.Pane,
            lineTop = 2.dp,
            small = true,
        )

        /** The pane beside the open drawer, a little narrower. */
        val NarrowPane = Pane.copy(seekTop = 14.dp, controls = ControlSizes.Pane.copy(playWidth = 72.dp))
    }
}

/** The open player's width on a foldable, beside the queue. */
private val FoldPlayerWidth = 344.dp

/**
 * The player: where it plays from, the cover, the track with its artist and album, the position,
 * the controls, the output and sleep timer, and what plays next. [artwork] gives a track's
 * cover. A skip slides the covers one place, the way [skip] says it went: 1 to the next, -1 back,
 * and the text fades through. Pausing shrinks the cover and flattens the wave. [group] gives each
 * part the look of its way in; [coverShown] hides the cover while another draws it flying, and
 * [onCoverPlaced] tells where it is, from the player's top left corner. A foldable shows the
 * [queue] beside it; a phone held sideways puts the cover beside the rest.
 */
@Composable
fun PlayerContent(
    ui: PlayerUi,
    artwork: @Composable (TrackRow) -> ImageBitmap?,
    seconds: () -> Double,
    length: Double,
    actions: PlayerActions,
    modifier: Modifier = Modifier,
    skip: () -> Int = { 1 },
    group: (PlayerGroup) -> Modifier = { Modifier },
    coverShown: () -> Boolean = { true },
    onCoverPlaced: (Rect) -> Unit = {},
    queue: @Composable (Modifier) -> Unit = {},
) {
    val root = remember { arrayOfNulls<LayoutCoordinates>(1) }
    val paused = rememberPaused(ui.playing)
    val phase = rememberWavePhase(ui.playing)
    val placed = Modifier.onPlaced { root[0] = it }
    val coverPlaced = Modifier.onPlaced { cover ->
        root[0]?.let { onCoverPlaced(Rect(it.localPositionOf(cover, Offset.Zero), cover.size.toSize())) }
    }

    @Composable
    fun CoverSlot(modifier: Modifier) = PlayerCover(ui, artwork, skip, { paused.value }, coverShown, modifier.then(coverPlaced))

    @Composable
    fun Title(metrics: PlayerMetrics) {
        val shift = with(LocalDensity.current) { 12.dp.roundToPx() }
        AnimatedContent(
            targetState = ui,
            contentKey = { it.item },
            transitionSpec = { fadeThrough(skip(), shift) },
            modifier = Modifier.then(group(PlayerGroup.TITLE)),
            label = "title",
        ) { shown ->
            TitleBlock(shown, actions, metrics)
        }
    }

    @Composable
    fun Seek(metrics: PlayerMetrics) = SeekBar(
        seconds = seconds,
        length = length,
        onSeek = actions::seek,
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = metrics.side, end = metrics.side, top = metrics.seekTop)
            .then(group(PlayerGroup.SEEK)),
        item = ui.item,
        skip = skip,
        wave = { 1f - 0.98f * paused.value },
        phase = { phase.floatValue },
    )

    @Composable
    fun Controls(metrics: PlayerMetrics, modifier: Modifier = Modifier) = PlayerControls(
        playing = ui.playing,
        shuffle = ui.shuffle,
        loop = ui.loop,
        actions = actions,
        modifier = modifier.then(group(PlayerGroup.CONTROLS)),
        sizes = metrics.controls,
    )

    @Composable
    fun Chips(top: Dp) = Row(
        Modifier
            .fillMaxWidth()
            .padding(top = top)
            .then(group(PlayerGroup.CHIPS)),
        horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
    ) {
        OutputChip(ui.output, actions::openOutput)
        SleepChip(ui.sleep, actions::openSleepTimer)
    }

    Box(modifier.fillMaxSize()) {
        when (LocalWindowLayout.current) {
            WindowLayout.FOLD -> Row(placed.fillMaxSize()) {
                val metrics = PlayerMetrics.Fold
                Column(
                    Modifier
                        .width(FoldPlayerWidth)
                        .fillMaxHeight()
                        .windowInsetsPadding(WindowInsets.statusBars)
                ) {
                    TopBar(ui, actions, Modifier.then(group(PlayerGroup.TOP_BAR)))
                    CoverSlot(
                        Modifier
                            .weight(1f, fill = false)
                            .padding(start = metrics.side, end = metrics.side, top = 12.dp)
                            .aspectRatio(1f)
                            .align(Alignment.CenterHorizontally)
                    )
                    Title(metrics)
                    Seek(metrics)
                    Controls(
                        metrics,
                        Modifier
                            .fillMaxWidth()
                            .padding(start = metrics.side, end = metrics.side, top = metrics.controlsTop),
                    )
                    Chips(top = 20.dp)
                    Spacer(Modifier.windowInsetsBottomHeight(WindowInsets.navigationBars))
                }
                queue(
                    Modifier
                        .weight(1f)
                        .fillMaxHeight()
                        .windowInsetsPadding(WindowInsets.statusBars)
                        .windowInsetsPadding(WindowInsets.navigationBars)
                        .padding(top = 8.dp, end = 8.dp)
                        .then(group(PlayerGroup.UP_NEXT))
                )
            }

            WindowLayout.LANDSCAPE -> Row(
                placed
                    .fillMaxSize()
                    .windowInsetsPadding(WindowInsets.safeDrawing)
                    .padding(start = 48.dp, top = 4.dp, end = 24.dp, bottom = 24.dp),
                horizontalArrangement = Arrangement.spacedBy(32.dp),
            ) {
                val metrics = PlayerMetrics.Landscape
                CoverSlot(
                    Modifier
                        .fillMaxHeight()
                        .aspectRatio(1f)
                )
                // Its top bar lines up with the cover's top edge, the rest under it.
                Column(
                    Modifier
                        .weight(1f)
                        .offset(y = (-12).dp)
                ) {
                    SideTopBar(ui, actions, Modifier.then(group(PlayerGroup.TOP_BAR)))
                    Title(metrics)
                    Seek(metrics)
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .padding(top = metrics.controlsTop),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Controls(metrics)
                        Spacer(Modifier.weight(1f))
                        QueueButton(actions::openQueue, Modifier.then(group(PlayerGroup.CHIPS)))
                    }
                }
            }

            else -> {
                val metrics = PlayerMetrics.Phone
                Column(
                    placed
                        .fillMaxSize()
                        .windowInsetsPadding(WindowInsets.statusBars)
                ) {
                    TopBar(ui, actions, Modifier.then(group(PlayerGroup.TOP_BAR)))
                    CoverSlot(
                        Modifier
                            .weight(1f, fill = false)
                            .padding(start = metrics.side, end = metrics.side, top = 12.dp)
                            .aspectRatio(1f)
                            .align(Alignment.CenterHorizontally)
                    )
                    Title(metrics)
                    Seek(metrics)
                    Controls(
                        metrics,
                        Modifier
                            .fillMaxWidth()
                            .padding(start = metrics.side, end = metrics.side, top = metrics.controlsTop),
                    )
                    Chips(top = 24.dp)
                    // Room for what plays next, which stays at the bottom.
                    Spacer(Modifier.height(UpNextHeight))
                    Spacer(Modifier.windowInsetsBottomHeight(WindowInsets.navigationBars))
                }
                UpNext(
                    ui.next,
                    actions::openQueue,
                    Modifier
                        .align(Alignment.BottomCenter)
                        .then(group(PlayerGroup.UP_NEXT)),
                    skip,
                )
            }
        }
    }
}

/** From 0 while [playing] to 1 while paused: the cover shrinks to 94% and the wave flattens. */
@Composable
internal fun rememberPaused(playing: Boolean): Animatable<Float, AnimationVector1D> {
    val paused = remember { Animatable(if (playing) 0f else 1f) }
    LaunchedEffect(playing) { paused.animateTo(if (playing) 0f else 1f, NMotion.spatialSlow()) }
    return paused
}

/**
 * Where the wave is along its length, from 0 to 1: it travels a wavelength a second while
 * [playing], and stands still while paused or while animations are off.
 */
@Composable
internal fun rememberWavePhase(playing: Boolean): FloatState {
    val phase = remember { mutableFloatStateOf(0f) }
    LaunchedEffect(playing) {
        if (!playing || coroutineContext[MotionDurationScale]?.scaleFactor == 0f) return@LaunchedEffect
        var last = withFrameNanos { it }
        while (true) {
            withFrameNanos { now ->
                phase.floatValue = (phase.floatValue + (now - last) / 1e9f) % 1f
                last = now
            }
        }
    }
    return phase
}

/** The cover, which a skip slides one place, shrunk while [paused] and hidden while not [shown]. */
@Composable
internal fun PlayerCover(
    ui: PlayerUi,
    artwork: @Composable (TrackRow) -> ImageBitmap?,
    skip: () -> Int,
    paused: () -> Float,
    shown: () -> Boolean,
    modifier: Modifier,
    shape: Shape = PlayerCoverShape,
) {
    val density = LocalDensity.current
    Box(
        modifier.graphicsLayer {
            alpha = if (shown()) 1f else 0f
            scaleX = 1f - 0.06f * paused()
            scaleY = scaleX
        }
    ) {
        // The covers sit a gap apart, so a skip slides them one place.
        val gap = with(density) { 32.dp.roundToPx() }
        AnimatedContent(
            targetState = ui,
            contentKey = { it.item },
            transitionSpec = {
                val way = skip()
                slideInHorizontally(NMotion.spatialDefault()) { (it + gap) * way }
                    .togetherWith(slideOutHorizontally(NMotion.spatialDefault()) { -(it + gap) * way })
                    .using(SizeTransform(clip = false))
            },
            label = "cover",
        ) { shown ->
            Cover(
                artwork(shown.track),
                Modifier
                    .fillMaxSize()
                    .floating(shape),
                shape = shape,
                placeholder = if (shown.track.loaded) CoverPlaceholder.ALBUM else CoverPlaceholder.UNREAD,
            )
        }
    }
}

/**
 * Text fading through on a skip: the old leaves [shift] pixels the way the skip went, and the new
 * comes in from the other side once the old is gone. [way] is 1 to the next, -1 back.
 */
fun AnimatedContentTransitionScope<*>.fadeThrough(way: Int, shift: Int): ContentTransform =
    (fadeIn(NMotion.effectsDefault<Float>().delayed(FADE_THROUGH_MS)) +
        slideInHorizontally(NMotion.spatialDefault<IntOffset>().delayed(FADE_THROUGH_MS)) { shift * way })
        .togetherWith(fadeOut(NMotion.effectsFast()) + slideOutHorizontally(NMotion.spatialDefault()) { -shift * way })
        .using(SizeTransform(clip = false))

/** A number rolling on a skip: up to the next, down back, as [way] is 1 or -1. */
fun AnimatedContentTransitionScope<*>.roll(way: Int, shift: Int): ContentTransform =
    (fadeIn(NMotion.effectsDefault<Float>().delayed(FADE_THROUGH_MS)) +
        slideInVertically(NMotion.spatialDefault<IntOffset>().delayed(FADE_THROUGH_MS)) { shift * way })
        .togetherWith(fadeOut(NMotion.effectsFast()) + slideOutVertically(NMotion.spatialDefault()) { -shift * way })
        .using(SizeTransform(clip = false))

/** The height of what plays next, over the navigation bar. */
private val UpNextHeight = 52.dp

@Composable
private fun TopBar(ui: PlayerUi, actions: PlayerActions, modifier: Modifier) {
    Row(
        modifier
            .fillMaxWidth()
            .height(64.dp)
            .padding(horizontal = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        NIconButton(NIcons.Collapse, stringResource(R.string.close_player), actions::close, tint = colors.onSurface)
        val playingFrom = stringResource(R.string.playing_from)
        val origin = ui.origin?.let { dotted(it, if (ui.shuffle) stringResource(R.string.shuffled) else null) }
        Column(
            Modifier
                .weight(1f)
                .height(48.dp)
                .clip(RoundedCornerShape(24.dp))
                .then(
                    if (origin != null) {
                        Modifier
                            .tappable(actions::openOrigin)
                            .clearAndSetSemantics {
                                contentDescription = "$playingFrom $origin"
                                role = Role.Button
                            }
                    } else {
                        Modifier
                    }
                ),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            if (origin != null) {
                Text(playingFrom, style = text(12), color = colors.onSurfaceVariant, maxLines = 1)
                Text(
                    origin,
                    style = text(14, FontWeight.Bold),
                    color = colors.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        NIconButton(
            NIcons.More,
            stringResource(R.string.more_for, ui.track.title),
            actions::openMore,
            tint = colors.onSurface,
        )
    }
}

/**
 * The top bar of a player held sideways: where it plays from on one line, then the output, the
 * sleep timer and ⋮, which have no room below.
 */
@Composable
private fun SideTopBar(ui: PlayerUi, actions: PlayerActions, modifier: Modifier) {
    Row(
        modifier
            .bleed(8.dp)
            .height(48.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        NIconButton(NIcons.Collapse, stringResource(R.string.close_player), actions::close, tint = colors.onSurface)
        val origin = ui.origin
        val muted = colors.onSurfaceVariant
        val strong = colors.onSurface
        val shuffled = stringResource(R.string.shuffled)
        // The sentence around the name, wherever a language puts it.
        val sentence = stringResource(R.string.queue_playing_from, "\u0000").split('\u0000')
        val playingFrom = stringResource(R.string.playing_from)
        Box(
            Modifier
                .weight(1f)
                .height(48.dp)
                .then(
                    if (origin != null) {
                        Modifier
                            .clip(RoundedCornerShape(24.dp))
                            .tappable(actions::openOrigin)
                            .clearAndSetSemantics {
                                contentDescription = "$playingFrom ${dotted(origin, if (ui.shuffle) shuffled else null)}"
                                role = Role.Button
                            }
                    } else {
                        Modifier
                    }
                ),
            contentAlignment = Alignment.CenterStart,
        ) {
            if (origin != null) {
                Text(
                    buildAnnotatedString {
                        append(sentence[0])
                        withStyle(SpanStyle(color = strong, fontWeight = FontWeight.Bold)) { append(origin) }
                        append(sentence.getOrElse(1) { "" })
                        if (ui.shuffle) append(" · $shuffled")
                    },
                    style = text(13),
                    color = muted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        NIconButton(
            NIcons.Output,
            stringResource(R.string.output_named, ui.output),
            actions::openOutput,
            iconSize = 22.dp,
            tint = muted,
        )
        // Filled while a timer runs.
        val sleep = ui.sleep
        val sleepLabel = sleep?.description ?: stringResource(R.string.sleep_timer)
        Box(
            Modifier
                .size(48.dp)
                .clip(CircleShape)
                .tappable(actions::openSleepTimer)
                .semantics { contentDescription = sleepLabel },
            contentAlignment = Alignment.Center,
        ) {
            if (sleep != null) {
                Box(Modifier.size(40.dp).background(colors.secondaryContainer, CircleShape))
            }
            NIcon(NIcons.SleepTimer, size = 22.dp, tint = if (sleep != null) colors.onSecondaryContainer else muted)
        }
        NIconButton(
            NIcons.More,
            stringResource(R.string.more_for, ui.track.title),
            actions::openMore,
            tint = colors.onSurface,
        )
    }
}

/** Reaches [by] past both sides of its room, so the icons it starts and ends with line up with the text. */
private fun Modifier.bleed(by: Dp): Modifier = layout { measurable, constraints ->
    val extra = by.roundToPx()
    val width = constraints.maxWidth + 2 * extra
    val placeable = measurable.measure(constraints.copy(minWidth = width, maxWidth = width))
    layout(constraints.maxWidth, placeable.height) { placeable.place(-extra, 0) }
}

/** The queue, behind its button on a player held sideways. */
@Composable
private fun QueueButton(onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(20.dp)
    val description = stringResource(R.string.open_queue)
    Row(
        modifier
            .height(40.dp)
            .clip(shape)
            .border(1.dp, colors.outlineVariant, shape)
            .tappable(onClick)
            .clearAndSetSemantics {
                contentDescription = description
                role = Role.Button
            }
            // The outline takes 1 dp of its own outside the padding.
            .padding(start = 11.dp, end = 15.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        NIcon(NIcons.PlayNext, size = 18.dp, tint = colors.onSurfaceVariant)
        Text(stringResource(R.string.queue), style = text(13, FontWeight.SemiBold), color = colors.onSurfaceVariant, maxLines = 1)
    }
}

/** The title, the artist and album, each a link to its page, and the track's format and plays. */
@Composable
internal fun TitleBlock(ui: PlayerUi, actions: PlayerActions, metrics: PlayerMetrics) {
    val track = ui.track
    Column(
        Modifier
            .fillMaxWidth()
            .padding(start = metrics.side, end = metrics.side, top = metrics.titleTop)
    ) {
        Text(
            track.title,
            style = metrics.title,
            color = colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
        val line = metrics.line
        SharedLine(
            Modifier.padding(top = metrics.lineTop),
            first = {
                Link(track.artist.ifEmpty { stringResource(R.string.unknown_artist) }, line, actions::openArtist)
            },
            second = track.album?.let { album ->
                {
                    Row {
                        Text(" · ", style = line, color = colors.onSurfaceVariant, modifier = Modifier.clearAndSetSemantics {})
                        Link(album, line, actions::openAlbum)
                    }
                }
            },
        )
        val details = ui.details
        // The chips keep their room while the track's details are read, so nothing below moves.
        val small = metrics.small
        val chip = if (small) 26.dp else 28.dp
        val fill = if (small) colors.surfaceHigh else colors.surface
        val padding = if (small) 9.dp else 10.dp
        Row(
            Modifier
                .padding(top = metrics.chipsTop)
                .height(chip),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            if (details != null) {
                trackFormat(details)?.let { InfoChip(it, fill, height = chip, padding = padding) }
                InfoChip(
                    if (details.plays == 0u) {
                        stringResource(R.string.plays_never)
                    } else {
                        stringResource(R.string.played_count, details.plays.toInt())
                    },
                    fill,
                    height = chip,
                    padding = padding,
                )
            }
        }
    }
}

/** A link in the artist and album line, with a touch area taller than the line. */
@Composable
private fun Link(label: String, style: TextStyle, onClick: () -> Unit) {
    Text(
        label,
        style = style,
        color = colors.onSurfaceVariant,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier = Modifier
            .margins(top = 13.dp, bottom = 13.dp)
            .tappable(onClick, role = Role.Button)
            .padding(vertical = 13.dp),
    )
}

/**
 * [first] and [second] on one line, sharing it: the shorter keeps its whole width while it takes
 * no more than half, and the longer ends early.
 */
@Composable
private fun SharedLine(modifier: Modifier, first: @Composable () -> Unit, second: (@Composable () -> Unit)?) {
    Layout(
        contents = listOf(first, second ?: {}),
        modifier = modifier,
    ) { (firsts, seconds), constraints ->
        val width = constraints.maxWidth
        val a = firsts.firstOrNull()
        val b = seconds.firstOrNull()
        val wantA = a?.maxIntrinsicWidth(Constraints.Infinity) ?: 0
        val wantB = b?.maxIntrinsicWidth(Constraints.Infinity) ?: 0
        val (widthA, widthB) = when {
            wantA + wantB <= width -> wantA to wantB
            wantB <= width / 2 -> width - wantB to wantB
            wantA <= width / 2 -> wantA to width - wantA
            else -> width / 2 to width - width / 2
        }
        val placedA = a?.measure(Constraints(maxWidth = widthA))
        val placedB = b?.measure(Constraints(maxWidth = widthB))
        val height = maxOf(placedA?.height ?: 0, placedB?.height ?: 0)
        val size = IntSize((placedA?.width ?: 0) + (placedB?.width ?: 0), height)
        layout(size.width, size.height) {
            placedA?.place(0, 0)
            placedB?.place(placedA?.width ?: 0, 0)
        }
    }
}

/** Where it plays, opening Android's output switcher; [small] in a tablet's pane. */
@Composable
internal fun OutputChip(name: String, onClick: () -> Unit, small: Boolean = false) {
    val description = stringResource(R.string.output_named, name)
    val shape = RoundedCornerShape(if (small) 16.dp else 18.dp)
    Row(
        Modifier
            .height(if (small) 32.dp else 36.dp)
            .clip(shape)
            .border(1.dp, colors.outlineVariant, shape)
            .tappable(onClick)
            .clearAndSetSemantics {
                contentDescription = description
                role = Role.Button
            }
            // The outline takes 1 dp of its own outside the padding.
            .padding(start = 11.dp, end = if (small) 13.dp else 15.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(if (small) 6.dp else 8.dp),
    ) {
        NIcon(NIcons.Output, size = 18.dp, tint = colors.onSurfaceVariant)
        Text(name, style = text(13, FontWeight.SemiBold), color = colors.onSurfaceVariant, maxLines = 1)
    }
}

/** The sleep timer: filled with its time left while one runs; [small] in a tablet's pane. */
@Composable
internal fun SleepChip(sleep: Sleep?, onClick: () -> Unit, small: Boolean = false) {
    val shape = RoundedCornerShape(if (small) 16.dp else 18.dp)
    val left = sleep?.label
    val description = sleep?.description ?: stringResource(R.string.sleep_timer)
    Row(
        Modifier
            .height(if (small) 32.dp else 36.dp)
            .clip(shape)
            .then(
                if (left != null) {
                    Modifier.background(colors.secondaryContainer)
                } else {
                    Modifier.border(1.dp, colors.outlineVariant, shape)
                }
            )
            .tappable(onClick)
            .clearAndSetSemantics {
                contentDescription = description
                role = Role.Button
            }
            // The outline takes 1 dp of its own outside the padding, drawn or not.
            .padding(start = 11.dp, end = if (small) 13.dp else 15.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(if (small) 6.dp else 8.dp),
    ) {
        val tint = if (left != null) colors.onSecondaryContainer else colors.onSurfaceVariant
        NIcon(NIcons.SleepTimer, size = 18.dp, tint = tint)
        Text(
            left ?: stringResource(R.string.sleep_timer),
            style = text(13, if (left != null) FontWeight.Bold else FontWeight.SemiBold, tabular = true),
            color = tint,
            maxLines = 1,
        )
    }
}

/**
 * What plays next, on a handle at the bottom: tapping it or swiping it up opens the queue. A
 * swipe down is left to the player, which closes.
 */
@Composable
private fun UpNext(next: TrackRow?, onOpen: () -> Unit, modifier: Modifier, skip: () -> Int) {
    val description = stringResource(R.string.open_queue)
    Column(
        modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp))
            .background(colors.surfaceLow)
            .pointerInput(onOpen) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    awaitVerticalTouchSlopOrCancellation(down.id) { change, over ->
                        if (over < 0) change.consume()
                    }?.let { onOpen() }
                }
            }
            .tappable(onOpen)
            .semantics { contentDescription = description },
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            Modifier
                .padding(top = 10.dp)
                .size(32.dp, 4.dp)
                .background(colors.onSurfaceVariant.copy(alpha = 0.45f), RoundedCornerShape(2.dp))
        )
        Row(
            Modifier
                .fillMaxWidth()
                .height(UpNextHeight - 14.dp)
                .padding(start = 24.dp, end = 24.dp, top = 10.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(stringResource(R.string.up_next), style = text(13, FontWeight.Bold), color = colors.primary)
            val muted = colors.onSurfaceVariant
            val shift = with(LocalDensity.current) { 8.dp.roundToPx() }
            AnimatedContent(
                targetState = next,
                contentKey = { it?.locator },
                transitionSpec = { fadeThrough(skip(), shift) },
                modifier = Modifier.weight(1f),
                label = "next",
            ) { shown ->
                Text(
                    if (shown == null) {
                        buildAnnotatedString {
                            withStyle(SpanStyle(color = muted)) { append(stringResource(R.string.queue_end)) }
                        }
                    } else {
                        val artist = shown.artist.ifEmpty { stringResource(R.string.unknown_artist) }
                        buildAnnotatedString {
                            append(shown.title)
                            withStyle(SpanStyle(color = muted)) { append(" · $artist") }
                        }
                    },
                    style = text(14),
                    color = colors.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            NIcon(NIcons.Expand, size = 20.dp, tint = muted)
        }
        Spacer(Modifier.windowInsetsBottomHeight(WindowInsets.navigationBars))
    }
}
