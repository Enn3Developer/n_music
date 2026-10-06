package com.enn3developer.n_music.ui

import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.Current
import com.enn3developer.n_music.R
import com.enn3developer.n_music.Scan
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.key
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * The interim app: the library as one list and a player bar. It reads the core's state from
 * [CoreRepository] and sends it commands, nothing more.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NMusicApp(onPickFolder: () -> Unit) {
    val tracks by CoreRepository.tracks.collectAsStateWithLifecycle()
    val roots by CoreRepository.roots.collectAsStateWithLifecycle()
    val scan by CoreRepository.scan.collectAsStateWithLifecycle()
    val current by CoreRepository.current.collectAsStateWithLifecycle()
    val snackbar = remember { SnackbarHostState() }
    LaunchedEffect(Unit) {
        CoreRepository.notices.collect { snackbar.showSnackbar(it) }
    }

    Scaffold(
        topBar = {
            Column {
                TopAppBar(
                    title = { Text(stringResource(R.string.app_name)) },
                    actions = {
                        if (roots?.isNotEmpty() == true) {
                            IconButton(onClick = onPickFolder) {
                                Icon(
                                    painterResource(R.drawable.ic_folder),
                                    stringResource(R.string.change_folder),
                                )
                            }
                        }
                    },
                )
                scan?.let { ScanBar(it) }
            }
        },
        bottomBar = { current?.let { PlayerBar(it) } },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        val modifier = Modifier
            .fillMaxSize()
            .padding(padding)
        val list = tracks
        when {
            roots == null -> Loading(modifier)
            roots?.isEmpty() == true -> Message(
                stringResource(R.string.empty_library_title),
                stringResource(R.string.empty_library_body),
                onPickFolder,
                modifier,
            )

            list == null || (list.isEmpty() && scan != null) -> Loading(modifier)
            list.isEmpty() -> Message(
                stringResource(R.string.empty_folder_title),
                stringResource(R.string.empty_folder_body),
                onPickFolder,
                modifier,
            )

            else -> TrackList(list, current, modifier)
        }
    }
}

@Composable
private fun ScanBar(scan: Scan) {
    val read = (scan.found - scan.pending).toFloat()
    if (scan.found == 0uL) {
        LinearProgressIndicator(Modifier.fillMaxWidth())
    } else {
        LinearProgressIndicator(
            progress = { read / scan.found.toFloat() },
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
private fun Loading(modifier: Modifier) {
    Box(modifier, contentAlignment = Alignment.Center) {
        CircularProgressIndicator()
    }
}

@Composable
private fun Message(title: String, body: String, onPickFolder: () -> Unit, modifier: Modifier) {
    Column(
        modifier.padding(32.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Icon(
            painterResource(R.drawable.ic_folder),
            contentDescription = null,
            modifier = Modifier.size(48.dp),
            tint = MaterialTheme.colorScheme.primary,
        )
        Spacer(Modifier.height(16.dp))
        Text(title, style = MaterialTheme.typography.titleLarge, textAlign = TextAlign.Center)
        Spacer(Modifier.height(8.dp))
        Text(
            body,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
        Spacer(Modifier.height(24.dp))
        Button(onClick = onPickFolder) {
            Text(stringResource(R.string.pick_folder))
        }
    }
}

@Composable
private fun TrackList(tracks: List<TrackRow>, current: Current?, modifier: Modifier) {
    val playing = current?.track?.locator
    LazyColumn(modifier) {
        items(tracks, key = { it.locator.key }) { track ->
            val isCurrent = track.locator == playing
            ListItem(
                headlineContent = {
                    Text(track.title, maxLines = 1, overflow = TextOverflow.Ellipsis)
                },
                supportingContent = subtitle(track)?.let {
                    { Text(it, maxLines = 1, overflow = TextOverflow.Ellipsis) }
                },
                trailingContent = duration(track.length)?.let { { Text(it) } },
                colors = if (isCurrent) {
                    ListItemDefaults.colors(
                        containerColor = MaterialTheme.colorScheme.secondaryContainer,
                        headlineColor = MaterialTheme.colorScheme.onSecondaryContainer,
                    )
                } else {
                    ListItemDefaults.colors()
                },
                modifier = Modifier.clickable {
                    // Plays the whole library from this track, as the Slint app did.
                    CoreRepository.send(Command.PlayFrom(CoreRepository.library, track.locator))
                },
            )
        }
    }
}

@Composable
private fun PlayerBar(current: Current) {
    val playing by CoreRepository.playing.collectAsStateWithLifecycle()
    val track = current.track
    Surface(tonalElevation = 3.dp) {
        Column(
            Modifier
                .navigationBarsPadding()
                .padding(horizontal = 16.dp, vertical = 8.dp)
        ) {
            SeekBar()
            Row(verticalAlignment = Alignment.CenterVertically) {
                Cover(track.cover, Modifier.size(48.dp))
                Column(
                    Modifier
                        .weight(1f)
                        .padding(horizontal = 12.dp)
                ) {
                    Text(
                        track.title,
                        style = MaterialTheme.typography.titleMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    subtitle(track)?.let {
                        Text(
                            it,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
                IconButton(onClick = { CoreRepository.send(Command.PlayPrevious) }) {
                    Icon(
                        painterResource(R.drawable.ic_skip_previous),
                        stringResource(R.string.previous),
                    )
                }
                FilledIconButton(onClick = { CoreRepository.send(Command.TogglePause) }) {
                    Icon(
                        painterResource(if (playing) R.drawable.ic_pause else R.drawable.ic_play),
                        stringResource(if (playing) R.string.pause else R.string.play),
                    )
                }
                IconButton(onClick = { CoreRepository.send(Command.PlayNext) }) {
                    Icon(painterResource(R.drawable.ic_skip_next), stringResource(R.string.next))
                }
            }
        }
    }
}

/** The position, which follows a drag and then holds it until the core applied the seek. */
@Composable
private fun SeekBar() {
    val position by CoreRepository.position.collectAsStateWithLifecycle()
    var dragged by remember { mutableStateOf<Float?>(null) }
    var pending by remember { mutableStateOf<Pair<ULong, Float>?>(null) }
    val waiting = pending?.takeIf { position.seek < it.first }
    val length = position.length.toFloat().coerceAtLeast(0f)
    val shown = (dragged ?: waiting?.second ?: position.position.toFloat())
        .coerceIn(0f, length)
    Column {
        Slider(
            value = shown,
            onValueChange = { dragged = it },
            onValueChangeFinished = {
                dragged?.let { pending = CoreRepository.seek(it.toDouble()) to it }
                dragged = null
            },
            valueRange = 0f..length.coerceAtLeast(1f),
            enabled = length > 0f,
        )
        Row(Modifier.fillMaxWidth()) {
            Text(duration(shown.toDouble()) ?: "0:00", style = MaterialTheme.typography.labelSmall)
            Spacer(Modifier.weight(1f))
            Text(duration(position.length) ?: "", style = MaterialTheme.typography.labelSmall)
        }
    }
}

@Composable
private fun Cover(path: String?, modifier: Modifier) {
    val bitmap by produceState<ImageBitmap?>(null, path) {
        value = path?.let {
            withContext(Dispatchers.IO) { BitmapFactory.decodeFile(it)?.asImageBitmap() }
        }
    }
    Box(
        modifier
            .clip(RoundedCornerShape(8.dp))
            .background(MaterialTheme.colorScheme.surfaceVariant),
        contentAlignment = Alignment.Center,
    ) {
        val image = bitmap
        if (image != null) {
            Image(
                image,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize(),
            )
        } else {
            Icon(
                painterResource(R.drawable.ic_music_note),
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.width(24.dp),
            )
        }
    }
}

private fun subtitle(track: TrackRow): String? =
    listOfNotNull(track.artist.ifEmpty { null }, track.album?.ifEmpty { null })
        .joinToString(" · ")
        .ifEmpty { null }

/** `m:ss`, or `h:mm:ss` past an hour; `null` while unknown. */
private fun duration(seconds: Double): String? {
    if (seconds <= 0.0 || seconds.isNaN()) return null
    val total = seconds.toLong()
    val hours = total / 3600
    val minutes = total % 3600 / 60
    val rest = total % 60
    return if (hours > 0) {
        "%d:%02d:%02d".format(hours, minutes, rest)
    } else {
        "%d:%02d".format(minutes, rest)
    }
}
