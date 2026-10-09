package com.enn3developer.n_music.ui.player

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.ContentTransform
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.Animatable
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
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsBottomHeight
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.unit.toSize
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.core.TrackDetails
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.CoverPlaceholder
import com.enn3developer.n_music.ui.components.InfoChip
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.NIconButton
import com.enn3developer.n_music.ui.components.PlaybackActions
import com.enn3developer.n_music.ui.components.floating
import com.enn3developer.n_music.ui.components.margins
import com.enn3developer.n_music.ui.components.tappable
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
    /** A running sleep timer's time left, as its chip says it; `null` while none runs. */
    val sleep: String?,
)

/** The covers' corners on the player. */
val PlayerCoverShape = RoundedCornerShape(28.dp)

/** How long new text waits for the old to leave when it fades through. */
private const val FADE_THROUGH_MS = 90L

/**
 * The player: where it plays from, the cover, the track with its artist and album, the position,
 * the controls, the output and sleep timer, and what plays next. [artwork] gives a track's
 * cover. A skip slides the covers one place, the way [skip] says it went: 1 to the next, -1 back,
 * and the text fades through. Pausing shrinks the cover and flattens the wave. [group] gives each
 * part the look of its way in; [coverShown] hides the cover while another draws it flying, and
 * [onCoverPlaced] tells where it is, from the player's top left corner.
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
) {
    val root = remember { arrayOfNulls<LayoutCoordinates>(1) }
    val density = LocalDensity.current
    // From 0 while playing to 1 while paused: the cover shrinks to 94% and the wave flattens.
    val paused = remember { Animatable(if (ui.playing) 0f else 1f) }
    LaunchedEffect(ui.playing) { paused.animateTo(if (ui.playing) 0f else 1f, NMotion.spatialSlow()) }
    // The wave travels a wavelength a second while playing, and stands still while paused or
    // while animations are off.
    val phase = remember { mutableFloatStateOf(0f) }
    LaunchedEffect(ui.playing) {
        if (!ui.playing || coroutineContext[MotionDurationScale]?.scaleFactor == 0f) return@LaunchedEffect
        var last = withFrameNanos { it }
        while (true) {
            withFrameNanos { now ->
                phase.floatValue = (phase.floatValue + (now - last) / 1e9f) % 1f
                last = now
            }
        }
    }
    Box(modifier.fillMaxSize()) {
        Column(
            Modifier
                .fillMaxSize()
                .onPlaced { root[0] = it }
                .windowInsetsPadding(WindowInsets.statusBars)
        ) {
            TopBar(ui, actions, Modifier.then(group(PlayerGroup.TOP_BAR)))
            Box(
                Modifier
                    .weight(1f, fill = false)
                    .padding(start = 32.dp, end = 32.dp, top = 12.dp)
                    .aspectRatio(1f)
                    .align(Alignment.CenterHorizontally)
                    .onPlaced { placed ->
                        root[0]?.let { onCoverPlaced(Rect(it.localPositionOf(placed, Offset.Zero), placed.size.toSize())) }
                    }
                    .graphicsLayer {
                        alpha = if (coverShown()) 1f else 0f
                        scaleX = 1f - 0.06f * paused.value
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
                            .floating(PlayerCoverShape),
                        shape = PlayerCoverShape,
                        placeholder = if (shown.track.loaded) CoverPlaceholder.ALBUM else CoverPlaceholder.UNREAD,
                    )
                }
            }
            val shift = with(density) { 12.dp.roundToPx() }
            AnimatedContent(
                targetState = ui,
                contentKey = { it.item },
                transitionSpec = { fadeThrough(skip(), shift) },
                modifier = Modifier.then(group(PlayerGroup.TITLE)),
                label = "title",
            ) { shown ->
                TitleBlock(shown, actions)
            }
            SeekBar(
                seconds = seconds,
                length = length,
                onSeek = actions::seek,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(start = 32.dp, end = 32.dp, top = 20.dp)
                    .then(group(PlayerGroup.SEEK)),
                item = ui.item,
                skip = skip,
                wave = { 1f - 0.98f * paused.value },
                phase = { phase.floatValue },
            )
            PlayerControls(
                playing = ui.playing,
                shuffle = ui.shuffle,
                loop = ui.loop,
                actions = actions,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(start = 32.dp, end = 32.dp, top = 20.dp)
                    .then(group(PlayerGroup.CONTROLS)),
            )
            Row(
                Modifier
                    .fillMaxWidth()
                    .padding(top = 24.dp)
                    .then(group(PlayerGroup.CHIPS)),
                horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
            ) {
                OutputChip(ui.output, actions::openOutput)
                SleepChip(ui.sleep, actions::openSleepTimer)
            }
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
        Column(
            Modifier
                .weight(1f)
                .height(48.dp)
                .clip(RoundedCornerShape(24.dp))
                .then(
                    if (ui.origin != null) {
                        Modifier
                            .tappable(actions::openOrigin)
                            .clearAndSetSemantics {
                                contentDescription = "$playingFrom ${ui.origin}"
                                role = Role.Button
                            }
                    } else {
                        Modifier
                    }
                ),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            if (ui.origin != null) {
                Text(playingFrom, style = text(12), color = colors.onSurfaceVariant, maxLines = 1)
                Text(
                    ui.origin,
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

/** The title, the artist and album, each a link to its page, and the track's format and plays. */
@Composable
private fun TitleBlock(ui: PlayerUi, actions: PlayerActions) {
    val track = ui.track
    Column(
        Modifier
            .fillMaxWidth()
            .padding(start = 32.dp, end = 32.dp, top = 24.dp)
    ) {
        Text(
            track.title,
            style = NType.titleLarge,
            color = colors.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
        val line = text(16, lineHeight = 22.sp)
        SharedLine(
            Modifier.padding(top = 4.dp),
            first = {
                Link(track.artist.ifEmpty { stringResource(R.string.unknown_artist) }, actions::openArtist)
            },
            second = track.album?.let { album ->
                {
                    Row {
                        Text(" · ", style = line, color = colors.onSurfaceVariant, modifier = Modifier.clearAndSetSemantics {})
                        Link(album, actions::openAlbum)
                    }
                }
            },
        )
        val details = ui.details
        // The chips keep their room while the track's details are read, so nothing below moves.
        Row(
            Modifier
                .padding(top = 14.dp)
                .height(28.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            if (details != null) {
                trackFormat(details)?.let { InfoChip(it, colors.surface) }
                InfoChip(
                    if (details.plays == 0u) {
                        stringResource(R.string.plays_never)
                    } else {
                        stringResource(R.string.played_count, details.plays.toInt())
                    },
                    colors.surface,
                )
            }
        }
    }
}

/** A link in the artist and album line, with a touch area taller than the line. */
@Composable
private fun Link(label: String, onClick: () -> Unit) {
    Text(
        label,
        style = text(16, lineHeight = 22.sp),
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

/** Where it plays, opening Android's output switcher. */
@Composable
private fun OutputChip(name: String, onClick: () -> Unit) {
    val description = stringResource(R.string.output_named, name)
    Row(
        Modifier
            .height(36.dp)
            .clip(RoundedCornerShape(18.dp))
            .border(1.dp, colors.outlineVariant, RoundedCornerShape(18.dp))
            .tappable(onClick)
            .clearAndSetSemantics {
                contentDescription = description
                role = Role.Button
            }
            .padding(start = 10.dp, end = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        NIcon(NIcons.Output, size = 18.dp, tint = colors.onSurfaceVariant)
        Text(name, style = text(13, FontWeight.SemiBold), color = colors.onSurfaceVariant, maxLines = 1)
    }
}

/** The sleep timer: filled with its time left while one runs. */
@Composable
private fun SleepChip(left: String?, onClick: () -> Unit) {
    val shape = RoundedCornerShape(18.dp)
    Row(
        Modifier
            .height(36.dp)
            .clip(shape)
            .then(
                if (left != null) {
                    Modifier.background(colors.secondaryContainer)
                } else {
                    Modifier.border(1.dp, colors.outlineVariant, shape)
                }
            )
            .tappable(onClick)
            .padding(start = 10.dp, end = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
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
