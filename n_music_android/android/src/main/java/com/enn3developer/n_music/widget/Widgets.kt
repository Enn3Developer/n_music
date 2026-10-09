package com.enn3developer.n_music.widget

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.BroadcastReceiver
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.BitmapShader
import android.graphics.Canvas
import android.graphics.Matrix
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Shader
import android.os.Build
import android.os.Bundle
import android.view.KeyEvent
import android.view.View
import android.widget.RemoteViews
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.toArgb
import androidx.core.content.ContextCompat
import androidx.core.graphics.createBitmap
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.MainActivity
import com.enn3developer.n_music.PlaybackService
import com.enn3developer.n_music.PlayingFrom
import com.enn3developer.n_music.R
import com.enn3developer.n_music.Theme
import com.enn3developer.n_music.UiPreferences
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.TrackRow
import com.enn3developer.n_music.ui.Origin
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.player.originName
import com.enn3developer.n_music.ui.theme.Accent
import com.enn3developer.n_music.ui.theme.NColors
import com.enn3developer.n_music.ui.theme.colorsFor
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import kotlin.math.roundToInt

/** What the widgets show: what plays, where from, and the library's size, in the app's colours. */
internal data class WidgetState(
    val track: TrackRow?,
    val playing: Boolean,
    val shuffle: Boolean,
    /** What "Playing from" names; `null` when it is not known. */
    val origin: String?,
    val tracks: UInt,
    val theme: Theme,
    val accent: Accent,
)

/**
 * The home screen widgets: the player and Shuffle everything. While the process runs they follow
 * what plays. Their buttons start [PlaybackService] in the foreground, as the media notification's
 * Play does, so music starts without opening the app.
 */
object Widgets {
    /** Marks a start of [PlaybackService] that a widget asked for. */
    const val EXTRA_WIDGET = "com.enn3developer.n_music.extra.WIDGET"

    /** With [EXTRA_WIDGET]: the whole library, shuffled, before the key's Play. */
    const val EXTRA_SHUFFLE_ALL = "com.enn3developer.n_music.extra.SHUFFLE_ALL"

    private const val READY_TIMEOUT_MS = 2_000L

    /** Shuffle everything's pending intent, apart from the media keys' own codes. */
    private const val SHUFFLE_ALL_CODE = 1

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val drawing = Mutex()

    /** Keeps the widgets following what plays, for as long as the process runs. */
    fun start(context: Context) {
        val app = context.applicationContext
        scope.launch { state(app).collectLatest { render(app, it) } }
    }

    /** Draws the widgets again as they are, like after a change of theme or orientation. */
    fun refresh(context: Context) {
        val app = context.applicationContext
        scope.launch { render(app, state(app).first()) }
    }

    /** Draws the widgets for an update Android asked [receiver] for, keeping the process until then. */
    fun update(receiver: BroadcastReceiver, context: Context) {
        val app = context.applicationContext
        val pending = receiver.goAsync()
        scope.launch {
            try {
                // A process started for the update has an empty library until the core reads it.
                withTimeoutOrNull(READY_TIMEOUT_MS) { CoreRepository.ready.first { it } }
                render(app, state(app).first())
            } finally {
                pending.finish()
            }
        }
    }

    /** Plays the whole library shuffled, from the library: what Shuffle everything asks for. */
    fun shuffleEverything() {
        PlayingFrom.set(Origin.Library)
        CoreRepository.send(Command.SetShuffle(true))
        CoreRepository.send(Command.PlayFrom(CoreRepository.everything, null))
    }

    private fun state(context: Context): Flow<WidgetState> {
        val playback = combine(CoreRepository.current, CoreRepository.playing, CoreRepository.shuffle) { current, playing, shuffle ->
            Triple(current?.track, playing, shuffle)
        }
        val origin = combine(PlayingFrom.origin, CoreRepository.playlists, CoreRepository.sources) { origin, playlists, sources ->
            originName(origin, context.resources, playlists, sources)
        }
        return combine(playback, origin, CoreRepository.library, UiPreferences.settings) { (track, playing, shuffle), name, library, ui ->
            WidgetState(track, playing, shuffle, name, library.tracks, ui.theme, ui.accent)
        }.distinctUntilChanged()
    }

    private suspend fun render(context: Context, state: WidgetState) = drawing.withLock {
        withContext(Dispatchers.IO) {
            val manager = AppWidgetManager.getInstance(context) ?: return@withContext
            val players = manager.getAppWidgetIds(ComponentName(context, PlayerWidget::class.java))
            val shuffles = manager.getAppWidgetIds(ComponentName(context, ShuffleWidget::class.java))
            for (id in players) manager.updateAppWidget(id, playerViews(context, state, manager.getAppWidgetOptions(id)))
            if (shuffles.isNotEmpty()) manager.updateAppWidget(shuffles, shuffleViews(context, state))
        }
    }

    /** The player widget for [state], its cover sized for the widget's [options]. */
    internal fun playerViews(context: Context, state: WidgetState, options: Bundle): RemoteViews {
        val views = RemoteViews(context.packageName, R.layout.widget_player)
        val palette = Palette(context, state.theme, state.accent)
        views.color(context, R.id.widget_surface, "setColorFilter", palette) { it.widget }
        views.color(context, R.id.widget_origin, "setTextColor", palette) { it.primary }
        views.color(context, R.id.widget_title, "setTextColor", palette) { it.onSurface }
        views.color(context, R.id.widget_artist, "setTextColor", palette) { it.onSurfaceVariant }
        views.color(context, R.id.widget_previous, "setColorFilter", palette) { it.onSurface }
        views.color(context, R.id.widget_next, "setColorFilter", palette) { it.onSurface }
        views.color(context, R.id.widget_play_pill, "setColorFilter", palette) { it.primaryContainer }
        views.color(context, R.id.widget_play_icon, "setColorFilter", palette) { it.onPrimaryContainer }

        val track = state.track
        val (width, height) = playerSize(context, options)
        // A short widget gives up where it plays from, then the artist, so the buttons fit.
        val origin = state.origin.takeIf { track != null && height >= ROOM_FOR_ORIGIN }
        views.setTextViewText(
            R.id.widget_origin,
            dotted(origin, context.getString(R.string.shuffled).takeIf { state.shuffle }),
        )
        views.setViewVisibility(R.id.widget_origin, if (origin != null) View.VISIBLE else View.GONE)
        views.setViewVisibility(R.id.widget_artist, if (height >= ROOM_FOR_ARTIST) View.VISIBLE else View.GONE)
        views.setTextViewText(R.id.widget_title, track?.title ?: context.getString(R.string.app_name))
        views.setTextViewText(
            R.id.widget_artist,
            when {
                track == null -> context.getString(R.string.widget_nothing_playing)
                else -> track.artist.ifEmpty { context.getString(R.string.unknown_artist) }
            },
        )
        views.setImageViewResource(R.id.widget_play_icon, if (state.playing) R.drawable.ic_widget_pause else R.drawable.ic_widget_play)
        views.setContentDescription(R.id.widget_play, context.getString(if (state.playing) R.string.pause else R.string.play))

        val side = coverSide(width, height)
        if (side > 0) views.setImageViewBitmap(R.id.widget_cover, cover(context, track, side, palette.now(context)))
        views.setViewVisibility(R.id.widget_cover, if (side > 0) View.VISIBLE else View.GONE)

        val open = openApp(context)
        views.setOnClickPendingIntent(R.id.widget_body, open)
        if (track == null) {
            // Nothing to play yet: the buttons lead to the app, where the library is.
            for (button in listOf(R.id.widget_previous, R.id.widget_play, R.id.widget_next)) {
                views.setOnClickPendingIntent(button, open)
            }
        } else {
            views.setOnClickPendingIntent(R.id.widget_previous, mediaButton(context, KeyEvent.KEYCODE_MEDIA_PREVIOUS))
            views.setOnClickPendingIntent(
                R.id.widget_play,
                mediaButton(context, if (state.playing) KeyEvent.KEYCODE_MEDIA_PAUSE else KeyEvent.KEYCODE_MEDIA_PLAY),
            )
            views.setOnClickPendingIntent(R.id.widget_next, mediaButton(context, KeyEvent.KEYCODE_MEDIA_NEXT))
        }
        return views
    }

    /** Shuffle everything for [state]: with the library's size, or leading to the app while it's empty. */
    internal fun shuffleViews(context: Context, state: WidgetState): RemoteViews {
        val views = RemoteViews(context.packageName, R.layout.widget_shuffle)
        val palette = Palette(context, state.theme, state.accent)
        views.color(context, R.id.widget_surface, "setColorFilter", palette) { it.widget }
        views.color(context, R.id.widget_shuffle_pill, "setColorFilter", palette) { it.primaryContainer }
        views.color(context, R.id.widget_shuffle_icon, "setColorFilter", palette) { it.onPrimaryContainer }
        views.color(context, R.id.widget_shuffle_title, "setTextColor", palette) { it.onSurface }
        views.color(context, R.id.widget_shuffle_count, "setTextColor", palette) { it.onSurfaceVariant }
        val count = formatCount(state.tracks)
        views.setTextViewText(
            R.id.widget_shuffle_count,
            context.resources.getQuantityString(R.plurals.tracks_count, state.tracks.toInt(), count),
        )
        if (state.tracks > 0u) {
            views.setOnClickPendingIntent(
                android.R.id.background,
                mediaButton(context, KeyEvent.KEYCODE_MEDIA_PLAY, shuffleAll = true),
            )
            views.setContentDescription(android.R.id.background, context.getString(R.string.shuffle_all, count))
        } else {
            views.setOnClickPendingIntent(android.R.id.background, openApp(context))
        }
        return views
    }

    /**
     * Presses [key] on the playback service, starting it in the foreground as the media
     * notification's Play does; [shuffleAll] plays the whole library shuffled first.
     */
    private fun mediaButton(context: Context, key: Int, shuffleAll: Boolean = false): PendingIntent {
        val intent = Intent(Intent.ACTION_MEDIA_BUTTON)
            .setComponent(ComponentName(context, PlaybackService::class.java))
            .putExtra(Intent.EXTRA_KEY_EVENT, KeyEvent(KeyEvent.ACTION_DOWN, key))
            .putExtra(EXTRA_WIDGET, true)
            .putExtra(EXTRA_SHUFFLE_ALL, shuffleAll)
        // The intents differ only in their extras, so each press needs its own code.
        return PendingIntent.getForegroundService(
            context,
            if (shuffleAll) SHUFFLE_ALL_CODE else key,
            intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
    }

    private fun openApp(context: Context): PendingIntent = PendingIntent.getActivity(
        context,
        0,
        Intent(context, MainActivity::class.java)
            .setAction(Intent.ACTION_MAIN)
            .addCategory(Intent.CATEGORY_LAUNCHER),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )
}

/** The player widget's padding, the gap after its cover, and its buttons' width, in dp. */
private const val PADDING = 16
private const val COVER_GAP = 14
private const val BUTTONS = 44 + 4 + 64 + 4 + 44

/** The design's player widget, while the launcher hasn't told its size. */
private const val DESIGN_WIDTH = 379
private const val DESIGN_HEIGHT = 184

/** Below this the cover is left out, for the title's sake. */
private const val SMALLEST_COVER = 56

/** The heights from which the lines above the buttons have room for the artist, then for where from. */
private const val ROOM_FOR_ARTIST = 132
private const val ROOM_FOR_ORIGIN = 148

/** The player widget's width and height in dp, as the launcher tells them for the way the phone is held. */
private fun playerSize(context: Context, options: Bundle): Pair<Int, Int> {
    val landscape = context.resources.configuration.orientation == Configuration.ORIENTATION_LANDSCAPE
    // In portrait a widget is its smallest width and its largest height; in landscape the reverse.
    val width = options.getInt(
        if (landscape) AppWidgetManager.OPTION_APPWIDGET_MAX_WIDTH else AppWidgetManager.OPTION_APPWIDGET_MIN_WIDTH,
    )
    val height = options.getInt(
        if (landscape) AppWidgetManager.OPTION_APPWIDGET_MIN_HEIGHT else AppWidgetManager.OPTION_APPWIDGET_MAX_HEIGHT,
    )
    return if (width > 0 && height > 0) width to height else DESIGN_WIDTH to DESIGN_HEIGHT
}

/**
 * The cover's side in dp: the widget's height inside its padding, as long as the buttons still
 * fit beside it; 0 when they wouldn't.
 */
private fun coverSide(width: Int, height: Int): Int {
    val side = minOf(height - 2 * PADDING, width - 2 * PADDING - COVER_GAP - BUTTONS)
    return if (side >= SMALLEST_COVER) side else 0
}

/** [track]'s cover cropped to a square of [side] dp with the design's corners, or a disc where it has none. */
private fun cover(context: Context, track: TrackRow?, side: Int, colors: NColors): Bitmap {
    val density = context.resources.displayMetrics.density
    val size = (side * density).roundToInt()
    val radius = 20 * density
    val bounds = RectF(0f, 0f, size.toFloat(), size.toFloat())
    val bitmap = createBitmap(size, size)
    val canvas = Canvas(bitmap)
    val paint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG)
    val source = track?.cover?.let { BitmapFactory.decodeFile(it) }
    if (source != null) {
        val scale = size / minOf(source.width, source.height).toFloat()
        paint.shader = BitmapShader(source, Shader.TileMode.CLAMP, Shader.TileMode.CLAMP).apply {
            setLocalMatrix(
                Matrix().apply {
                    setScale(scale, scale)
                    postTranslate((size - source.width * scale) / 2, (size - source.height * scale) / 2)
                }
            )
        }
        canvas.drawRoundRect(bounds, radius, radius, paint)
    } else {
        paint.color = colors.surfaceHighest.toArgb()
        canvas.drawRoundRect(bounds, radius, radius, paint)
        val disc = ContextCompat.getDrawable(context, R.drawable.ic_widget_album)?.mutate()
        if (disc != null) {
            val icon = (size * 0.42f).roundToInt()
            val start = (size - icon) / 2
            disc.setBounds(start, start, start + icon, start + icon)
            disc.setTint(colors.onSurfaceQuiet.toArgb())
            disc.draw(canvas)
        }
    }
    return bitmap
}

/** The widgets' colours in the light and the dark theme; the same twice while the app's theme is fixed. */
private class Palette(context: Context, theme: Theme, accent: Accent) {
    val light: NColors = colorsFor(context, accent, dark = theme == Theme.DARK)
    val dark: NColors = if (theme == Theme.LIGHT) light else colorsFor(context, accent, dark = true)

    /** The colours as the screen is now, for what a launcher can't switch by itself, like a cover. */
    fun now(context: Context): NColors = if (context.isNight) dark else light
}

private val Context.isNight: Boolean
    get() = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES

/** A widget's card: the design's, a step lighter than the app's surface in the dark. */
private val NColors.widget: Color
    get() = if (dark) lerp(surface, surfaceHigh, 0.67f) else surfaceLowest

/**
 * Calls [method] on [id] with [role]'s colour: from Android 12 the launcher picks the light or the
 * dark one as the theme switches, before that it's the one for the theme now.
 */
private fun RemoteViews.color(context: Context, id: Int, method: String, palette: Palette, role: (NColors) -> Color) {
    val light = role(palette.light).toArgb()
    val dark = role(palette.dark).toArgb()
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        setColorInt(id, method, light, dark)
    } else {
        setInt(id, method, if (context.isNight) dark else light)
    }
}

/** The player widget: Android asks it to draw when it's placed and when it's resized. */
class PlayerWidget : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) = Widgets.update(this, context)

    override fun onAppWidgetOptionsChanged(context: Context, manager: AppWidgetManager, id: Int, options: Bundle) =
        Widgets.update(this, context)
}

/** Shuffle everything: Android asks it to draw when it's placed. */
class ShuffleWidget : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) = Widgets.update(this, context)
}
