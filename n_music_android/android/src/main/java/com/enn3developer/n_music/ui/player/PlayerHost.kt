package com.enn3developer.n_music.ui.player

import android.os.Build
import android.view.RoundedCorner
import android.view.View
import androidx.activity.compose.PredictiveBackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitVerticalTouchSlopOrCancellation
import androidx.compose.foundation.gestures.verticalDrag
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.geometry.isSpecified
import androidx.compose.ui.geometry.lerp
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.util.lerp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.PlayingFrom
import com.enn3developer.n_music.MiniButton
import com.enn3developer.n_music.R
import com.enn3developer.n_music.SleepMode
import com.enn3developer.n_music.SleepTimer
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.core.QueueRow
import com.enn3developer.n_music.core.TrackDetails
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.LocalWindowLayout
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.Snack
import com.enn3developer.n_music.ui.components.CoverPlaceholder
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.MiniPlayerRow
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.PlaybackActions
import com.enn3developer.n_music.ui.components.PlaybackUi
import com.enn3developer.n_music.ui.components.PullGesture
import com.enn3developer.n_music.ui.components.SnackbarHost
import com.enn3developer.n_music.ui.components.floating
import com.enn3developer.n_music.ui.components.miniProgress
import com.enn3developer.n_music.ui.components.miniShape
import com.enn3developer.n_music.ui.components.rememberPlaybackSeconds
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.rememberLibrary
import com.enn3developer.n_music.ui.sheets.Sheet
import com.enn3developer.n_music.ui.sheets.albumOf
import com.enn3developer.n_music.ui.sheets.rememberClock
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.delayed
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlin.math.roundToInt

/** How dark the scrim over the app gets behind the open player. */
private const val SCRIM = 0.5f

/** How small a drag down makes the player, around the finger. */
private const val SQUEEZED = 0.95f

/** How far a drag goes before the player is as small as it gets. */
private val SqueezeDistance = 200.dp

/** Letting go faster than this flicks the player. */
private val FlickSpeed = 400.dp

/** How far back's gesture drags the player by its end. */
private val BackDistance = 160.dp

/** How long after the player starts opening each part starts coming in: once the cover cleared it. */
private val RevealDelays = longArrayOf(50, 110, 140, 170, 200, 230)

/** How precisely the player's springs end, as a share of their way. */
private const val PRECISION = 0.0005f

/**
 * The player opening out of the mini player and closing back into it. The mini player is the
 * container: it grows to the whole screen while the navigation bar slides away, the cover flies
 * from the mini player's to its place and lands last, and the player's parts rise in once the
 * cover cleared them. A drag down follows the finger and shrinks the player around it; letting
 * go keeps the finger's speed into the mini player. A finger catches it anywhere on its way.
 */
@Stable
class PlayerTransition(private val scope: CoroutineScope) {
    /** The container, from the mini player's bounds at 0 to the whole screen at 1. */
    val expand = Animatable(0f, PRECISION)

    /** The cover, from the mini player's at 0 to its place in the player at 1. */
    val cover = Animatable(0f, PRECISION)

    /** How far a finger dragged the player down, in pixels. */
    val drag = Animatable(0f, 0.5f)

    /** The player's scale around the finger while it is dragged. */
    val squeeze = Animatable(1f, PRECISION)

    /** The scrim over the app, as its alpha. */
    val scrim = Animatable(0f)

    /** The container's colour, from the mini player's at 0 to the page's at 1. */
    val tone = Animatable(0f)

    /** What the mini player shows, inside the container while it grows. */
    val mini = Animatable(1f)

    /** The mini player's shadow, under the container. */
    val shadow = Animatable(1f)

    /** How far each of the player's parts came in, faded until 1... */
    val groups = PlayerGroup.entries.map { Animatable(0f) }

    /** ...and 12 dp low until it rose, at 0. */
    val rises = PlayerGroup.entries.map { Animatable(1f) }

    /** Where the finger took hold of the player, which shrinks around it. */
    var pivot by mutableStateOf(Offset.Unspecified)
        private set

    /** Where the mini player is, in the window's pixels, while it shows. */
    var miniBounds by mutableStateOf<Rect?>(null)

    /** The player is open, or opening. */
    var isOpen by mutableStateOf(false)
        private set

    /** A finger holds the player, or the mini player pulling it open. */
    var held by mutableStateOf(false)
        private set

    private var job: Job? = null
    private var heldDrag = 0f
    private var heldExpand = 0f

    /** Some of the player shows, so the mini player hides under it. */
    val shown: Boolean
        get() = isOpen || held || expand.value > 0f || cover.value > 0f || drag.value != 0f ||
            mini.value < 1f || shadow.value < 1f

    /** The cover is on its way between the mini player's and its place, drawn on its own. */
    val flying: Boolean
        get() = cover.value != 1f || cover.isRunning

    private val moving: Boolean
        get() = expand.isRunning || cover.isRunning || drag.isRunning || squeeze.isRunning

    /** Opens the player, at [velocity] of the way a second when a finger flicked it open. */
    fun open(velocity: Float = 0f) {
        isOpen = true
        held = false
        job?.cancel()
        job = scope.launch {
            launch { expand.animateTo(1f, NMotion.throwing(1f - expand.value, velocity, PRECISION), velocity) }
            launch { cover.animateTo(1f, NMotion.spatialSlow(PRECISION), velocity) }
            launch { drag.animateTo(0f, NMotion.spatialDefault(0.5f)) }
            launch { squeeze.animateTo(1f, NMotion.spatialDefault(PRECISION)) }
            launch { scrim.animateTo(SCRIM, NMotion.effectsDefault()) }
            launch { tone.animateTo(1f, NMotion.effectsDefault()) }
            launch { mini.animateTo(0f, NMotion.effectsFast()) }
            launch { shadow.animateTo(0f, NMotion.effectsFast()) }
            for (index in groups.indices) {
                val delay = RevealDelays[index]
                launch { groups[index].animateTo(1f, NMotion.effectsDefault<Float>().delayed(delay)) }
                launch { rises[index].animateTo(0f, NMotion.spatialDefault<Float>().delayed(delay)) }
            }
        }
    }

    /**
     * Closes the player into the mini player, at [velocity] of the way a second: the container,
     * the cover and the drag all share one curve, so the point a finger held keeps its speed.
     */
    fun close(velocity: Float = 0f) {
        isOpen = false
        held = false
        job?.cancel()
        job = scope.launch {
            val speed = velocity.coerceAtLeast(0f)
            coroutineScope {
                launch { expand.animateTo(0f, NMotion.throwing(1f, speed, PRECISION), -speed * expand.value) }
                launch { cover.animateTo(0f, NMotion.throwing(1f, speed, PRECISION), -speed * cover.value) }
                launch { drag.animateTo(0f, NMotion.throwing(1f, speed, 0.5f), -speed * drag.value) }
                launch {
                    squeeze.animateTo(1f, NMotion.throwing(1f, speed, PRECISION), speed * (1f - squeeze.value))
                }
                launch { scrim.animateTo(0f, NMotion.effectsDefault()) }
                launch { tone.animateTo(0f, NMotion.effectsDefault<Float>().delayed(60)) }
                launch { mini.animateTo(1f, NMotion.effectsDefault<Float>().delayed(150)) }
                launch { shadow.animateTo(1f, NMotion.effectsDefault<Float>().delayed(260)) }
                for (group in groups) launch { group.animateTo(0f, NMotion.effectsFast()) }
            }
            for (rise in rises) rise.snapTo(1f)
        }
    }

    /** A finger took hold of the player [at] a point: what moved stops under it. */
    fun hold(at: Offset) {
        job?.cancel()
        held = true
        heldDrag = drag.value
        heldExpand = expand.value
        if (squeeze.value == 1f) pivot = at
    }

    /** Lets go of a hold that never dragged: the player goes on where it was going. */
    fun resume() {
        if (isOpen) open() else close()
    }

    /**
     * The finger holding the player moved [total] pixels down since it took hold. Down drags the
     * player and shrinks it; up opens what was not open yet, over [travel] pixels.
     */
    fun dragTo(total: Float, travel: Float, squeezeDistance: Float) {
        val down = heldDrag + total
        val dragged = down.coerceAtLeast(0f)
        val opened = if (down < 0f) (heldExpand - down / travel).coerceIn(0f, 1f) else heldExpand
        val squeezed = 1f - (1f - SQUEEZED) * (dragged / squeezeDistance).coerceIn(0f, 1f)
        scope.launch {
            drag.snapTo(dragged)
            expand.snapTo(opened)
            squeeze.snapTo(squeezed)
        }
    }

    /**
     * The finger let go, moving [velocity] pixels a second down. A flick down, or a drag past a
     * quarter of the [height], closes the player, keeping the speed of the point held, which
     * has [distance] pixels to go into the mini player. Anything else puts it back.
     */
    fun letGo(velocity: Float, flick: Float, height: Float, distance: Float) {
        held = false
        val far = drag.value > height / 4
        if (velocity > flick || (far && velocity > -flick)) {
            close(velocity / distance.coerceAtLeast(1f))
        } else {
            open()
        }
    }

    /** A drag up on the mini player pulling the player open, [travel] pixels from closed to open. */
    fun pullGesture(flick: Float, travel: () -> Float): PullGesture = object : PullGesture {
        private var total = 0f

        override fun start() {
            job?.cancel()
            held = true
            heldExpand = expand.value
            total = 0f
        }

        override fun pull(dy: Float) {
            total += dy
            val opened = (heldExpand - total / travel()).coerceIn(0f, 1f)
            scope.launch {
                expand.snapTo(opened)
                cover.snapTo(opened)
                scrim.snapTo(SCRIM * opened)
                tone.snapTo(opened)
                // What the mini player shows is gone a quarter of the way up.
                mini.snapTo((1f - 4 * opened).coerceAtLeast(0f))
                shadow.snapTo((1f - 4 * opened).coerceAtLeast(0f))
            }
        }

        override fun end(velocity: Float) {
            held = false
            val speed = -velocity / travel()
            if (velocity < -flick || (expand.value > 0.3f && velocity < flick)) {
                open(speed.coerceAtLeast(0f))
            } else {
                close((-speed).coerceAtLeast(0f))
            }
        }
    }
}

/** The outline of the container: [rect] with [radius] corners, wherever the layer is. */
private class PanelShape(private val rect: Rect, private val radius: Float) : Shape {
    override fun createOutline(size: Size, layoutDirection: LayoutDirection, density: Density): Outline =
        Outline.Rounded(RoundRect(rect, CornerRadius(radius)))
}

/** Rounded corners whose [radius] is read when drawn, for a cover changing size on its flight. */
private class LiveCorners(private val radius: () -> Float) : Shape {
    override fun createOutline(size: Size, layoutDirection: LayoutDirection, density: Density): Outline =
        Outline.Rounded(RoundRect(Rect(Offset.Zero, size), CornerRadius(radius())))
}

/** The radius of the screen's own corners, which the open player's container takes. */
private fun screenCorner(view: View): Float {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) return 0f
    val insets = view.rootWindowInsets ?: return 0f
    return insets.getRoundedCorner(RoundedCorner.POSITION_TOP_LEFT)?.radius?.toFloat() ?: 0f
}

/**
 * The player over the app while any of it shows: its scrim, the container growing out of the
 * mini player with the player in it, and the cover on its flight.
 */
@Composable
fun PlayerHost(transition: PlayerTransition) {
    val shown by remember(transition) { derivedStateOf { transition.shown } }
    if (!shown) return
    val current by CoreRepository.current.collectAsStateWithLifecycle()
    val track = current?.track
    LaunchedEffect(track == null) { if (track == null) transition.close() }
    if (track == null) return

    val app = LocalApp.current
    val settings by UiPreferences.settings.collectAsStateWithLifecycle()
    val playing by CoreRepository.playing.collectAsStateWithLifecycle()
    val shuffle by CoreRepository.shuffle.collectAsStateWithLifecycle()
    val loop by CoreRepository.loopStatus.collectAsStateWithLifecycle()
    val position by CoreRepository.position.collectAsStateWithLifecycle()
    val queue by CoreRepository.queue.collectAsStateWithLifecycle()
    val origin by PlayingFrom.origin.collectAsStateWithLifecycle()
    val details = rememberLibrary<TrackDetails?>(null, track.locator) { CoreRepository.details(track.locator) }
    val seconds = rememberPlaybackSeconds(position, playing)
    // A seek sent and not yet applied: what the player shows meanwhile.
    val pending = remember { mutableStateOf<Pair<ULong, Double>?>(null) }
    LaunchedEffect(position) {
        val seek = pending.value
        if (seek != null && position.seek >= seek.first) pending.value = null
    }
    val length = position.length.takeIf { it > 0 } ?: track.length
    val skips = remember { SkipTracker() }
    skips.update(current?.item, queue)
    val ui = PlayerUi(
        item = current?.item ?: 0u,
        track = track,
        details = details,
        playing = playing,
        shuffle = shuffle,
        loop = loop,
        origin = originName(origin)?.let { dotted(it, if (shuffle) stringResource(R.string.shuffled) else null) },
        next = upNext(queue, current?.item, loop),
        output = rememberOutputName(),
        sleep = sleepLabel(),
    )
    val latestTrack by rememberUpdatedState(track)
    val latestOrigin by rememberUpdatedState(origin)
    val actions = remember(app, transition) {
        object : PlayerActions, PlaybackActions by app.playback {
            override fun close() = transition.close()

            override fun openOrigin() {
                latestOrigin?.let { app.openOrigin(it) }
            }

            override fun openMore() = app.show(Sheet.TrackActions(latestTrack.locator))

            override fun openArtist() = app.open(Page.Artist(latestTrack.artists.firstOrNull()))

            override fun openAlbum() = app.open(albumOf(latestTrack))

            override fun seek(seconds: Double) {
                pending.value = CoreRepository.seek(seconds) to seconds
            }

            override fun openQueue() = app.show(Sheet.Queue)
        }
    }
    val width = LocalWindowInfo.current.containerSize.width
    val size = width - with(LocalDensity.current) { 64.dp.roundToPx() }
    PlayerOverlay(
        transition = transition,
        ui = ui,
        artwork = { rememberArtwork(it, size) },
        skip = { skips.way },
        seconds = { pending.value?.second ?: seconds.value },
        length = length,
        actions = actions,
        miniButtons = settings.miniButtons,
        // Over a sheet, the app shows it.
        snack = app.snack.takeIf { app.sheet == null },
        onSnackTimeout = app::dismissSnack,
        onSnackAction = app::snackAction,
    )
}

/**
 * The player over the app, for [ui] as it plays: its scrim, the container growing out of the
 * mini player with the player in it, and the cover on its flight, as [transition] has them.
 */
@Composable
fun PlayerOverlay(
    transition: PlayerTransition,
    ui: PlayerUi,
    artwork: @Composable (TrackRow) -> ImageBitmap?,
    skip: () -> Int,
    seconds: () -> Double,
    length: Double,
    actions: PlayerActions,
    miniButtons: List<MiniButton>,
    snack: Pair<Long, Snack>?,
    onSnackTimeout: (Long) -> Unit,
    onSnackAction: (Long) -> Unit,
) {
    val track = ui.track
    val density = LocalDensity.current
    // Beside a rail, the mini player is the taller bar.
    val bar = LocalWindowLayout.current.rail
    val view = LocalView.current
    val corner = remember(view) { screenCorner(view) }
    val miniColor = colors.surfaceHigh
    val pageColor = colors.background
    val nowPlaying = stringResource(R.string.now_playing)
    var slot by remember { mutableStateOf<Rect?>(null) }

    BoxWithConstraints(
        Modifier
            .fillMaxSize()
            // Nothing under the player takes touches while any of it shows.
            .pointerInput(Unit) {}
    ) {
        val width = constraints.maxWidth.toFloat()
        val height = constraints.maxHeight.toFloat()
        val navInset = WindowInsets.navigationBars.getBottom(density).toFloat()
        val mini = transition.miniBounds ?: with(density) {
            Rect(8.dp.toPx(), height - navInset - 72.dp.toPx(), width - 8.dp.toPx(), height - navInset - 8.dp.toPx())
        }
        val full = Rect(0f, 0f, width, height)
        val miniCover = with(density) {
            val side = (if (bar) 56.dp else 48.dp).toPx()
            Rect(Offset(mini.left + 8.dp.toPx(), mini.top + 8.dp.toPx()), Size(side, side))
        }
        val squeezeDistance = with(density) { SqueezeDistance.toPx() }
        val miniRadius = with(density) { (if (bar) 20.dp else 16.dp).toPx() }
        fun panel(): Rect = lerp(mini, full, transition.expand.value)
        fun dragged(): Float = (transition.drag.value / squeezeDistance).coerceIn(0f, 1f)

        // The scrim, lighter while a drag takes the player away.
        Box(
            Modifier
                .fillMaxSize()
                .graphicsLayer { alpha = transition.scrim.value * (1f - 0.3f * dragged()) }
                .background(Color.Black)
        )
        // The mini player's shadow, under the container while it leaves the mini player.
        Box(
            Modifier
                .offset { IntOffset(mini.left.roundToInt(), mini.top.roundToInt()) }
                .layout { measurable, _ ->
                    val placeable = measurable.measure(Constraints.fixed(mini.width.roundToInt(), mini.height.roundToInt()))
                    layout(placeable.width, placeable.height) { placeable.place(0, 0) }
                }
                .graphicsLayer { alpha = transition.shadow.value }
                .floating(miniShape(bar))
        )
        Box(
            Modifier
                .fillMaxSize()
                .graphicsLayer {
                    translationY = transition.drag.value
                    scaleX = transition.squeeze.value
                    scaleY = transition.squeeze.value
                    val pivot = transition.pivot
                    transformOrigin = if (pivot.isSpecified) {
                        TransformOrigin(pivot.x / size.width, pivot.y / size.height)
                    } else {
                        TransformOrigin.Center
                    }
                    shape = PanelShape(panel(), lerp(miniRadius, corner, transition.expand.value.coerceIn(0f, 1f)))
                    clip = true
                }
                .drawBehind { drawRect(lerp(miniColor, pageColor, transition.tone.value.coerceIn(0f, 1f))) }
                .pointerInput(transition, mini) {
                    val flick = FlickSpeed.toPx()
                    awaitEachGesture {
                        val down = awaitFirstDown(requireUnconsumed = false)
                        val moving = transition.expand.isRunning || transition.cover.isRunning ||
                            transition.drag.isRunning || transition.squeeze.isRunning
                        if (moving) transition.hold(down.position)
                        var over = 0f
                        val start = awaitVerticalTouchSlopOrCancellation(down.id) { change, slop ->
                            change.consume()
                            over = slop
                        }
                        if (start == null) {
                            if (moving) transition.resume()
                            return@awaitEachGesture
                        }
                        if (!moving) transition.hold(down.position)
                        val tracker = VelocityTracker()
                        var total = over
                        tracker.addPosition(start.uptimeMillis, Offset(0f, total))
                        transition.dragTo(total, mini.top, squeezeDistance)
                        verticalDrag(start.id) { change ->
                            total += change.positionChange().y
                            tracker.addPosition(change.uptimeMillis, Offset(0f, total))
                            transition.dragTo(total, mini.top, squeezeDistance)
                            change.consume()
                        }
                        // The point held heads for the middle of the mini player's cover.
                        val held = transition.pivot.takeIf { it.isSpecified }?.y ?: down.position.y
                        val distance = miniCover.center.y - (held + transition.drag.value)
                        transition.letGo(tracker.calculateVelocity().y, flick, height, distance)
                    }
                }
                .semantics { paneTitle = nowPlaying },
        ) {
            // What the mini player shows, on the container's top edge until it fades.
            MiniPlayerRow(
                track = track,
                ui = PlaybackUi(track, ui.playing, ui.shuffle, ui.loop, ui.sleep != null),
                buttons = miniButtons,
                actions = actions,
                onOpen = {},
                showCover = false,
                bar = bar,
                modifier = Modifier
                    .offset { IntOffset(mini.left.roundToInt(), panel().top.roundToInt()) }
                    .layout { measurable, _ ->
                        val placeable = measurable.measure(Constraints.fixed(mini.width.roundToInt(), mini.height.roundToInt()))
                        layout(placeable.width, placeable.height) { placeable.place(0, 0) }
                    }
                    .graphicsLayer { alpha = transition.mini.value }
                    .miniProgress({ if (length > 0) (seconds() / length).toFloat() else 0f }, if (bar) 16.dp else 12.dp)
                    .clearAndSetSemantics {},
            ) {
                if (bar) {
                    Box(Modifier.size(48.dp), contentAlignment = Alignment.Center) {
                        NIcon(NIcons.PlayNext, tint = colors.onSurfaceVariant)
                    }
                }
            }
            PlayerContent(
                ui = ui,
                artwork = artwork,
                seconds = seconds,
                length = length,
                actions = actions,
                skip = skip,
                modifier = Modifier.offset { IntOffset(0, panel().top.roundToInt()) },
                group = { part ->
                    Modifier.graphicsLayer {
                        alpha = transition.groups[part.ordinal].value.coerceIn(0f, 1f) * (1f - 0.4f * dragged())
                        if (part != PlayerGroup.UP_NEXT) {
                            translationY = transition.rises[part.ordinal].value * 12.dp.toPx()
                        }
                    }
                },
                coverShown = { !transition.flying },
                onCoverPlaced = { slot = it },
            )
            // The cover on its flight between the mini player's and its place.
            val landing = slot
            if (landing != null) {
                val coverShape = remember(density) {
                    LiveCorners { with(density) { lerp((if (bar) 12.dp else 10.dp).toPx(), 28.dp.toPx(), transition.cover.value) } }
                }
                Box(
                    Modifier
                        .offset {
                            val rect = lerp(miniCover, landing, transition.cover.value)
                            IntOffset(rect.left.roundToInt(), rect.top.roundToInt())
                        }
                        .layout { measurable, _ ->
                            val rect = lerp(miniCover, landing, transition.cover.value)
                            val placeable = measurable.measure(
                                Constraints.fixed(rect.width.roundToInt().coerceAtLeast(0), rect.height.roundToInt().coerceAtLeast(0))
                            )
                            layout(placeable.width, placeable.height) { placeable.place(0, 0) }
                        }
                        .graphicsLayer { alpha = if (transition.flying) 1f else 0f }
                        .floating(coverShape)
                ) {
                    Cover(
                        artwork(track),
                        Modifier.fillMaxSize(),
                        shape = coverShape,
                        placeholder = if (track.loaded) CoverPlaceholder.ALBUM else CoverPlaceholder.UNREAD,
                    )
                }
            }
        }
        // Messages show over the player while it is open, above what plays next.
        SnackbarHost(
            if (transition.isOpen) snack else null,
            onSnackTimeout,
            onSnackAction,
            Modifier
                .align(Alignment.BottomCenter)
                .padding(bottom = with(density) { navInset.toDp() } + 52.dp + 8.dp),
        )

        // Back shrinks the player as a drag would, and closes it once let go.
        PredictiveBackHandler(transition.isOpen) { progress ->
            try {
                var started = false
                progress.collect { event ->
                    if (!started) {
                        started = true
                        transition.hold(Offset(width / 2, event.touchY))
                    }
                    transition.dragTo(event.progress * with(density) { BackDistance.toPx() }, mini.top, squeezeDistance)
                }
                transition.close()
            } catch (cancelled: CancellationException) {
                transition.open()
                throw cancelled
            }
        }
    }
}

/** The sleep timer as the player's chip says it, ticking while one runs. */
@Composable
private fun sleepLabel(): Sleep? {
    val state by SleepTimer.state.collectAsStateWithLifecycle()
    val running = state ?: return null
    val left = SleepTimer.remaining(running, rememberClock())
    return when {
        running.ends == null && running.option == SleepMode.TrackEnd -> Sleep(
            stringResource(R.string.sleep_at_track_end),
            stringResource(R.string.sleep_stops_at_track_end),
        )
        running.ends == null -> Sleep(
            stringResource(R.string.sleep_at_queue_end),
            stringResource(R.string.sleep_stops_at_queue_end),
        )
        else -> {
            // 24:12 left says 24 min, and the last minute still says 1.
            val minutes = ((left ?: 0L) / 60_000).toInt().coerceAtLeast(1)
            Sleep(
                stringResource(R.string.sleep_minutes_left, minutes),
                pluralStringResource(R.plurals.sleep_stops_in, minutes, minutes),
            )
        }
    }
}

/** Tells which way playback went from one item to the next: on through the queue, or back. */
private class SkipTracker {
    private var last: ULong? = null

    /** 1 when the last change went on, -1 when it went back. */
    var way = 1
        private set

    fun update(item: ULong?, queue: List<QueueRow>) {
        val from = last
        if (item == from) return
        last = item
        if (from == null || item == null) return
        val old = queue.indexOfFirst { it.item == from }
        val new = queue.indexOfFirst { it.item == item }
        way = when {
            old < 0 || new < 0 -> 1
            // Round from the end to the start on repeat is on; from the start to the end, back.
            new == 0 && old == queue.lastIndex -> 1
            new == queue.lastIndex && old == 0 -> -1
            new < old -> -1
            else -> 1
        }
    }
}

/** What plays after [current] in [queue]: the next item, or the first once at its end on repeat. */
private fun upNext(queue: List<QueueRow>, current: ULong?, loop: LoopStatus): TrackRow? {
    val index = queue.indexOfFirst { it.item == current }
    if (index < 0) return null
    return queue.getOrNull(index + 1)?.track ?: if (loop == LoopStatus.PLAYLIST) queue.firstOrNull()?.track else null
}
