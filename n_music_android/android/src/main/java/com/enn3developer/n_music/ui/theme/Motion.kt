package com.enn3developer.n_music.ui.theme

import androidx.compose.animation.core.AnimationVector
import androidx.compose.animation.core.FiniteAnimationSpec
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.TwoWayConverter
import androidx.compose.animation.core.VectorizedFiniteAnimationSpec
import androidx.compose.animation.core.spring
import kotlin.math.exp
import kotlin.math.sqrt

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

    /**
     * The spring a thrown thing leaves on: [noBounce], made softer where it would speed up past
     * [MAX_THROW] times the flick, though never so soft that a slow flick takes much over a
     * second to leave. [distance] is how far it still goes and [velocity] how fast the flick
     * moved it that way, in the same units.
     */
    fun throwing(distance: Float, velocity: Float, visibilityThreshold: Float? = null): FiniteAnimationSpec<Float> {
        if (distance <= 0f || velocity <= 0f) return noBounce(visibilityThreshold)
        // A critically damped spring of angular frequency w, starting at the flick's speed,
        // peaks at (k / w) * e^(w * v / k - 1), with k = w * (w * d - v); past its start only
        // when k > w * v.
        fun peak(w: Float): Float {
            val k = w * (w * distance - velocity)
            return if (k <= w * velocity) velocity else k / w * exp(w * velocity / k - 1)
        }
        val limit = MAX_THROW * velocity
        var high = sqrt(Spring.StiffnessMediumLow)
        if (peak(high) <= limit) return noBounce(visibilityThreshold)
        var low = 0f
        repeat(24) {
            val middle = (low + high) / 2
            if (peak(middle) <= limit) low = middle else high = middle
        }
        return spring(
            Spring.DampingRatioNoBouncy,
            (low * low).coerceAtLeast(Spring.StiffnessVeryLow),
            visibilityThreshold,
        )
    }

    /** When several things change at once, the rest follow the leader this far apart. */
    const val STAGGER_MS = 50L
}

/** This spec, started [millis] late: how a follower trails its leader in a staggered move. */
fun <T> FiniteAnimationSpec<T>.delayed(millis: Long): FiniteAnimationSpec<T> =
    if (millis <= 0) this else Delayed(this, millis * 1_000_000)

private class Delayed<T>(val spec: FiniteAnimationSpec<T>, val delayNanos: Long) : FiniteAnimationSpec<T> {
    override fun <V : AnimationVector> vectorize(converter: TwoWayConverter<T, V>): VectorizedFiniteAnimationSpec<V> =
        DelayedVectorized(spec.vectorize(converter), delayNanos)
}

private class DelayedVectorized<V : AnimationVector>(
    val spec: VectorizedFiniteAnimationSpec<V>,
    val delayNanos: Long,
) : VectorizedFiniteAnimationSpec<V> {
    override fun getValueFromNanos(playTimeNanos: Long, initialValue: V, targetValue: V, initialVelocity: V): V =
        if (playTimeNanos < delayNanos) {
            initialValue
        } else {
            spec.getValueFromNanos(playTimeNanos - delayNanos, initialValue, targetValue, initialVelocity)
        }

    override fun getVelocityFromNanos(playTimeNanos: Long, initialValue: V, targetValue: V, initialVelocity: V): V =
        if (playTimeNanos < delayNanos) {
            initialVelocity
        } else {
            spec.getVelocityFromNanos(playTimeNanos - delayNanos, initialValue, targetValue, initialVelocity)
        }

    override fun getDurationNanos(initialValue: V, targetValue: V, initialVelocity: V): Long =
        delayNanos + spec.getDurationNanos(initialValue, targetValue, initialVelocity)
}
