package com.enn3developer.n_music.ui.theme

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** The pixel N of the app icon, on its 49 × 49 grid. */
private object LogoPaths {
    const val RED =
        "M18 19h6v2h-6zM25 19h6v2h-6zM18 16h6v2h-6zM25 16h6v2h-6zM18 13h6v2h-6zM25 13h6v2h-6zM13 19h4v2h-4zM13 16h4v2h-4zM32 19h4v2h-4zM32 16h4v2h-4z"
    const val ORANGE =
        "M18 28h6v2h-6zM25 28h6v2h-6zM18 25h6v2h-6zM25 25h6v2h-6zM18 22h6v2h-6zM25 22h6v2h-6zM13 28h4v2h-4zM13 25h4v2h-4zM13 22h4v2h-4zM32 28h4v2h-4zM32 25h4v2h-4zM32 22h4v2h-4zM37 28h5v2h-5zM37 25h5v2h-5zM37 22h5v2h-5zM7 28h5v2h-5zM7 25h5v2h-5zM7 22h5v2h-5z"
    const val YELLOW =
        "M18 35h6v1h-6zM18 33h6v1h-6zM25 35h6v1h-6zM25 33h6v1h-6zM18 31h6v1h-6zM25 31h6v1h-6zM13 35h4v1h-4zM10 35h2v1h-2zM9 33h3v1h-3zM8 31h4v1h-4zM37 35h2v1h-2zM37 33h3v1h-3zM37 31h4v1h-4zM13 33h4v1h-4zM13 31h4v1h-4zM32 35h4v1h-4zM32 33h4v1h-4zM32 31h4v1h-4z"
    const val GREEN =
        "M18 39h6v1h-6zM18 37h6v1h-6zM25 39h6v1h-6zM25 37h6v1h-6zM14 39h3v1h-3zM13 37h4v1h-4zM32 39h3v1h-3zM32 37h4v1h-4z"
    const val DARK_GREEN = "M18 41h6v1h-6zM25 41h6v1h-6z"
    const val SPARKS =
        "M13 12h1v1h-1zM8 20h1v1h-1zM16 10h1v1h-1zM14 11h2v1h-2zM17 9h4v1h-4zM21 8h7v1h-7zM9 17h1v2h-1zM40 20h1v1h-1zM35 12h1v1h-1zM32 10h1v1h-1zM38 16h1v1h-1zM36 14h2v1h-2zM33 11h2v1h-2zM28 9h4v1h-4zM39 17h1v2h-1zM10 16h1v1h-1zM11 14h2v1h-2z"
    const val N = "M21 20h2v8h-2zM23 21h1v4h-1zM24 22h1v4h-1zM25 23h1v4h-1zM26 20h2v8h-2z"
}

private fun logoVector(build: ImageVector.Builder.() -> Unit) = ImageVector.Builder(
    defaultWidth = 49.dp,
    defaultHeight = 49.dp,
    viewportWidth = 49f,
    viewportHeight = 49f,
).apply(build).build()

/** The app icon's colours, for its own black tile. */
val LogoColors: ImageVector by lazy {
    logoVector {
        for ((path, color) in listOf(
            LogoPaths.RED to Color(0xFFFF0000),
            LogoPaths.ORANGE to Color(0xFFFF6600),
            LogoPaths.YELLOW to Color(0xFFFFFF00),
            LogoPaths.GREEN to Color(0xFF00FF00),
            LogoPaths.DARK_GREEN to Color(0xFF008000),
            LogoPaths.SPARKS to Color.White,
            LogoPaths.N to Color.White,
        )) {
            addPath(addPathNodes(path), fill = SolidColor(color))
        }
    }
}

/** The app icon drawn in [marks], with its N in [letter]: covers not read yet. */
fun logoMono(marks: Color, letter: Color): ImageVector = logoVector {
    val all = listOf(
        LogoPaths.RED,
        LogoPaths.ORANGE,
        LogoPaths.YELLOW,
        LogoPaths.GREEN,
        LogoPaths.DARK_GREEN,
        LogoPaths.SPARKS,
    ).joinToString("")
    addPath(addPathNodes(all), fill = SolidColor(marks))
    addPath(addPathNodes(LogoPaths.N), fill = SolidColor(letter))
}

/** The app icon on its black tile, as Welcome and About show it. */
@Composable
fun AppLogo(size: Dp, modifier: Modifier = Modifier, corner: Dp = size * 18 / 64) {
    Box(
        modifier
            .size(size)
            .clip(RoundedCornerShape(corner))
            .background(Color.Black)
    ) {
        Image(LogoColors, contentDescription = null, modifier = Modifier.size(size))
    }
}

/** The icon in one colour with its N in another, sized to [size]. */
@Composable
fun LogoMark(marks: Color, letter: Color, size: Dp, modifier: Modifier = Modifier) {
    val vector = remember(marks, letter) { logoMono(marks, letter) }
    Image(vector, contentDescription = null, modifier = modifier.size(size))
}
