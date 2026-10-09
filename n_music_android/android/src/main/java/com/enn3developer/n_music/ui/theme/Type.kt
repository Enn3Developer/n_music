package com.enn3developer.n_music.ui.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.LineHeightStyle
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.sp
import com.enn3developer.n_music.R

val Figtree = FontFamily(
    Font(R.font.figtree_regular, FontWeight.Normal),
    Font(R.font.figtree_medium, FontWeight.Medium),
    Font(R.font.figtree_semibold, FontWeight.SemiBold),
    Font(R.font.figtree_bold, FontWeight.Bold),
    Font(R.font.figtree_extrabold, FontWeight.ExtraBold),
)

/** Line boxes as CSS draws them: the leading split evenly above and below the text. */
private val CenteredLines = LineHeightStyle(
    alignment = LineHeightStyle.Alignment.Center,
    trim = LineHeightStyle.Trim.None,
)

/**
 * A Figtree style of [size] sp in [weight]. [lineHeight] defaults to the font's own, as CSS's
 * `normal` does.
 */
fun text(
    size: Int,
    weight: FontWeight = FontWeight.Normal,
    lineHeight: TextUnit = TextUnit.Unspecified,
    letterSpacing: TextUnit = 0.sp,
    tabular: Boolean = false,
) = TextStyle(
    fontFamily = Figtree,
    fontSize = size.sp,
    fontWeight = weight,
    lineHeight = lineHeight,
    letterSpacing = letterSpacing,
    lineHeightStyle = CenteredLines,
    fontFeatureSettings = if (tabular) "tnum" else null,
)

/** [text] at a fractional size, like the design's 11.5 or 15 px labels. */
fun text(size: Float, weight: FontWeight = FontWeight.Normal) = text(0, weight).copy(fontSize = size.sp)

/** The design's type roles. Numbers use tabular figures so they line up as they change. */
object NType {
    /** "Where's your music?", "Settings": 32 / 40 · 800. */
    val headline = text(32, FontWeight.ExtraBold, 40.sp, (-0.6).sp)

    /** An album's or a playlist's name: 26 / 32 · 800. */
    val titleLarge = text(26, FontWeight.ExtraBold, 32.sp, (-0.4).sp)

    /** A sheet's title: 22 / 28 · 700. */
    val titleMedium = text(22, FontWeight.Bold, 28.sp)

    /** A settings card's title: 16 / 22 · 700. */
    val titleSmall = text(16, FontWeight.Bold, 22.sp)

    /** A track's title in a list: 16 / 22 · 500. */
    val bodyLarge = text(16, FontWeight.Medium, 22.sp)

    /** The line under a title: 14 / 20 · 400. */
    val bodyMedium = text(14, FontWeight.Normal, 20.sp)

    /** Buttons: 15 / 20 · 700. */
    val labelLarge = text(15, FontWeight.Bold, 20.sp)

    /** Chips and the sort control: 14 / 20 · 600. */
    val labelMedium = text(14, FontWeight.SemiBold, 20.sp)

    /** The navigation bar: 12 / 16 · 700. */
    val labelSmall = text(12, FontWeight.Bold, 16.sp)

    /** Lengths and counts: 13 / 18 · 500, tabular. */
    val numbers = text(13, FontWeight.Medium, 18.sp, tabular = true)
}

/** Material's roles in Figtree, for the components that read them. */
val NTypography = Typography(
    displayLarge = text(57, FontWeight.Normal, 64.sp),
    displayMedium = text(45, FontWeight.Normal, 52.sp),
    displaySmall = text(36, FontWeight.Normal, 44.sp),
    headlineLarge = NType.headline,
    headlineMedium = text(28, FontWeight.ExtraBold, 34.sp, (-0.5).sp),
    headlineSmall = text(24, FontWeight.Bold, 32.sp),
    titleLarge = NType.titleMedium,
    titleMedium = NType.titleSmall,
    titleSmall = text(14, FontWeight.Bold, 20.sp),
    bodyLarge = NType.bodyLarge,
    bodyMedium = NType.bodyMedium,
    bodySmall = text(12, FontWeight.Normal, 16.sp),
    labelLarge = NType.labelLarge,
    labelMedium = NType.labelMedium,
    labelSmall = NType.labelSmall,
)
