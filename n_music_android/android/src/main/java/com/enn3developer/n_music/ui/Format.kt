package com.enn3developer.n_music.ui

import android.text.format.DateFormat
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import com.enn3developer.n_music.R
import java.text.NumberFormat
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit

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

/** How long a list plays: `52 min`, `3 h 52 min`, `8 days 2 h`. */
@Composable
fun formatDuration(seconds: Double): String {
    val minutes = (seconds / 60).toLong().coerceAtLeast(0)
    val hours = minutes / 60
    val days = hours / 24
    return when {
        days > 0 -> pluralStringResource(R.plurals.duration_days, quantity(days), days, hours % 24)
        hours > 0 && minutes % 60 == 0L -> stringResource(R.string.duration_hours_only, hours)
        hours > 0 -> stringResource(R.string.duration_hours, hours, minutes % 60)
        else -> stringResource(R.string.duration_minutes, minutes)
    }
}

/** `12 plays`. */
@Composable
fun playsCount(count: UInt): String =
    pluralStringResource(R.plurals.plays_count, quantity(count.toLong()), formatCount(count))

/**
 * When something happened, [unixSeconds] ago from now: today, yesterday, 3 days ago, then the
 * date, with the year once it is another year's.
 */
@Composable
fun formatWhen(unixSeconds: Long): String {
    val zone = ZoneId.systemDefault()
    val day = Instant.ofEpochSecond(unixSeconds).atZone(zone).toLocalDate()
    val today = LocalDate.now(zone)
    val days = ChronoUnit.DAYS.between(day, today)
    val locale = LocalConfiguration.current.locales[0]
    return when {
        days <= 0 -> stringResource(R.string.when_today)
        days == 1L -> stringResource(R.string.when_yesterday)
        days < 7 -> pluralStringResource(R.plurals.when_days_ago, days.toInt(), days)
        else -> {
            val skeleton = if (day.year == today.year) "MMMd" else "MMMdyyyy"
            DateTimeFormatter.ofPattern(DateFormat.getBestDateTimePattern(locale, skeleton), locale).format(day)
        }
    }
}

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
