package com.enn3developer.n_music.ui

import android.graphics.BitmapFactory
import android.util.LruCache
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.gestures.stopScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.R
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlin.math.max
import kotlin.math.min

/** `seconds` as `mm:ss`, like the Slint app. */
fun formatTime(seconds: Double): String {
    val whole = if (seconds.isNaN() || seconds < 0) 0L else seconds.toLong()
    return "%02d:%02d".format(whole / 60, whole % 60)
}

/** A 48dp square button with rounded corners, as the Slint control panel drew them. */
@Composable
fun MediaButton(
    icon: Int,
    label: String,
    background: Color,
    iconColor: Color,
    onClick: () -> Unit,
    checked: Boolean? = null,
) {
    val shape = RoundedCornerShape(48.dp / 5)
    val action = if (checked == null) {
        Modifier.clickable(role = Role.Button, onClick = onClick)
    } else {
        Modifier.toggleable(value = checked, role = Role.Checkbox, onValueChange = { onClick() })
    }
    Box(
        Modifier
            .size(48.dp)
            .clip(shape)
            .background(background)
            .then(action)
            .semantics { contentDescription = label },
        contentAlignment = Alignment.Center,
    ) {
        Icon(painterResource(icon), contentDescription = null, tint = iconColor, modifier = Modifier.size(24.dp))
    }
}

/**
 * The Slint app's search bar: a pill whose search icon turns into a back arrow while it is in
 * use, with a button to clear what was typed.
 */
@Composable
fun SearchBar(
    text: String,
    onTextChange: (String) -> Unit,
    placeholder: String,
    dismissLabel: String,
    clearLabel: String,
    modifier: Modifier = Modifier,
) {
    val focus = remember { FocusRequester() }
    val focusManager = LocalFocusManager.current
    var focused by remember { mutableStateOf(false) }
    val active = focused || text.isNotEmpty()
    val colors = MaterialTheme.colorScheme
    Surface(
        modifier = modifier.heightIn(min = 56.dp),
        shape = RoundedCornerShape(28.dp),
        color = colors.surfaceContainerHigh,
    ) {
        Row(
            Modifier.padding(4.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = { if (active) focusManager.clearFocus() else focus.requestFocus() }) {
                Icon(
                    painterResource(if (active) R.drawable.ic_arrow_back else R.drawable.ic_search),
                    contentDescription = if (active) dismissLabel else placeholder,
                    tint = colors.onSurfaceVariant,
                )
            }
            Box(Modifier.weight(1f), contentAlignment = Alignment.CenterStart) {
                if (text.isEmpty()) {
                    Text(
                        placeholder,
                        style = MaterialTheme.typography.bodyLarge,
                        color = colors.onSurfaceVariant,
                        maxLines = 1,
                    )
                }
                BasicTextField(
                    value = text,
                    onValueChange = onTextChange,
                    singleLine = true,
                    textStyle = MaterialTheme.typography.bodyLarge.copy(color = colors.onSurface),
                    cursorBrush = SolidColor(colors.primary),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                    keyboardActions = KeyboardActions(onSearch = { focusManager.clearFocus() }),
                    modifier = Modifier
                        .fillMaxWidth()
                        .focusRequester(focus)
                        .onFocusChanged { focused = it.isFocused }
                        .semantics { contentDescription = placeholder },
                )
            }
            if (active && text.isNotEmpty()) {
                IconButton(onClick = {
                    onTextChange("")
                    focusManager.clearFocus()
                }) {
                    Icon(
                        painterResource(R.drawable.ic_close),
                        contentDescription = clearLabel,
                        tint = colors.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

/** Decoded covers by path, so the tracks of an album share one bitmap. */
private object Covers {
    private val cache = object : LruCache<String, ImageBitmap>(
        (Runtime.getRuntime().maxMemory() / 16).toInt()
    ) {
        override fun sizeOf(key: String, value: ImageBitmap) = value.width * value.height * 4
    }

    fun cached(path: String): ImageBitmap? = cache.get(path)

    suspend fun load(path: String, size: Int): ImageBitmap? = withContext(Dispatchers.IO) {
        cache.get(path)?.let { return@withContext it }
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(path, bounds)
        var sample = 1
        while (bounds.outWidth / (sample * 2) >= size && bounds.outHeight / (sample * 2) >= size) {
            sample *= 2
        }
        val bitmap = BitmapFactory.decodeFile(path, BitmapFactory.Options().apply { inSampleSize = sample })
            ?: return@withContext null
        bitmap.asImageBitmap().also { cache.put(path, it) }
    }
}

/** A track's cover, cropped to a rounded square; nothing while there is none. */
@Composable
fun Cover(path: String?, size: Dp, modifier: Modifier = Modifier) {
    val pixels = with(LocalDensity.current) { size.roundToPx() }
    // The state outlives a change of path: the old cover goes as soon as the path changes.
    val bitmap by produceState(path?.let(Covers::cached), path) {
        value = path?.let(Covers::cached)
        if (path != null && value == null) value = Covers.load(path, pixels)
    }
    Box(
        modifier
            .size(size)
            .clip(RoundedCornerShape(8.dp))
    ) {
        bitmap?.let {
            Image(
                it,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize(),
            )
        }
    }
}

/**
 * Lays out `artist • title` on one line as the Slint control panel did: each text takes what it
 * needs while the line has room, and a text that does not fit is cut short.
 */
@Composable
fun ArtistAndTitle(artist: String, title: String, modifier: Modifier = Modifier) {
    val colors = MaterialTheme.colorScheme
    val spacing = 8.dp
    Layout(
        modifier = modifier,
        content = {
            Text(
                artist,
                style = MaterialTheme.typography.bodyMedium,
                color = colors.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text("•", style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceVariant)
            Text(
                title,
                style = MaterialTheme.typography.titleMedium,
                color = colors.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        },
    ) { measurables, constraints ->
        val gap = spacing.roundToPx()
        val (artistText, bullet, titleText) = measurables
        val bulletPlaceable = bullet.measure(Constraints())
        val room = max(0, constraints.maxWidth - bulletPlaceable.width - 2 * gap)
        val artistWidth = artistText.maxIntrinsicWidth(constraints.maxHeight)
        val titleWidth = titleText.maxIntrinsicWidth(constraints.maxHeight)
        val half = room / 2
        val (artistRoom, titleRoom) = when {
            artistWidth + titleWidth <= room -> artistWidth to titleWidth
            artistWidth <= half -> artistWidth to room - artistWidth
            titleWidth <= half -> room - titleWidth to titleWidth
            else -> half to room - half
        }
        val artistPlaceable = artistText.measure(Constraints(maxWidth = artistRoom))
        val titlePlaceable = titleText.measure(Constraints(maxWidth = titleRoom))
        val height = maxOf(artistPlaceable.height, bulletPlaceable.height, titlePlaceable.height)
        layout(constraints.maxWidth, height) {
            var x = 0
            artistPlaceable.placeRelative(x, (height - artistPlaceable.height) / 2)
            x += artistPlaceable.width + gap
            bulletPlaceable.placeRelative(x, (height - bulletPlaceable.height) / 2)
            x += bulletPlaceable.width + gap
            titlePlaceable.placeRelative(x, (height - titlePlaceable.height) / 2)
        }
    }
}

/** A settings row: its name on the left, its control on the right. */
@Composable
fun Setting(text: String, content: @Composable () -> Unit) {
    Row(
        Modifier.heightIn(min = 56.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(1f),
        )
        Row(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            content()
        }
    }
}

/** The Slint app's combo box: a read-only field that opens the choices under it. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ComboBox(
    options: List<String>,
    selected: String,
    label: String,
    onSelect: (Int) -> Unit,
    maxVisibleItems: Int = 6,
) {
    var expanded by remember { mutableStateOf(false) }
    ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
        TextField(
            value = selected,
            onValueChange = {},
            readOnly = true,
            singleLine = true,
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier = Modifier
                .width(150.dp)
                .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable)
                .semantics { contentDescription = label },
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
            containerColor = MaterialTheme.colorScheme.surfaceContainer,
            modifier = Modifier.heightIn(max = 48.dp * maxVisibleItems + 16.dp),
        ) {
            options.forEachIndexed { index, option ->
                DropdownMenuItem(
                    text = { Text(option, style = MaterialTheme.typography.bodyLarge) },
                    onClick = {
                        expanded = false
                        onSelect(index)
                    },
                    modifier = if (option == selected) {
                        Modifier.background(MaterialTheme.colorScheme.surfaceContainerHighest)
                    } else {
                        Modifier
                    },
                )
            }
        }
    }
}

/**
 * The Slint app's scrollbar for a list of [itemHeight] rows: a thumb on a track that parts
 * around it, which can be dragged. As there, it takes touches 32dp in from the edge while the
 * list scrolls.
 */
@Composable
fun ListScrollbar(
    state: LazyListState,
    itemCount: Int,
    itemHeight: Dp,
    modifier: Modifier = Modifier,
) {
    val colors = MaterialTheme.colorScheme
    val count by rememberUpdatedState(itemCount)
    val scope = rememberCoroutineScope()
    val scrolls = state.canScrollForward || state.canScrollBackward
    val drag = if (scrolls) {
        Modifier.pointerInput(state) {
            detectVerticalDragGestures(
                // A fling would go on moving the list under the thumb.
                onDragStart = { scope.launch { state.stopScroll() } },
            ) { change, dragAmount ->
                change.consume()
                val metrics = ScrollMetrics(state, count, itemHeight.toPx(), size.height.toFloat(), this)
                if (metrics.maximum > 0f && metrics.travel > 0f) {
                    state.dispatchRawDelta(dragAmount * metrics.maximum / metrics.travel)
                }
            }
        }
    } else {
        Modifier
    }
    Canvas(
        modifier
            .width(32.dp)
            .then(drag)
    ) {
        val metrics = ScrollMetrics(state, itemCount, itemHeight.toPx(), size.height, this)
        if (metrics.maximum <= 0f) return@Canvas
        val barWidth = 8.dp.toPx()
        val x = size.width - barWidth - 4.dp.toPx()
        val radius = CornerRadius(barWidth / 2)
        val gap = 8.dp.toPx()
        val thumbTop = metrics.thumbTop
        val thumbBottom = thumbTop + metrics.thumb
        if (thumbTop - gap > 0f) {
            drawRoundRect(colors.secondaryContainer, Offset(x, 0f), Size(barWidth, thumbTop - gap), radius)
        }
        if (size.height - thumbBottom - gap > 0f) {
            drawRoundRect(
                colors.secondaryContainer,
                Offset(x, thumbBottom + gap),
                Size(barWidth, size.height - thumbBottom - gap),
                radius,
            )
        }
        drawRoundRect(colors.primary, Offset(x, thumbTop), Size(barWidth, metrics.thumb), radius)
    }
}

/** Where the scrollbar's thumb goes, from how far the list scrolled. */
private class ScrollMetrics(
    state: LazyListState,
    itemCount: Int,
    itemHeight: Float,
    height: Float,
    density: androidx.compose.ui.unit.Density,
) {
    private val offset = with(density) { 2.dp.toPx() }
    private val track = height - 2 * offset
    val maximum = itemCount * itemHeight - height
    val thumb = if (maximum <= 0f) {
        0f
    } else {
        max(min(with(density) { 32.dp.toPx() }, track), track * height / (maximum + height))
    }
    val travel = track - thumb
    private val scrolled = state.firstVisibleItemIndex * itemHeight + state.firstVisibleItemScrollOffset
    val thumbTop = offset + travel * (scrolled / max(1f, maximum)).coerceIn(0f, 1f)
}
