package com.enn3developer.n_music.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** A page's ⋮ menu, dropping from its button while [expanded]. */
@Composable
fun NMenu(expanded: Boolean, onDismissRequest: () -> Unit, content: @Composable ColumnScope.() -> Unit) {
    DropdownMenu(
        expanded = expanded,
        onDismissRequest = onDismissRequest,
        modifier = Modifier.width(220.dp),
        offset = DpOffset((-8).dp, 0.dp),
        shape = RoundedCornerShape(16.dp),
        containerColor = colors.surfaceHigh,
        tonalElevation = 0.dp,
        shadowElevation = 6.dp,
        content = content,
    )
}

/**
 * An item of a menu: its icon and label; [danger] for one that deletes or removes. Without an
 * icon, [checked] puts a check in its place on the one picked.
 */
@Composable
fun MenuItem(label: String, icon: ImageVector?, onClick: () -> Unit, danger: Boolean = false, checked: Boolean = false) {
    Row(
        Modifier
            .fillMaxWidth()
            .height(48.dp)
            .clickable(role = Role.Button, onClick = onClick)
            .padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        when {
            icon != null -> NIcon(icon, size = 20.dp, tint = if (danger) colors.error else colors.onSurfaceVariant)
            checked -> NIcon(NIcons.Check, size = 20.dp, tint = colors.primary)
            else -> Box(Modifier.size(20.dp))
        }
        Text(
            label,
            style = text(15, if (danger) FontWeight.SemiBold else FontWeight.Medium),
            color = if (danger) colors.error else colors.onSurface,
            maxLines = 1,
        )
    }
}

/** The line between groups of a menu's items. */
@Composable
fun MenuDivider() {
    Box(
        Modifier
            .padding(vertical = 6.dp)
            .fillMaxWidth()
            .height(1.dp)
            .background(colors.outlineVariant)
    )
}
