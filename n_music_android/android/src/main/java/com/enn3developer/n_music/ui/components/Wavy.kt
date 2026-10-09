package com.enn3developer.n_music.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.ui.theme.colors
import kotlin.math.PI
import kotlin.math.sin

/**
 * Material 3 Expressive's wavy progress: a wave up to [progress], then a flat track in
 * [trackColor] after a gap, ending in a dot. With [thumb], a bar stands where the wave ends, as
 * the player's position does. [amplitude] flattens the wave, [phase] makes it travel.
 */
@Composable
fun WavyProgress(
    progress: () -> Float,
    modifier: Modifier = Modifier,
    color: Color = colors.primary,
    trackColor: Color = colors.secondaryContainer,
    stroke: Dp = 4.dp,
    amplitude: () -> Dp = { 2.5.dp },
    wavelength: Dp = 16.dp,
    gap: Dp = 6.dp,
    stopDot: Boolean = true,
    phase: () -> Float = { 0f },
    thumb: Boolean = false,
    thumbWidth: Dp = 5.dp,
) {
    val path = remember { Path() }
    Canvas(modifier) {
        drawWavy(
            path = path,
            fraction = progress().coerceIn(0f, 1f),
            color = color,
            trackColor = trackColor,
            stroke = stroke.toPx(),
            amplitude = amplitude().toPx(),
            wavelength = wavelength.toPx(),
            gap = gap.toPx(),
            stopDot = stopDot,
            phase = phase(),
            thumbWidth = if (thumb) thumbWidth.toPx() else 0f,
        )
    }
}

private fun DrawScope.drawWavy(
    path: Path,
    fraction: Float,
    color: Color,
    trackColor: Color,
    stroke: Float,
    amplitude: Float,
    wavelength: Float,
    gap: Float,
    stopDot: Boolean,
    phase: Float,
    thumbWidth: Float,
) {
    val cap = stroke / 2
    val middle = size.height / 2
    val start = cap
    val end = size.width - cap
    val at = start + (end - start) * fraction
    val waveEnd = if (thumbWidth > 0) at - thumbWidth / 2 - gap else at
    val trackStart = if (thumbWidth > 0) at + thumbWidth / 2 + gap else at + gap + stroke
    if (waveEnd > start) {
        path.reset()
        path.moveTo(start, middle - amplitude * sin(2 * PI.toFloat() * phase))
        var x = start
        while (x < waveEnd) {
            x = minOf(x + 1f, waveEnd)
            val y = middle - amplitude * sin(2 * PI.toFloat() * ((x - start) / wavelength + phase))
            path.lineTo(x, y)
        }
        drawPath(path, color, style = Stroke(width = stroke, cap = StrokeCap.Round))
    }
    if (thumbWidth > 0) {
        val height = size.height
        drawRoundRect(
            color,
            Offset(at - thumbWidth / 2, 0f),
            Size(thumbWidth, height),
            CornerRadius(thumbWidth / 2),
        )
    }
    // The stop dot sits a stroke in from the end, a little apart from the track.
    val dot = size.width - stroke
    val trackEnd = if (stopDot) dot - 3 * cap else end
    if (trackStart < trackEnd) {
        drawLine(trackColor, Offset(trackStart, middle), Offset(trackEnd, middle), stroke, StrokeCap.Round)
    }
    if (stopDot) drawCircle(color, cap, Offset(dot, middle))
}
