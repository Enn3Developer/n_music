package com.enn3developer.n_music.ui.components

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.layout
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.offset
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.LocalPageMargins
import com.enn3developer.n_music.ui.LocalWindowLayout
import com.enn3developer.n_music.ui.WindowLayout
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors

/** A page's bar under the status bar: Back, and the page's [actions] at its end. A tablet's is lower. */
@Composable
fun PageBar(onBack: () -> Unit, modifier: Modifier = Modifier, actions: @Composable RowScope.() -> Unit = {}) {
    val margins = LocalPageMargins.current
    val tablet = LocalWindowLayout.current == WindowLayout.TABLET
    Row(
        modifier
            .windowInsetsPadding(WindowInsets.statusBars)
            .fillMaxWidth()
            .height(if (tablet) 56.dp else 64.dp)
            // The buttons' own 12 dp make up the rest of the page's margins.
            .padding(start = margins.startLess(12.dp), end = margins.endLess(12.dp)),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        NIconButton(NIcons.Back, stringResource(R.string.back), onBack, tint = colors.onSurface)
        Spacer(Modifier.weight(1f))
        actions()
    }
}

/**
 * Takes [top], [bottom], [start] and [end] less room than it draws over, as negative margins do:
 * a link keeps a large touch area without pushing the text around it away.
 */
fun Modifier.margins(top: Dp = 0.dp, bottom: Dp = 0.dp, start: Dp = 0.dp, end: Dp = 0.dp): Modifier =
    layout { measurable, constraints ->
        val sides = start.roundToPx() + end.roundToPx()
        val placeable = measurable.measure(constraints.offset(horizontal = sides))
        val above = top.roundToPx()
        layout((placeable.width - sides).coerceAtLeast(0), placeable.height - above - bottom.roundToPx()) {
            placeable.placeRelative(-start.roundToPx(), -above)
        }
    }
