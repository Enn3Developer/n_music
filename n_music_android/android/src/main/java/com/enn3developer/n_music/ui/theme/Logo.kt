package com.enn3developer.n_music.ui.theme

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ColorMatrix
import androidx.compose.ui.res.colorResource
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.Dp
import com.enn3developer.n_music.R

/** The app icon on its tile, as Welcome and About show it: the launcher icon's layers, whole. */
@Composable
fun AppLogo(size: Dp, modifier: Modifier = Modifier, corner: Dp = size * 18 / 64) {
    Image(
        painterResource(R.drawable.ic_launcher_foreground),
        contentDescription = null,
        modifier = modifier
            .size(size)
            .clip(RoundedCornerShape(corner))
            .background(colorResource(R.color.ic_launcher_background)),
    )
}

/**
 * The launcher's monochrome icon sized to [size], with its marks in [marks] and its N in [letter]:
 * the layer draws the marks black and the N white, and the filter maps one to each.
 */
@Composable
fun LogoMark(marks: Color, letter: Color, size: Dp, modifier: Modifier = Modifier) {
    val filter = remember(marks, letter) { ColorFilter.colorMatrix(blackAndWhiteTo(marks, letter)) }
    Image(
        painterResource(R.drawable.ic_launcher_monochrome),
        contentDescription = null,
        modifier = modifier.size(size),
        colorFilter = filter,
    )
}

/** Takes black to [black] and white to [white] by the red channel, keeping alpha. */
private fun blackAndWhiteTo(black: Color, white: Color) = ColorMatrix(
    floatArrayOf(
        white.red - black.red, 0f, 0f, 0f, black.red * 255f,
        white.green - black.green, 0f, 0f, 0f, black.green * 255f,
        white.blue - black.blue, 0f, 0f, 0f, black.blue * 255f,
        0f, 0f, 0f, 1f, 0f,
    )
)
