package com.enn3developer.n_music.ui

import android.app.Activity
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat
import com.enn3developer.n_music.Theme

// The Material palette of the Slint app, color for color.

private val LightColors = lightColorScheme(
    primary = Color(68, 94, 145),
    onPrimary = Color(255, 255, 255),
    primaryContainer = Color(216, 226, 255),
    onPrimaryContainer = Color(0, 26, 66),
    inversePrimary = Color(173, 198, 255),
    secondary = Color(87, 94, 113),
    onSecondary = Color(255, 255, 255),
    secondaryContainer = Color(219, 226, 249),
    onSecondaryContainer = Color(20, 27, 44),
    tertiary = Color(113, 85, 115),
    onTertiary = Color(255, 255, 255),
    tertiaryContainer = Color(252, 215, 251),
    onTertiaryContainer = Color(41, 19, 45),
    background = Color(249, 249, 255),
    onBackground = Color(26, 27, 32),
    surface = Color(249, 249, 255),
    onSurface = Color(26, 27, 32),
    surfaceVariant = Color(225, 226, 236),
    onSurfaceVariant = Color(68, 71, 79),
    surfaceTint = Color(68, 94, 145),
    inverseSurface = Color(47, 48, 54),
    inverseOnSurface = Color(240, 240, 247),
    error = Color(186, 26, 26),
    onError = Color(255, 255, 255),
    errorContainer = Color(255, 218, 214),
    onErrorContainer = Color(65, 0, 2),
    outline = Color(117, 119, 127),
    outlineVariant = Color(196, 198, 208),
    scrim = Color(0, 0, 0),
    surfaceBright = Color(246, 246, 255),
    surfaceContainer = Color(238, 237, 244),
    surfaceContainerHigh = Color(232, 231, 239),
    surfaceContainerHighest = Color(226, 226, 233),
    surfaceContainerLow = Color(243, 243, 250),
    surfaceContainerLowest = Color(255, 255, 255),
    surfaceDim = Color(217, 217, 224),
)

private val DarkColors = darkColorScheme(
    primary = Color(173, 198, 255),
    onPrimary = Color(17, 47, 96),
    primaryContainer = Color(43, 70, 120),
    onPrimaryContainer = Color(216, 226, 255),
    inversePrimary = Color(68, 94, 145),
    secondary = Color(191, 198, 220),
    onSecondary = Color(41, 48, 65),
    secondaryContainer = Color(63, 71, 89),
    onSecondaryContainer = Color(219, 226, 249),
    tertiary = Color(222, 188, 223),
    onTertiary = Color(64, 40, 67),
    tertiaryContainer = Color(88, 62, 91),
    onTertiaryContainer = Color(252, 215, 251),
    background = Color(17, 19, 24),
    onBackground = Color(226, 226, 233),
    surface = Color(17, 19, 24),
    onSurface = Color(226, 226, 233),
    surfaceVariant = Color(68, 71, 79),
    onSurfaceVariant = Color(196, 198, 208),
    surfaceTint = Color(173, 198, 255),
    inverseSurface = Color(226, 226, 233),
    inverseOnSurface = Color(47, 48, 54),
    error = Color(255, 180, 171),
    onError = Color(105, 0, 5),
    errorContainer = Color(147, 0, 10),
    onErrorContainer = Color(255, 218, 214),
    outline = Color(142, 144, 153),
    outlineVariant = Color(68, 71, 79),
    scrim = Color(0, 0, 0),
    surfaceBright = Color(55, 57, 62),
    surfaceContainer = Color(30, 31, 37),
    surfaceContainerHigh = Color(40, 42, 47),
    surfaceContainerHighest = Color(51, 53, 58),
    surfaceContainerLow = Color(26, 27, 32),
    surfaceContainerLowest = Color(12, 14, 19),
    surfaceDim = Color(17, 19, 24),
)

/** The Slint app's palette, light or dark as [theme] says, with system bars to match. */
@Composable
fun NMusicTheme(theme: Theme, content: @Composable () -> Unit) {
    val dark = when (theme) {
        Theme.SYSTEM -> isSystemInDarkTheme()
        Theme.LIGHT -> false
        Theme.DARK -> true
    }
    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !dark
                isAppearanceLightNavigationBars = !dark
            }
        }
    }
    MaterialTheme(colorScheme = if (dark) DarkColors else LightColors, content = content)
}
