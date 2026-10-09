package com.enn3developer.n_music.ui.theme

import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** The design's corner radii. */
object NShapes {
    /** Chips, menu items. */
    val chip = RoundedCornerShape(8.dp)

    /** Fields and selects. */
    val field = RoundedCornerShape(12.dp)

    /** Album tiles, the mini player. */
    val tile = RoundedCornerShape(16.dp)

    /** Groups and cards. */
    val card = RoundedCornerShape(24.dp)

    /** Dialogs and the big cover. */
    val large = RoundedCornerShape(28.dp)

    /** Bottom sheets. */
    val sheet = RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp)

    /** Pills and round buttons. */
    val pill = CircleShape

    /** The first of a row of connected buttons: round outside, tight where it meets the next. */
    fun first(outer: Dp, inner: Dp) =
        RoundedCornerShape(topStart = outer, bottomStart = outer, topEnd = inner, bottomEnd = inner)

    /** A connected button between two others. */
    fun middle(inner: Dp) = RoundedCornerShape(inner)

    /** The last of a row of connected buttons. */
    fun last(outer: Dp, inner: Dp) =
        RoundedCornerShape(topStart = inner, bottomStart = inner, topEnd = outer, bottomEnd = outer)

    /** The first of a column of joined cards. */
    fun top(outer: Dp, inner: Dp) =
        RoundedCornerShape(topStart = outer, topEnd = outer, bottomStart = inner, bottomEnd = inner)

    /** The last of a column of joined cards. */
    fun bottom(outer: Dp, inner: Dp) =
        RoundedCornerShape(topStart = inner, topEnd = inner, bottomStart = outer, bottomEnd = outer)
}
