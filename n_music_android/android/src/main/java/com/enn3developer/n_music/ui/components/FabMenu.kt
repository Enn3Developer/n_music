package com.enn3developer.n_music.ui.components

import androidx.activity.compose.BackHandler
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.lerp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.delayed
import com.enn3developer.n_music.ui.theme.text

/** One of a [FabMenu]'s choices. */
data class FabItem(val icon: ImageVector, val label: String, val onClick: () -> Unit)

/** How much bigger the + is as the ×, for its arms to reach the corners of the close icon. */
private const val CROSS_SCALE = 1.21f

/**
 * A floating button that opens into [items] over a scrim, as Material 3's FAB menu does: it
 * rounds into a close button as they rise out of it, the nearest first. An item, the scrim or
 * back closes it. [description] names the closed button; it sits [bottom] above the bottom of
 * the screen, and grows in and out as [visible] turns on and off.
 */
@Composable
fun FabMenu(
    visible: Boolean,
    open: Boolean,
    onOpenChange: (Boolean) -> Unit,
    description: String,
    items: List<FabItem>,
    bottom: Dp,
    modifier: Modifier = Modifier,
) {
    val present by animateFloatAsState(
        if (visible) 1f else 0f,
        if (visible) NMotion.spatialDefault() else NMotion.effectsFast(),
        label = "fab",
    )
    val expand by animateFloatAsState(if (open) 1f else 0f, NMotion.spatialDefault(), label = "expand")
    val scrim by animateFloatAsState(
        if (open) 1f else 0f,
        if (open) NMotion.effectsDefault() else NMotion.effectsFast(),
        label = "scrim",
    )
    BackHandler(open) { onOpenChange(false) }
    Box(modifier.fillMaxSize()) {
        if (scrim > 0f) {
            Box(
                Modifier
                    .fillMaxSize()
                    .graphicsLayer { alpha = scrim }
                    .background(colors.scrim)
                    .then(if (open) Modifier.pointerInput(Unit) { detectTapGestures { onOpenChange(false) } } else Modifier)
            )
        }
        Column(
            Modifier
                .align(Alignment.BottomEnd)
                .padding(end = 16.dp, bottom = bottom),
            horizontalAlignment = Alignment.End,
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items.forEachIndexed { index, item ->
                // Opening, the nearest rises first; closing, they all go at once.
                val shown by animateFloatAsState(
                    if (open) 1f else 0f,
                    if (open) {
                        NMotion.spatialDefault<Float>().delayed((items.size - 1 - index) * NMotion.STAGGER_MS)
                    } else {
                        NMotion.effectsFast()
                    },
                    label = "item",
                )
                if (open || shown > 0f) {
                    FabMenuItem(item, shown, enabled = open) {
                        onOpenChange(false)
                        item.onClick()
                    }
                }
            }
            if (present > 0f) {
                FabButton(
                    expand = expand,
                    open = open,
                    description = description,
                    onClick = { onOpenChange(!open) },
                    modifier = Modifier.graphicsLayer {
                        alpha = present.coerceIn(0f, 1f)
                        scaleX = 0.6f + 0.4f * present
                        scaleY = 0.6f + 0.4f * present
                    },
                )
            }
        }
    }
}

/**
 * The menu's button: a rounded square with a + while closed, turning into a circle with an × in
 * the primary colour as [expand] goes to 1.
 */
@Composable
private fun FabButton(expand: Float, open: Boolean, description: String, onClick: () -> Unit, modifier: Modifier) {
    val shape = RoundedCornerShape(lerp(16.dp, 28.dp, expand.coerceIn(0f, 1f)))
    val container by animateColorAsState(
        if (open) colors.primary else colors.primaryContainer,
        NMotion.effectsDefault(),
        label = "container",
    )
    val content by animateColorAsState(
        if (open) colors.onPrimary else colors.onPrimaryContainer,
        NMotion.effectsDefault(),
        label = "content",
    )
    val close = stringResource(R.string.close_menu)
    Box(
        modifier
            .size(56.dp)
            .floating(shape)
            .clip(shape)
            .background(container)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = if (open) close else description },
        contentAlignment = Alignment.Center,
    ) {
        // The + turned an eighth of the way round is the ×.
        NIcon(
            NIcons.Add,
            Modifier.graphicsLayer {
                rotationZ = 45f * expand
                val scale = 1f + (CROSS_SCALE - 1f) * expand
                scaleX = scale
                scaleY = scale
            },
            tint = content,
        )
    }
}

/** One of the menu's choices: a pill with its icon and name, [shown] as far as it has risen. */
@Composable
private fun FabMenuItem(item: FabItem, shown: Float, enabled: Boolean, onClick: () -> Unit) {
    val shape = RoundedCornerShape(28.dp)
    ButtonSurface(
        onClick = onClick,
        shape = shape,
        container = colors.primaryContainer,
        content = colors.onPrimaryContainer,
        modifier = Modifier
            .height(56.dp)
            .graphicsLayer {
                alpha = shown.coerceIn(0f, 1f)
                val scale = 0.7f + 0.3f * shown
                scaleX = scale
                scaleY = scale
                transformOrigin = TransformOrigin(1f, 1f)
                translationY = (1f - shown) * 16.dp.toPx()
            }
            .floating(shape),
        enabled = enabled,
        padding = PaddingValues(start = 18.dp, end = 24.dp),
        arrangement = Arrangement.spacedBy(12.dp),
    ) {
        NIcon(item.icon)
        Text(item.label, style = text(16, FontWeight.Bold))
    }
}
