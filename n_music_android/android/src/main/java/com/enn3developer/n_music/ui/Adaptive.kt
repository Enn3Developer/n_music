package com.enn3developer.n_music.ui

import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/**
 * How the app lays itself out, after Android's window size classes. Height wins over width when
 * it is short: a phone held sideways is wide, but gets a foldable's rail and mini player bar.
 */
enum class WindowLayout {
    /** Under 600 dp wide: the bottom bar and the mini player; the player opens full screen. */
    PHONE,

    /**
     * 600 to 840 dp wide: a rail with a drawer over the pages, and the mini player bar; the open
     * player shows the queue beside it.
     */
    FOLD,

    /** Wide but under 480 dp tall: a foldable's rail and bar; the player's cover sits beside it. */
    LANDSCAPE,

    /**
     * 840 dp wide and up: a rail that widens into a drawer that stays, a now playing pane, and
     * sheets from the side.
     */
    TABLET;

    /** A rail beside the pages, instead of the bottom bar. */
    val rail: Boolean get() = this != PHONE
}

/** The layout of a window [width] wide and [height] tall. */
fun windowLayout(width: Dp, height: Dp): WindowLayout = when {
    width < 600.dp -> WindowLayout.PHONE
    height < 480.dp -> WindowLayout.LANDSCAPE
    width < 840.dp -> WindowLayout.FOLD
    else -> WindowLayout.TABLET
}

val LocalWindowLayout = staticCompositionLocalOf { WindowLayout.PHONE }

/** The room a page leaves between its content and its sides. */
@Immutable
data class PageMargins(val start: Dp, val end: Dp) {
    /** The start margin less [inset], the room a control's own padding already keeps. */
    fun startLess(inset: Dp): Dp = (start - inset).coerceAtLeast(0.dp)

    /** The end margin less [inset], the room a control's own padding already keeps. */
    fun endLess(inset: Dp): Dp = (end - inset).coerceAtLeast(0.dp)
}

/**
 * The margins of the page showing: 16 dp on a phone, 24 dp beside a rail or a drawer, and none
 * at the start beside a tablet's rail, whose own width keeps the room.
 */
val LocalPageMargins = compositionLocalOf { PageMargins(16.dp, 16.dp) }

/** [content] laid out for the space it fills, the whole window for the app. */
@Composable
fun WindowLayoutBox(content: @Composable () -> Unit) {
    BoxWithConstraints(Modifier.fillMaxSize()) {
        CompositionLocalProvider(LocalWindowLayout provides windowLayout(maxWidth, maxHeight)) { content() }
    }
}
