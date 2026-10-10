package com.enn3developer.n_music.ui.player

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.exclude
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.Placeable
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.isTraversalGroup
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.Current
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.core.QueueRow
import com.enn3developer.n_music.core.Seek
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.ui.components.Cover
import com.enn3developer.n_music.ui.components.CoverPlaceholder
import com.enn3developer.n_music.ui.components.TextAction
import com.enn3developer.n_music.ui.components.margins
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** The pane's width beside the rail. */
private val PaneWidth = 352.dp

/** The pane's width beside the open drawer. */
private val NarrowPaneWidth = 336.dp

/** How tall the pane's parts are but for the cover and what plays next, as it lays them out. */
private val PaneParts = 378.dp

/** A row of what plays next. */
private val UpNextRow = 50.dp

/** The pane's covers' corners. */
private val PaneCoverShape = RoundedCornerShape(24.dp)

/** How much of the bottom inset the pane's margin takes in: a gesture area's, as the design has it. */
private val GestureArea = 16.dp

/**
 * What plays, in a pane beside a tablet's pages: where it plays from, the cover, the track, the
 * position, the controls, the output and sleep timer, and what plays next. [narrow] beside the
 * open drawer. The last track stays while the pane leaves once nothing plays.
 */
@Composable
fun NowPlayingPane(narrow: Boolean, modifier: Modifier = Modifier) {
    val current by CoreRepository.current.collectAsStateWithLifecycle()
    val last = remember { arrayOfNulls<Current>(1) }
    current?.let { last[0] = it }
    val now = last[0] ?: return
    val live = livePlayback(now, close = {})
    val size = with(LocalDensity.current) { PaneWidth.roundToPx() }
    NowPlayingPane(
        ui = live.ui,
        queue = live.queue,
        artwork = { rememberArtwork(it, size) },
        seconds = live.seconds,
        length = live.length,
        actions = live.actions,
        narrow = narrow,
        onPlay = { CoreRepository.send(Command.Seek(Seek.ToItem(it.item, 0.0))) },
        modifier = modifier,
        skip = live.skip,
    )
}

/**
 * The pane itself, for [ui] as it plays from [queue]. A skip slides the covers and fades the text
 * through, the way [skip] says it went; tapping a row of what plays next plays it with [onPlay].
 */
@Composable
fun NowPlayingPane(
    ui: PlayerUi,
    queue: List<QueueRow>,
    artwork: @Composable (TrackRow) -> ImageBitmap?,
    seconds: () -> Double,
    length: Double,
    actions: PlayerActions,
    narrow: Boolean,
    onPlay: (QueueRow) -> Unit,
    modifier: Modifier = Modifier,
    skip: () -> Int = { 1 },
) {
    val width by animateDpAsState(if (narrow) NarrowPaneWidth else PaneWidth, NMotion.spatialDefault(), label = "pane")
    val side by animateDpAsState(if (narrow) 22.dp else 24.dp, NMotion.spatialDefault(), label = "paneSide")
    val largest by animateDpAsState(if (narrow) 240.dp else 256.dp, NMotion.spatialDefault(), label = "paneCover")
    val metrics = if (narrow) PlayerMetrics.NarrowPane else PlayerMetrics.Pane
    val paused = rememberPaused(ui.playing)
    val phase = rememberWavePhase(ui.playing)
    val title = stringResource(R.string.now_playing)
    BoxWithConstraints(
        modifier
            .fillMaxHeight()
            .windowInsetsPadding(WindowInsets.statusBars)
            .windowInsetsPadding(WindowInsets.navigationBars.only(WindowInsetsSides.Bottom).exclude(WindowInsets(bottom = GestureArea)))
            // The margin is outside the pane's width.
            .padding(top = 4.dp, end = 12.dp, bottom = 12.dp)
            .width(width)
            .clip(RoundedCornerShape(28.dp))
            .background(colors.surfaceLow)
            .semantics {
                paneTitle = title
                isTraversalGroup = true
            }
            .padding(start = side, end = side, top = 20.dp, bottom = 16.dp),
    ) {
        // The cover gives way on a short screen, keeping room for one row of what plays next.
        val cover = (maxHeight - PaneParts - UpNextRow).coerceIn(120.dp, largest)
        Column(Modifier.fillMaxHeight()) {
            Heading(ui, actions::openOrigin)
            PlayerCover(
                ui, artwork, skip, { paused.value }, { true },
                Modifier
                    .padding(top = 14.dp)
                    .size(cover)
                    .align(Alignment.CenterHorizontally),
                shape = PaneCoverShape,
            )
            val shift = with(LocalDensity.current) { 12.dp.roundToPx() }
            AnimatedContent(
                targetState = ui,
                contentKey = { it.item },
                transitionSpec = { fadeThrough(skip(), shift) },
                label = "paneTitle",
            ) { shown ->
                TitleBlock(shown, actions, metrics)
            }
            SeekBar(
                seconds = seconds,
                length = length,
                onSeek = actions::seek,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = metrics.seekTop),
                item = ui.item,
                skip = skip,
                wave = { 1f - 0.98f * paused.value },
                phase = { phase.floatValue },
                compact = true,
            )
            PlayerControls(
                playing = ui.playing,
                shuffle = ui.shuffle,
                loop = ui.loop,
                actions = actions,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = metrics.controlsTop),
                sizes = metrics.controls,
            )
            Row(
                Modifier
                    .fillMaxWidth()
                    .padding(top = 14.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
            ) {
                OutputChip(ui.output, actions::openOutput, small = true)
                SleepChip(ui.sleep, actions::openSleepTimer, small = true)
            }
            Row(
                Modifier
                    .fillMaxWidth()
                    .padding(top = if (narrow) 16.dp else 14.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    stringResource(R.string.up_next),
                    style = text(14, FontWeight.ExtraBold),
                    color = colors.onSurface,
                    modifier = Modifier.weight(1f),
                )
                TextAction(
                    stringResource(R.string.open_queue_short),
                    actions::openQueue,
                    height = 32.dp,
                    textStyle = text(13, FontWeight.Bold),
                    padding = PaddingValues(horizontal = 8.dp),
                )
            }
            UpNextList(upcoming(queue, ui.item, ui.loop), onPlay, Modifier.weight(1f))
        }
    }
}

/** What plays after [current] in [queue], in the order it plays: round to the start on repeat. */
private fun upcoming(queue: List<QueueRow>, current: ULong, loop: LoopStatus): List<QueueRow> {
    val index = queue.indexOfFirst { it.item == current }
    if (index < 0) return emptyList()
    val after = queue.drop(index + 1)
    return if (loop == LoopStatus.PLAYLIST) after + queue.take(index) else after
}

/**
 * Now playing, from where and whether shuffled: tapping it opens the page of what plays, when
 * that is known.
 */
@Composable
private fun Heading(ui: PlayerUi, onOrigin: () -> Unit) {
    val accent = colors.primary
    val nowPlaying = stringResource(R.string.now_playing)
    val origin = ui.origin
    val shuffled = if (ui.shuffle) stringResource(R.string.shuffled) else null
    val rest = dotted(origin?.let { stringResource(R.string.now_playing_from, it) }, shuffled)
    val playingFrom = stringResource(R.string.playing_from)
    Text(
        buildAnnotatedString {
            withStyle(SpanStyle(color = accent, fontWeight = FontWeight.Bold)) { append(nowPlaying) }
            if (rest.isNotEmpty()) append(" · $rest")
        },
        style = text(12, FontWeight.SemiBold),
        color = colors.onSurfaceVariant,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier = if (origin != null) {
            // A touch area taller than the line.
            Modifier
                .margins(top = 16.dp, bottom = 16.dp)
                .tappable(onOrigin)
                .clearAndSetSemantics {
                    contentDescription = "$playingFrom ${dotted(origin, shuffled)}"
                    role = Role.Button
                }
                .padding(vertical = 16.dp)
        } else {
            Modifier
        },
    )
}

/** What plays next, as many rows as there is room for; or that the queue ends here. */
@Composable
private fun UpNextList(rows: List<QueueRow>, onPlay: (QueueRow) -> Unit, modifier: Modifier) {
    if (rows.isEmpty()) {
        Text(
            stringResource(R.string.queue_end),
            style = text(13),
            color = colors.onSurfaceVariant,
            modifier = modifier.padding(top = 12.dp),
        )
        return
    }
    FittingColumn(modifier) {
        // No more than a tall screen has room for.
        for (row in rows.take(8)) UpNextItem(row, onPlay)
    }
}

/** A track that plays next: its cover, title and artist, and whether it was queued. */
@Composable
private fun UpNextItem(row: QueueRow, onPlay: (QueueRow) -> Unit) {
    val track = row.track
    Row(
        Modifier
            .fillMaxWidth()
            .height(UpNextRow)
            .clip(RoundedCornerShape(12.dp))
            .tappable({ onPlay(row) }),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Cover(
            track.cover,
            Modifier.size(38.dp),
            shape = RoundedCornerShape(8.dp),
            placeholder = if (track.loaded) CoverPlaceholder.ALBUM else CoverPlaceholder.UNREAD,
        )
        Column(Modifier.weight(1f)) {
            Text(
                track.title,
                style = text(14, FontWeight.SemiBold),
                color = colors.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                track.artist.ifEmpty { stringResource(R.string.unknown_artist) },
                style = text(12),
                color = colors.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        if (row.queued) {
            Text(stringResource(R.string.queue_queued), style = text(12, FontWeight.Bold), color = colors.primary)
        }
    }
}

/** Its children one under another, as many as fit whole; the rest are left out. */
@Composable
private fun FittingColumn(modifier: Modifier, content: @Composable () -> Unit) {
    Layout(content, modifier) { measurables, constraints ->
        val loose = constraints.copy(minHeight = 0)
        val placed = mutableListOf<Placeable>()
        var height = 0
        for (measurable in measurables) {
            val placeable = measurable.measure(loose)
            if (height + placeable.height > constraints.maxHeight) break
            placed += placeable
            height += placeable.height
        }
        layout(constraints.maxWidth, height.coerceAtLeast(constraints.minHeight)) {
            var y = 0
            for (placeable in placed) {
                placeable.place(0, y)
                y += placeable.height
            }
        }
    }
}
