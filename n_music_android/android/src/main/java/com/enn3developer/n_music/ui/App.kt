package com.enn3developer.n_music.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.LoopStatus
import com.enn3developer.n_music.core.TrackRow
import kotlinx.coroutines.launch

/** The height of a track row, which scrolling to the playing track counts on. */
private val ROW_HEIGHT = 84.dp

/**
 * The Slint app's main screen: search, the library in play order, and the control panel. It
 * reads the core's state from [CoreRepository] and sends it commands, nothing more.
 */
@Composable
fun AppScreen(onSettings: () -> Unit) {
    val rows by CoreRepository.rows.collectAsStateWithLifecycle()
    val current by CoreRepository.current.collectAsStateWithLifecycle()
    var search by rememberSaveable { mutableStateOf("") }
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()
    val density = LocalDensity.current

    // Searching keeps where the list was, and clearing the search goes back there.
    var saved by rememberSaveable { mutableStateOf<Pair<Int, Int>?>(null) }
    val query = search.lowercase()
    val shown = remember(rows, query) {
        if (query.isEmpty()) {
            rows
        } else {
            rows.filter {
                it.title.lowercase().contains(query) || it.artist.lowercase().contains(query)
            }
        }
    }
    LaunchedEffect(query.isEmpty()) {
        if (query.isEmpty()) {
            saved?.let { (index, offset) -> listState.scrollToItem(index, offset) }
            saved = null
        }
    }
    // Before anything plays, the first track is the one shown, as in the Slint app.
    val playing = (current?.track ?: rows.firstOrNull())?.locator

    // The keyboard covers the bottom of the screen: the list's end scrolls above it.
    var belowList by remember { mutableIntStateOf(0) }
    val keyboard = WindowInsets.ime.getBottom(density)
    val aboveKeyboard = with(density) { (keyboard - belowList).coerceAtLeast(0).toDp() }

    Surface(color = MaterialTheme.colorScheme.background) {
        Column(
            Modifier
                .fillMaxSize()
                .windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Horizontal))
        ) {
            TopPanel(
                search = search,
                onSearch = {
                    if (search.isEmpty() && it.isNotEmpty()) {
                        saved = listState.firstVisibleItemIndex to listState.firstVisibleItemScrollOffset
                    }
                    search = it
                    // The results start at the top.
                    if (it.isNotEmpty()) scope.launch { listState.scrollToItem(0) }
                },
                onShowPlaying = {
                    val row = shown.indexOfFirst { it.locator == playing }
                    if (row >= 0) {
                        // The row before the playing one peeks in at the top, as in the Slint app.
                        val offset = if (row > 0) with(density) { 50.dp.roundToPx() } else 0
                        scope.launch { listState.scrollToItem((row - 1).coerceAtLeast(0), offset) }
                    }
                },
                onSettings = onSettings,
            )
            Box(Modifier.weight(1f)) {
                LazyColumn(
                    state = listState,
                    contentPadding = PaddingValues(end = 12.dp, bottom = aboveKeyboard),
                    modifier = Modifier.fillMaxSize(),
                ) {
                    // By position, not by track: when the play order changes, the list stays
                    // where it was, as the Slint app's did, rather than following a track.
                    items(shown) { track ->
                        TrackItem(track, playing = track.locator == playing) {
                            // Plays the whole library from this track.
                            CoreRepository.send(Command.PlayFrom(CoreRepository.library, track.locator))
                        }
                    }
                }
                ListScrollbar(
                    state = listState,
                    itemCount = shown.size,
                    itemHeight = ROW_HEIGHT,
                    modifier = Modifier
                        .align(Alignment.CenterEnd)
                        .fillMaxHeight(),
                )
            }
            // Before anything plays, the panel shows the first track, as in the Slint app.
            ControlPanel(
                current?.track ?: rows.firstOrNull(),
                Modifier.onSizeChanged { belowList = it.height },
            )
        }
    }
}

@Composable
private fun TopPanel(
    search: String,
    onSearch: (String) -> Unit,
    onShowPlaying: () -> Unit,
    onSettings: () -> Unit,
) {
    val strings = LocalStrings.current
    val progress by CoreRepository.scanProgress.collectAsStateWithLifecycle()
    Column(
        Modifier
            .windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Top))
            .padding(10.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(
            Modifier.height(56.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            SearchBar(
                text = search,
                onTextChange = onSearch,
                placeholder = strings.search,
                dismissLabel = stringResource(R.string.dismiss_search),
                clearLabel = stringResource(R.string.clear_search),
                modifier = Modifier.weight(1f),
            )
            IconButton(onClick = onShowPlaying) {
                Icon(
                    painterResource(R.drawable.ic_down),
                    stringResource(R.string.show_playing_track),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            FilledIconButton(onClick = onSettings) {
                Icon(painterResource(R.drawable.ic_settings), strings.settings)
            }
        }
        // A finished scan leaves the bar empty.
        LinearProgressIndicator(
            progress = { if (progress >= 1f) 0f else progress },
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
private fun TrackItem(track: TrackRow, playing: Boolean, onClick: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    val title = if (playing) colors.onSecondaryContainer else colors.onSurface
    val details = if (playing) colors.onSecondaryContainer else colors.onSurfaceVariant
    Row(
        Modifier
            .fillMaxWidth()
            .height(ROW_HEIGHT)
            .clip(RoundedCornerShape(12.dp))
            .background(if (playing) colors.secondaryContainer else colors.surface)
            .clickable(onClick = onClick)
            .padding(start = if (playing) 24.dp else 12.dp, end = 12.dp, top = 10.dp, bottom = 10.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Cover(track.cover, 64.dp)
        Column(Modifier.weight(1f)) {
            Text(
                track.title,
                style = MaterialTheme.typography.titleMedium,
                color = title,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                track.artist,
                style = MaterialTheme.typography.bodyMedium,
                color = details,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        Text(formatTime(track.length), style = MaterialTheme.typography.labelMedium, color = details)
    }
}

@Composable
private fun ControlPanel(track: TrackRow?, modifier: Modifier = Modifier) {
    val colors = MaterialTheme.colorScheme
    val playing by CoreRepository.playing.collectAsStateWithLifecycle()
    val loopStatus by CoreRepository.loopStatus.collectAsStateWithLifecycle()
    val repeatOne = loopStatus == LoopStatus.FILE
    // What the slider shows; Previous puts it back to the start straight away.
    var time by remember { mutableFloatStateOf(0f) }
    Surface(
        color = colors.surfaceContainer,
        shape = RoundedCornerShape(topStart = 24.dp, topEnd = 24.dp),
        modifier = modifier,
    ) {
        Column(
            Modifier
                .navigationBarsPadding()
                .padding(start = 8.dp, end = 8.dp, top = 8.dp, bottom = 16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            ArtistAndTitle(track?.artist ?: "", track?.title ?: "", Modifier.fillMaxWidth())
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Cover(track?.cover, 72.dp)
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    PositionRow(track, time = time, onTime = { time = it })
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
                    ) {
                        MediaButton(
                            icon = R.drawable.ic_previous,
                            label = stringResource(R.string.previous_track),
                            background = colors.surfaceContainerHighest,
                            iconColor = colors.onSurface,
                            onClick = {
                                CoreRepository.send(Command.PlayPrevious)
                                time = 0f
                            },
                        )
                        MediaButton(
                            icon = if (playing) R.drawable.ic_pause else R.drawable.ic_play,
                            label = stringResource(if (playing) R.string.pause else R.string.play),
                            background = colors.primaryContainer,
                            iconColor = colors.onPrimaryContainer,
                            onClick = { CoreRepository.send(Command.TogglePause) },
                        )
                        MediaButton(
                            icon = R.drawable.ic_next,
                            label = stringResource(R.string.next_track),
                            background = colors.surfaceContainerHighest,
                            iconColor = colors.onSurface,
                            onClick = { CoreRepository.send(Command.PlayNext) },
                        )
                        MediaButton(
                            icon = if (repeatOne) R.drawable.ic_repeat_on else R.drawable.ic_repeat_off,
                            label = stringResource(if (repeatOne) R.string.repeat_one else R.string.repeat_all),
                            background = if (repeatOne) colors.primaryContainer else colors.surfaceContainerHighest,
                            iconColor = if (repeatOne) colors.onPrimaryContainer else colors.onSurface,
                            onClick = { CoreRepository.send(Command.ToggleRepeat) },
                            checked = repeatOne,
                        )
                    }
                }
            }
        }
    }
}

/**
 * The Slint app's slider track: thin, with the played part ending a little before the thumb,
 * rounded where the track starts.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SliderTrack(state: SliderState) {
    val colors = MaterialTheme.colorScheme
    Canvas(
        Modifier
            .fillMaxWidth()
            .height(4.dp)
    ) {
        val range = state.valueRange.endInclusive - state.valueRange.start
        val fraction = if (range > 0f) {
            ((state.value - state.valueRange.start) / range).coerceIn(0f, 1f)
        } else {
            0f
        }
        val round = CornerRadius(size.height / 2)
        drawRoundRect(colors.surfaceContainerHighest, cornerRadius = round)
        val played = size.width * fraction - 6.dp.toPx()
        if (played > 0f) {
            val square = CornerRadius(1.dp.toPx())
            val path = Path().apply {
                addRoundRect(
                    RoundRect(
                        rect = Rect(0f, 0f, played, size.height),
                        topLeft = round,
                        topRight = square,
                        bottomRight = square,
                        bottomLeft = round,
                    )
                )
            }
            drawPath(path, colors.primary)
        }
    }
}

/**
 * The position, the slider and the track's length. While a drag is on, and until the core
 * applied the seek it ends with, positions do not move the slider.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PositionRow(track: TrackRow?, time: Float, onTime: (Float) -> Unit) {
    val colors = MaterialTheme.colorScheme
    val position by CoreRepository.position.collectAsStateWithLifecycle()
    var dragged by remember { mutableStateOf<Float?>(null) }
    var request by remember { mutableStateOf(0uL) }
    LaunchedEffect(position) {
        if (dragged == null && position.seek >= request) onTime(position.position.toFloat())
    }
    val maximum = if (position.length > 1.0) position.length.toFloat() else 1f
    val label = stringResource(R.string.playback_position)
    Row(
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            formatTime(position.position),
            style = MaterialTheme.typography.labelMedium,
            color = colors.onSurfaceVariant,
        )
        Slider(
            value = (dragged ?: time).coerceIn(0f, maximum),
            onValueChange = { dragged = it },
            onValueChangeFinished = {
                dragged?.let {
                    request = CoreRepository.seek(it.toDouble())
                    onTime(it)
                }
                dragged = null
            },
            valueRange = 0f..maximum,
            thumb = {
                Box(
                    Modifier
                        .size(width = 4.dp, height = 32.dp)
                        .background(colors.primary, RoundedCornerShape(2.dp))
                )
            },
            track = { state -> SliderTrack(state) },
            modifier = Modifier
                .weight(1f)
                .semantics { contentDescription = label },
        )
        Text(
            formatTime(track?.length ?: 0.0),
            style = MaterialTheme.typography.labelMedium,
            color = colors.onSurfaceVariant,
        )
    }
}
