package com.enn3developer.n_music.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** The app's three places. */
enum class Tab(val icon: ImageVector, val label: Int) {
    LIBRARY(NIcons.Library, R.string.nav_library),
    PLAYLISTS(NIcons.Playlist, R.string.nav_playlists),
    SOURCES(NIcons.Sources, R.string.nav_sources),
}

/** The bottom bar of phones: [Tab]s with their pill indicator. */
@Composable
fun NavBar(
    selected: Tab,
    onSelect: (Tab) -> Unit,
    sourcesBusy: Boolean,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier
            .fillMaxWidth()
            .background(colors.surface)
            .windowInsetsPadding(WindowInsets.navigationBars)
            .height(64.dp),
    ) {
        for (tab in Tab.entries) {
            NavItem(
                tab.icon,
                stringResource(tab.label),
                selected = tab == selected,
                onClick = { onSelect(tab) },
                badge = tab == Tab.SOURCES && sourcesBusy,
                modifier = Modifier
                    .weight(1f)
                    .fillMaxHeight(),
            )
        }
    }
}

/** A destination: its icon in a pill that fills in when selected, and its label under it. */
@Composable
fun NavItem(
    icon: ImageVector,
    label: String,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    badge: Boolean = false,
) {
    val pill by animateColorAsState(
        if (selected) colors.secondaryContainer else Color.Transparent,
        NMotion.effectsDefault(),
        label = "pill",
    )
    val width by animateFloatAsState(if (selected) 1f else 0.6f, NMotion.spatialFast(), label = "pillWidth")
    Column(
        modifier.selectable(selected = selected, role = Role.Tab, onClick = onClick),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp, Alignment.CenterVertically),
    ) {
        Box(Modifier.size(56.dp, 32.dp), contentAlignment = Alignment.Center) {
            Box(
                Modifier
                    .size(56.dp, 32.dp)
                    .scale(scaleX = width, scaleY = 1f)
                    .background(pill, RoundedCornerShape(16.dp))
            )
            NIcon(icon, tint = if (selected) colors.onSecondaryContainer else colors.onSurfaceVariant)
            if (badge) {
                Box(
                    Modifier
                        .align(Alignment.TopEnd)
                        .offset(x = (-14).dp, y = 4.dp)
                        .size(8.dp)
                        .background(colors.primary, CircleShape)
                )
            }
        }
        Text(
            label,
            style = text(12, if (selected) FontWeight.Bold else FontWeight.SemiBold),
            color = if (selected) colors.onSurface else colors.onSurfaceVariant,
        )
    }
}
