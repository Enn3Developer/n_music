package com.enn3developer.n_music.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.pluralStringResource
import com.enn3developer.n_music.R
import java.text.NumberFormat

/** `seconds` as a track's length: `3:02`, `1:02:44`; empty while unknown. */
fun formatLength(seconds: Double): String {
    if (seconds.isNaN() || seconds <= 0) return ""
    return formatPosition(seconds)
}

/** `seconds` as a position: `0:00`, `3:02`, `1:02:44`. */
fun formatPosition(seconds: Double): String {
    val whole = if (seconds.isNaN() || seconds < 0) 0L else seconds.toLong()
    val hours = whole / 3600
    val minutes = whole / 60 % 60
    val rest = whole % 60
    return if (hours > 0) {
        "%d:%02d:%02d".format(hours, minutes, rest)
    } else {
        "%d:%02d".format(minutes, rest)
    }
}

/** A count with the locale's grouping, `4,736`. */
fun formatCount(count: Long): String = NumberFormat.getIntegerInstance().format(count)

fun formatCount(count: Int): String = formatCount(count.toLong())

fun formatCount(count: UInt): String = formatCount(count.toLong())

/** `4,736 tracks`. */
@Composable
fun tracksCount(count: Long): String =
    pluralStringResource(R.plurals.tracks_count, quantity(count), formatCount(count))

@Composable
fun tracksCount(count: UInt): String = tracksCount(count.toLong())

/** The quantity a plural picks for [count]. */
fun quantity(count: Long): Int = count.coerceIn(0, Int.MAX_VALUE.toLong()).toInt()

fun quantity(count: Int): Int = count.coerceAtLeast(0)

/** [parts] that are there, joined by a middle dot. */
fun dotted(vararg parts: String?): String = parts.filterNot { it.isNullOrEmpty() }.joinToString(" · ")
