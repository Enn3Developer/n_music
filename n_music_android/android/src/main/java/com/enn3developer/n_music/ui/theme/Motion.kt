package com.enn3developer.n_music.ui.theme

import androidx.compose.animation.core.FiniteAnimationSpec
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring

/**
 * The springs every move and fade runs on: the six of Material 3 Expressive's motion scheme,
 * plus one without bounce for screen-sized edges. Spatial springs move and resize things and
 * overshoot a little; effects springs fade and recolour and never overshoot.
 */
object NMotion {
    /** Switch thumbs, checks, presses, a chip lifting off. */
    fun <T> spatialFast(visibilityThreshold: T? = null): FiniteAnimationSpec<T> =
        spring(0.6f, 800f, visibilityThreshold)

    /** Sheets, covers, list items making room, text sliding in. */
    fun <T> spatialDefault(visibilityThreshold: T? = null): FiniteAnimationSpec<T> =
        spring(0.8f, 380f, visibilityThreshold)

    /** Full-screen moves, like the player opening and closing. */
    fun <T> spatialSlow(visibilityThreshold: T? = null): FiniteAnimationSpec<T> =
        spring(0.8f, 200f, visibilityThreshold)

    /** Icons swapping, press states, text on its way out. */
    fun <T> effectsFast(visibilityThreshold: T? = null): FiniteAnimationSpec<T> =
        spring(1f, 3800f, visibilityThreshold)

    /** Content fading in, scrims, captions. */
    fun <T> effectsDefault(visibilityThreshold: T? = null): FiniteAnimationSpec<T> =
        spring(1f, 1600f, visibilityThreshold)

    /** Large colour changes, like a new accent colour. */
    fun <T> effectsSlow(visibilityThreshold: T? = null): FiniteAnimationSpec<T> =
        spring(1f, 800f, visibilityThreshold)

    /** Pages, the player's container, and throws off screen. */
    fun <T> noBounce(visibilityThreshold: T? = null): FiniteAnimationSpec<T> =
        spring(Spring.DampingRatioNoBouncy, Spring.StiffnessMediumLow, visibilityThreshold)

    /** A throw never gets faster than this many times the flick. */
    const val MAX_THROW = 1.3f

    /** When several things change at once, the rest follow the leader this far apart. */
    const val STAGGER_MS = 50L
}
