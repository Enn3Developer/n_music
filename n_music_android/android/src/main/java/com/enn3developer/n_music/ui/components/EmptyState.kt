package com.enn3developer.n_music.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/**
 * What a page shows with nothing to list: an [icon], what is missing, what to do about it, and a
 * button that does it. [bottom] keeps it clear of what floats over the page.
 */
@Composable
fun EmptyState(
    icon: ImageVector,
    title: String,
    message: String,
    modifier: Modifier = Modifier,
    action: String? = null,
    tonal: Boolean = false,
    bottom: Dp = 120.dp,
    onAction: () -> Unit = {},
) {
    Column(
        modifier
            .fillMaxSize()
            .padding(start = 40.dp, end = 40.dp, bottom = bottom),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Box(
            Modifier
                .size(72.dp)
                .background(colors.surfaceHigh, RoundedCornerShape(24.dp)),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(icon, size = 32.dp, tint = colors.onSurfaceVariant)
        }
        Text(
            title,
            style = text(20, FontWeight.Bold, 26.sp),
            color = colors.onSurface,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = 20.dp),
        )
        Text(
            message,
            style = text(15, lineHeight = 22.sp),
            color = colors.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = 8.dp),
        )
        if (action != null) {
            PillButton(
                action,
                onAction,
                modifier = Modifier.padding(top = 20.dp),
                style = if (tonal) PillStyle.TONAL else PillStyle.FILLED,
                padding = PaddingValues(horizontal = 24.dp),
            )
        }
    }
}
