package com.enn3developer.n_music.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import kotlinx.coroutines.launch
import kotlin.math.roundToInt

/**
 * A row that a swipe to the left takes away: Remove shows behind it as it goes, and letting go
 * past a third of the way removes it, else it springs back. [shape] is what Remove fills, and
 * [surface] the row's own colour as it slides. [enabled] off, it stays put. Accessibility
 * services get [label] as an action instead, unless the row offers its own.
 */
@Composable
fun SwipeToRemove(
    onRemove: () -> Unit,
    modifier: Modifier = Modifier,
    label: String? = stringResource(R.string.remove),
    shape: Shape = RectangleShape,
    surface: Color = colors.background,
    enabled: Boolean = true,
    content: @Composable () -> Unit,
) {
    val scope = rememberCoroutineScope()
    val offset = remember { Animatable(0f) }
    var width by remember { mutableIntStateOf(1) }
    val remove = stringResource(R.string.remove)
    Box(
        modifier
            .fillMaxWidth()
            .onSizeChanged { width = it.width }
            // Only while it slides, so a row lifted off its list keeps its shadow.
            .graphicsLayer {
                clip = offset.value < 0f
                this.shape = shape
            }
            .then(
                if (label == null) {
                    Modifier
                } else {
                    Modifier.semantics { customActions = listOf(CustomAccessibilityAction(label) { onRemove(); true }) }
                }
            )
    ) {
        Row(
            Modifier
                .matchParentSize()
                .graphicsLayer { alpha = if (offset.value < 0f) 1f else 0f }
                .background(colors.errorContainer)
                .padding(end = 24.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
        ) {
            NIcon(NIcons.Remove, size = 20.dp, tint = colors.onErrorContainer)
            Text(remove, style = text(14, FontWeight.Bold), color = colors.onErrorContainer)
        }
        val sliding = RoundedCornerShape(topEnd = 16.dp, bottomEnd = 16.dp)
        Box(
            Modifier
                .offset { IntOffset(offset.value.roundToInt(), 0) }
                .background(surface, if (offset.value < 0f) sliding else RectangleShape)
                .draggable(
                    rememberDraggableState { delta -> scope.launch { offset.snapTo((offset.value + delta).coerceAtMost(0f)) } },
                    Orientation.Horizontal,
                    enabled = enabled,
                    onDragStopped = { velocity ->
                        if (-offset.value > width / 3f || velocity < -1500f) {
                            offset.animateTo(-width.toFloat(), NMotion.throwing(width + offset.value, -velocity))
                            onRemove()
                        } else {
                            offset.animateTo(0f, NMotion.spatialDefault())
                        }
                    },
                )
        ) {
            content()
        }
    }
}
