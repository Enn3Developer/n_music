package com.enn3developer.n_music.ui.theme

import android.app.Activity
import android.os.Build
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MotionScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat
import com.enn3developer.n_music.Theme

/** The colours [theme] and [accent] pick; Wallpaper falls back to Amber before Android 12. */
@Composable
fun rememberColors(theme: Theme, accent: Accent): NColors {
    val dark = when (theme) {
        Theme.SYSTEM -> isSystemInDarkTheme()
        Theme.LIGHT -> false
        Theme.DARK -> true
    }
    val context = LocalContext.current
    return remember(dark, accent, context) {
        when {
            accent == Accent.WALLPAPER && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S ->
                dynamicColors(
                    if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context),
                    dark,
                )

            dark -> darkColors(accent)
            else -> lightColors(accent)
        }
    }
}

/**
 * The design's theme: Figtree, the graphite surfaces with [accent], and Material 3 Expressive's
 * motion scheme. A new accent or theme recolours everything on the slow effects spring.
 */
@Composable
fun NTheme(theme: Theme, accent: Accent, content: @Composable () -> Unit) {
    val colors = animated(rememberColors(theme, accent))
    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as? Activity)?.window ?: return@SideEffect
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !colors.dark
                isAppearanceLightNavigationBars = !colors.dark
            }
        }
    }
    CompositionLocalProvider(LocalNColors provides colors) {
        MaterialTheme(
            colorScheme = colors.toMaterial(),
            typography = NTypography,
            motionScheme = MotionScheme.expressive(),
            content = content,
        )
    }
}

/** The design's colours, for the code that reads them. */
val colors: NColors
    @Composable get() = LocalNColors.current

@Composable
private fun animated(target: NColors): NColors {
    return NColors(
        dark = target.dark,
        background = target.background.animate(),
        surfaceLowest = target.surfaceLowest.animate(),
        surfaceLow = target.surfaceLow.animate(),
        surface = target.surface.animate(),
        surfaceHigh = target.surfaceHigh.animate(),
        surfaceHighest = target.surfaceHighest.animate(),
        onSurface = target.onSurface.animate(),
        onSurfaceVariant = target.onSurfaceVariant.animate(),
        onSurfaceQuiet = target.onSurfaceQuiet.animate(),
        outline = target.outline.animate(),
        outlineVariant = target.outlineVariant.animate(),
        primary = target.primary.animate(),
        onPrimary = target.onPrimary.animate(),
        primaryContainer = target.primaryContainer.animate(),
        onPrimaryContainer = target.onPrimaryContainer.animate(),
        secondaryContainer = target.secondaryContainer.animate(),
        onSecondaryContainer = target.onSecondaryContainer.animate(),
        tint = target.tint.animate(),
        error = target.error.animate(),
        errorContainer = target.errorContainer.animate(),
        onErrorContainer = target.onErrorContainer.animate(),
        scrim = target.scrim.animate(),
        inverseSurface = target.inverseSurface.animate(),
        inverseOnSurface = target.inverseOnSurface.animate(),
        inversePrimary = target.inversePrimary.animate(),
        shadow = target.shadow.animate(),
    )
}

@Composable
private fun Color.animate(): Color =
    animateColorAsState(this, NMotion.effectsSlow(), label = "colour").value

/** Material's roles from the design's, for the Material components the app uses. */
private fun NColors.toMaterial(): ColorScheme {
    val base = if (dark) darkColorScheme() else lightColorScheme()
    return base.copy(
        primary = primary,
        onPrimary = onPrimary,
        primaryContainer = primaryContainer,
        onPrimaryContainer = onPrimaryContainer,
        inversePrimary = inversePrimary,
        secondary = onSecondaryContainer,
        onSecondary = secondaryContainer,
        secondaryContainer = secondaryContainer,
        onSecondaryContainer = onSecondaryContainer,
        background = background,
        onBackground = onSurface,
        surface = background,
        onSurface = onSurface,
        surfaceVariant = surfaceHighest,
        onSurfaceVariant = onSurfaceVariant,
        surfaceTint = primary,
        inverseSurface = inverseSurface,
        inverseOnSurface = inverseOnSurface,
        error = error,
        onError = background,
        errorContainer = errorContainer,
        onErrorContainer = onErrorContainer,
        outline = outline,
        outlineVariant = outlineVariant,
        scrim = Color.Black,
        surfaceBright = surfaceHighest,
        surfaceDim = background,
        surfaceContainer = surface,
        surfaceContainerHigh = surfaceHigh,
        surfaceContainerHighest = surfaceHighest,
        surfaceContainerLow = surfaceLow,
        surfaceContainerLowest = surfaceLowest,
    )
}
