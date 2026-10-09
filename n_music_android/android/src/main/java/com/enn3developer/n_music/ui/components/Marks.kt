package com.enn3developer.n_music.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors

/** A radio button's ring, with its dot while [on]. */
@Composable
fun RadioMark(on: Boolean, modifier: Modifier = Modifier) {
    val ring by animateColorAsState(
        if (on) colors.primary else colors.onSurfaceVariant,
        NMotion.effectsFast(),
        label = "ring",
    )
    val dot by animateFloatAsState(if (on) 1f else 0f, NMotion.spatialFast(), label = "dot")
    Box(
        modifier
            .size(20.dp)
            .border(2.dp, ring, CircleShape),
        contentAlignment = Alignment.Center,
    ) {
        Box(
            Modifier
                .size(10.dp)
                .scale(dot)
                .background(colors.primary, CircleShape)
        )
    }
}

/** A checkbox: an outlined square, filled with a bold check while [on]. */
@Composable
fun CheckMark(on: Boolean, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(6.dp)
    val fill by animateColorAsState(if (on) colors.primary else Color.Transparent, NMotion.effectsFast(), label = "fill")
    val edge by animateColorAsState(
        if (on) colors.primary else colors.onSurfaceVariant,
        NMotion.effectsFast(),
        label = "edge",
    )
    val check by animateFloatAsState(if (on) 1f else 0f, NMotion.spatialFast(), label = "check")
    Box(
        modifier
            .size(24.dp)
            .background(fill, shape)
            .border(2.dp, edge, shape),
        contentAlignment = Alignment.Center,
    ) {
        NIcon(NIcons.CheckBold, Modifier.scale(check), size = 18.dp, tint = colors.onPrimary)
    }
}
