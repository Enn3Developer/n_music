package com.enn3developer.n_music.ui.theme

import androidx.compose.material3.ColorScheme
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp

/**
 * The design's colour roles. Surfaces are graphite whatever the accent; the accent fills the
 * primary and secondary roles. [surfaceLow] to [surfaceHighest] are Material's surface
 * containers, [onSurfaceQuiet] the quietest text.
 */
@Immutable
data class NColors(
    val dark: Boolean,
    val background: Color,
    val surfaceLowest: Color,
    val surfaceLow: Color,
    val surface: Color,
    val surfaceHigh: Color,
    val surfaceHighest: Color,
    val onSurface: Color,
    val onSurfaceVariant: Color,
    val onSurfaceQuiet: Color,
    val outline: Color,
    val outlineVariant: Color,
    val primary: Color,
    val onPrimary: Color,
    val primaryContainer: Color,
    val onPrimaryContainer: Color,
    val secondaryContainer: Color,
    val onSecondaryContainer: Color,
    /** The wash behind the playing row. */
    val tint: Color,
    val error: Color,
    val errorContainer: Color,
    val onErrorContainer: Color,
    val scrim: Color,
    val inverseSurface: Color,
    val inverseOnSurface: Color,
    val inversePrimary: Color,
    /** The colour of the soft shadow under floating surfaces. */
    val shadow: Color,
)

val LocalNColors = staticCompositionLocalOf { amberDark() }

/** A preset accent: its fill and the ink on it, and the darker primary of the light theme. */
enum class Accent(
    val stored: String,
    val fill: Color,
    val onFill: Color,
    val lightPrimary: Color,
    val darkSecondary: Color,
    val darkOnSecondary: Color,
    val lightSecondary: Color,
    val lightOnSecondary: Color,
) {
    /** Android's dynamic colour, from Android 12. Its fields describe the fallback, Amber. */
    WALLPAPER(
        "Wallpaper", Color(0xFFF0A13A), Color(0xFF1A1206), Color(0xFF8F5100),
        Color(0xFF3E3121), Color(0xFFFFDDB3), Color(0xFFF7DFC0), Color(0xFF3B2606),
    ),
    AMBER(
        "Amber", Color(0xFFF0A13A), Color(0xFF1A1206), Color(0xFF8F5100),
        Color(0xFF3E3121), Color(0xFFFFDDB3), Color(0xFFF7DFC0), Color(0xFF3B2606),
    ),

    // The secondary roles of the other accents take Amber's tones and chroma on their own hue.
    GREEN(
        "Green", Color(0xFF4FC98E), Color(0xFF04150D), Color(0xFF176E46),
        Color(0xFF29362E), Color(0xFFCBEAD4), Color(0xFFD3E8D8), Color(0xFF103020),
    ),
    TEAL(
        "Teal", Color(0xFF3CC4C0), Color(0xFF031615), Color(0xFF0D6C69),
        Color(0xFF273635), Color(0xFFC5EAE7), Color(0xFFCFE7E5), Color(0xFF02302F),
    ),
    BLUE(
        "Blue", Color(0xFF6AA6FF), Color(0xFF06111F), Color(0xFF1F5CC2),
        Color(0xFF2E333E), Color(0xFFD6E3FE), Color(0xFFDBE3F5), Color(0xFF1B2B42),
    ),
    VIOLET(
        "Violet", Color(0xFFA68BFA), Color(0xFF120A26), Color(0xFF6B45D1),
        Color(0xFF35313E), Color(0xFFE8DEFE), Color(0xFFE7DFF5), Color(0xFF2D2642),
    ),
    ROSE(
        "Rose", Color(0xFFF2779B), Color(0xFF22070F), Color(0xFFB52A5B),
        Color(0xFF412E32), Color(0xFFFFD9E0), Color(0xFFFEDAE0), Color(0xFF412029),
    ),
}

fun amberDark() = darkColors(Accent.AMBER)

/** The graphite dark theme with [accent]. */
fun darkColors(accent: Accent) = NColors(
    dark = true,
    background = Color(0xFF111316),
    surfaceLowest = Color(0xFF0C0E10),
    surfaceLow = Color(0xFF16191D),
    surface = Color(0xFF1A1D21),
    surfaceHigh = Color(0xFF202328),
    surfaceHighest = Color(0xFF262A30),
    onSurface = Color(0xFFECEEF1),
    onSurfaceVariant = Color(0xFFA4ABB6),
    onSurfaceQuiet = Color(0xFF868E9A),
    outline = Color(0xFF6E7682),
    outlineVariant = Color(0xFF2E333A),
    primary = accent.fill,
    onPrimary = accent.onFill,
    primaryContainer = accent.fill,
    onPrimaryContainer = accent.onFill,
    secondaryContainer = accent.darkSecondary,
    onSecondaryContainer = accent.darkOnSecondary,
    tint = accent.fill.copy(alpha = 0.12f),
    error = Color(0xFFFF8A80),
    errorContainer = Color(0xFF4A2522),
    onErrorContainer = Color(0xFFFFD9D5),
    scrim = Color.Black.copy(alpha = 0.6f),
    inverseSurface = Color(0xFFECEEF1),
    inverseOnSurface = Color(0xFF1A1D21),
    inversePrimary = accent.lightPrimary,
    shadow = Color.Black.copy(alpha = 0.45f),
)

/** The graphite light theme with [accent]. */
fun lightColors(accent: Accent) = NColors(
    dark = false,
    background = Color(0xFFF6F7F9),
    surfaceLowest = Color(0xFFFFFFFF),
    surfaceLow = Color(0xFFF0F2F5),
    surface = Color(0xFFEAEDF1),
    surfaceHigh = Color(0xFFE4E7EC),
    surfaceHighest = Color(0xFFDDE1E7),
    onSurface = Color(0xFF15171A),
    onSurfaceVariant = Color(0xFF4D5560),
    onSurfaceQuiet = Color(0xFF626A76),
    outline = Color(0xFF7A828E),
    outlineVariant = Color(0xFFD3D8DF),
    primary = accent.lightPrimary,
    onPrimary = Color.White,
    primaryContainer = accent.fill,
    onPrimaryContainer = accent.onFill,
    secondaryContainer = accent.lightSecondary,
    onSecondaryContainer = accent.lightOnSecondary,
    tint = accent.lightPrimary.copy(alpha = 0.08f),
    error = Color(0xFFB3261E),
    errorContainer = Color(0xFFFCEEEE),
    onErrorContainer = Color(0xFF7A1C16),
    scrim = Color.Black.copy(alpha = 0.32f),
    inverseSurface = Color(0xFF2A2E35),
    inverseOnSurface = Color(0xFFF1F2F4),
    inversePrimary = accent.fill,
    shadow = Color(0xFF14181E).copy(alpha = 0.16f),
)

/** Every role from Android's dynamic colours, which seed the surfaces too. */
fun dynamicColors(scheme: ColorScheme, dark: Boolean) = NColors(
    dark = dark,
    background = scheme.surface,
    surfaceLowest = scheme.surfaceContainerLowest,
    surfaceLow = scheme.surfaceContainerLow,
    surface = scheme.surfaceContainer,
    surfaceHigh = scheme.surfaceContainerHigh,
    surfaceHighest = scheme.surfaceContainerHighest,
    onSurface = scheme.onSurface,
    onSurfaceVariant = scheme.onSurfaceVariant,
    onSurfaceQuiet = lerp(scheme.onSurfaceVariant, scheme.outline, 0.5f),
    outline = scheme.outline,
    outlineVariant = scheme.outlineVariant,
    primary = scheme.primary,
    onPrimary = scheme.onPrimary,
    primaryContainer = scheme.primaryContainer,
    onPrimaryContainer = scheme.onPrimaryContainer,
    secondaryContainer = scheme.secondaryContainer,
    onSecondaryContainer = scheme.onSecondaryContainer,
    tint = scheme.primary.copy(alpha = if (dark) 0.12f else 0.08f),
    error = scheme.error,
    errorContainer = scheme.errorContainer,
    onErrorContainer = scheme.onErrorContainer,
    scrim = Color.Black.copy(alpha = if (dark) 0.6f else 0.32f),
    inverseSurface = scheme.inverseSurface,
    inverseOnSurface = scheme.inverseOnSurface,
    inversePrimary = scheme.inversePrimary,
    shadow = if (dark) Color.Black.copy(alpha = 0.45f) else Color(0xFF14181E).copy(alpha = 0.16f),
)
