package com.enn3developer.n_music.ui.components

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalAccessibilityManager
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.enn3developer.n_music.ui.Snack
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import kotlinx.coroutines.delay

/**
 * The latest [snack], numbered so the same message twice shows twice. It rises from behind what
 * is under it, stays a few seconds, longer with an action or a screen reader on, then
 * [onTimeout] lets it go. [onAction] runs its action.
 */
@Composable
fun SnackbarHost(
    snack: Pair<Long, Snack>?,
    onTimeout: (Long) -> Unit,
    onAction: (Long) -> Unit,
    modifier: Modifier = Modifier,
) {
    val accessibility = LocalAccessibilityManager.current
    LaunchedEffect(snack?.first) {
        val (id, shown) = snack ?: return@LaunchedEffect
        val action = shown.action != null
        val base = if (action) 6_000L else 4_000L
        delay(
            accessibility?.calculateRecommendedTimeoutMillis(
                base,
                containsIcons = false,
                containsText = true,
                containsControls = action,
            ) ?: base
        )
        onTimeout(id)
    }
    AnimatedContent(
        snack,
        modifier,
        contentKey = { it?.first },
        transitionSpec = {
            (slideInVertically(NMotion.spatialDefault()) { it } + fadeIn(NMotion.effectsDefault()))
                .togetherWith(slideOutVertically(NMotion.spatialDefault()) { it } + fadeOut(NMotion.effectsFast()))
                .using(SizeTransform(clip = false))
        },
        label = "snack",
    ) { shown ->
        if (shown != null) {
            val (id, message) = shown
            Snackbar(message) { onAction(id) }
        }
    }
}

@Composable
private fun Snackbar(snack: Snack, onAction: () -> Unit) {
    val shape = RoundedCornerShape(12.dp)
    Row(
        Modifier
            .padding(horizontal = 8.dp)
            .fillMaxWidth()
            .heightIn(min = 48.dp)
            .floating(shape)
            .background(colors.inverseSurface, shape)
            .semantics { liveRegion = LiveRegionMode.Polite }
            .padding(start = 16.dp, end = 8.dp, top = 6.dp, bottom = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(
            snack.message,
            style = text(14, lineHeight = 20.sp),
            color = colors.inverseOnSurface,
            modifier = Modifier.weight(1f),
        )
        if (snack.action != null) {
            TextAction(
                snack.action,
                onAction,
                color = colors.inversePrimary,
                height = 36.dp,
                textStyle = text(14, FontWeight.Bold),
                padding = PaddingValues(horizontal = 12.dp),
            )
        }
    }
}
