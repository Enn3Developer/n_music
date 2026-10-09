package com.enn3developer.n_music.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Text
import androidx.compose.material3.ripple
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ViewMode
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.NShapes
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** A button's surface: its shape, fill and ink, with a ripple and press state. */
@Composable
fun ButtonSurface(
    onClick: () -> Unit,
    shape: Shape,
    container: Color,
    content: Color,
    modifier: Modifier = Modifier,
    border: BorderStroke? = null,
    enabled: Boolean = true,
    role: Role = Role.Button,
    interactionSource: MutableInteractionSource = remember { MutableInteractionSource() },
    padding: PaddingValues = PaddingValues(0.dp),
    arrangement: Arrangement.Horizontal = Arrangement.Center,
    body: @Composable RowScope.() -> Unit,
) {
    Row(
        modifier
            .clip(shape)
            .background(container, shape)
            .then(if (border != null) Modifier.border(border, shape) else Modifier)
            .clickable(
                enabled = enabled,
                role = role,
                interactionSource = interactionSource,
                indication = ripple(color = content),
                onClick = onClick,
            )
            .padding(padding),
        horizontalArrangement = arrangement,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CompositionLocalProvider(LocalContentColor provides content) { body() }
    }
}

/**
 * Play and Shuffle side by side, joined: [large] for a page's header, small beside the sort
 * control where Shuffle shows only its icon.
 */
@Composable
fun PlayShuffle(
    onPlay: () -> Unit,
    onShuffle: () -> Unit,
    modifier: Modifier = Modifier,
    large: Boolean = true,
    playLabel: String = stringResource(R.string.play),
    playDescription: String? = null,
    shuffleDescription: String? = null,
) {
    if (large) {
        Row(modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            ButtonSurface(
                onClick = onPlay,
                shape = NShapes.first(24.dp, 8.dp),
                container = colors.primaryContainer,
                content = colors.onPrimaryContainer,
                modifier = Modifier
                    .weight(1f)
                    .height(48.dp)
                    .described(playDescription),
            ) {
                NIcon(NIcons.Play, size = 20.dp)
                Box(Modifier.width(8.dp))
                Text(playLabel, style = text(15, FontWeight.Bold))
            }
            ButtonSurface(
                onClick = onShuffle,
                shape = NShapes.last(24.dp, 8.dp),
                container = colors.secondaryContainer,
                content = colors.onSecondaryContainer,
                modifier = Modifier
                    .weight(1f)
                    .height(48.dp)
                    .described(shuffleDescription),
            ) {
                NIcon(NIcons.Shuffle, size = 20.dp)
                Box(Modifier.width(8.dp))
                Text(stringResource(R.string.shuffle), style = text(15, FontWeight.Bold))
            }
        }
    } else {
        Row(modifier, horizontalArrangement = Arrangement.spacedBy(2.dp)) {
            ButtonSurface(
                onClick = onPlay,
                shape = NShapes.first(20.dp, 6.dp),
                container = colors.primaryContainer,
                content = colors.onPrimaryContainer,
                modifier = Modifier
                    .height(40.dp)
                    .described(playDescription),
                padding = PaddingValues(start = 12.dp, end = 16.dp),
            ) {
                NIcon(NIcons.Play, size = 18.dp)
                Box(Modifier.width(6.dp))
                Text(playLabel, style = text(14, FontWeight.Bold, tabular = true), maxLines = 1)
            }
            ButtonSurface(
                onClick = onShuffle,
                shape = NShapes.last(20.dp, 6.dp),
                container = colors.secondaryContainer,
                content = colors.onSecondaryContainer,
                modifier = Modifier
                    .size(48.dp, 40.dp)
                    .described(shuffleDescription ?: stringResource(R.string.shuffle)),
            ) {
                NIcon(NIcons.Shuffle, size = 20.dp)
            }
        }
    }
}

private fun Modifier.described(description: String?) =
    if (description == null) this else semantics { contentDescription = description }

/** The sort control: what the list is sorted by, opening the sort sheet. */
@Composable
fun SortControl(label: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    ButtonSurface(
        onClick = onClick,
        shape = RoundedCornerShape(20.dp),
        container = Color.Transparent,
        content = colors.onSurface,
        modifier = modifier.height(40.dp),
        padding = PaddingValues(start = 10.dp, end = 8.dp),
    ) {
        NIcon(NIcons.Sort, size = 18.dp, tint = colors.primary)
        Box(Modifier.width(6.dp))
        Text(
            label,
            style = text(14, FontWeight.Bold),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f, fill = false),
        )
        Box(Modifier.width(6.dp))
        NIcon(NIcons.Collapse, size = 18.dp, tint = colors.onSurfaceVariant)
    }
}

/**
 * The switch between a list and a grid: both views, the current one filled, and a tap
 * anywhere on it changes to the other. The thumb slides over as the icons swap colour.
 */
@Composable
fun ViewSwitch(mode: ViewMode, onToggle: () -> Unit, modifier: Modifier = Modifier) {
    val grid = mode == ViewMode.GRID
    val thumb by animateDpAsState(if (grid) 26.dp else 0.dp, NMotion.spatialFast(), label = "thumb")
    val listInk by animateColorAsState(
        if (grid) colors.onSurfaceVariant else colors.onPrimary,
        NMotion.effectsFast(),
        label = "list",
    )
    val gridInk by animateColorAsState(
        if (grid) colors.onPrimary else colors.onSurfaceVariant,
        NMotion.effectsFast(),
        label = "grid",
    )
    val description = stringResource(if (grid) R.string.grid_view else R.string.list_view)
    Box(
        modifier
            .size(58.dp, 36.dp)
            .clip(RoundedCornerShape(18.dp))
            .background(colors.surfaceHigh)
            .clickable(role = Role.Button, onClick = onToggle)
            .semantics { contentDescription = description }
            .padding(3.dp),
    ) {
        Box(
            Modifier
                .offset { IntOffset(thumb.roundToPx(), 0) }
                .size(26.dp, 30.dp)
                .background(colors.primary, RoundedCornerShape(15.dp))
        )
        Row(Modifier.fillMaxHeight()) {
            Box(Modifier.size(26.dp, 30.dp), contentAlignment = Alignment.Center) {
                NIcon(NIcons.ListView, size = 18.dp, tint = listInk)
            }
            Box(Modifier.size(26.dp, 30.dp), contentAlignment = Alignment.Center) {
                NIcon(NIcons.GridView, size = 18.dp, tint = gridInk)
            }
        }
    }
}

/** A text button in the primary colour: Clear all, See all, Undo. */
@Composable
fun TextAction(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    color: Color = colors.primary,
    height: Dp = 40.dp,
    textStyle: TextStyle = text(14, FontWeight.Bold),
    padding: PaddingValues = PaddingValues(horizontal = 12.dp),
    trailing: ImageVector? = null,
) {
    ButtonSurface(
        onClick = onClick,
        shape = RoundedCornerShape(height / 2),
        container = Color.Transparent,
        content = color,
        modifier = modifier.height(height),
        padding = padding,
    ) {
        Text(text, style = textStyle, maxLines = 1)
        if (trailing != null) NIcon(trailing, size = 18.dp, modifier = Modifier.padding(start = 2.dp))
    }
}

/**
 * A chip: outlined, or filled in the secondary container once [selected], when a check comes
 * in front of its label if [check] says so. [trailing] adds the chevron of a chip that opens a
 * picker.
 */
@Composable
fun NChip(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    selected: Boolean = false,
    check: Boolean = true,
    leading: ImageVector? = null,
    leadingTint: Color? = null,
    trailing: ImageVector? = null,
    bold: Boolean = false,
    role: Role = Role.Checkbox,
) {
    val container by animateColorAsState(
        if (selected) colors.secondaryContainer else Color.Transparent,
        NMotion.effectsFast(),
        label = "chip",
    )
    val ink = if (selected) colors.onSecondaryContainer else colors.onSurface
    val icon = leading ?: if (selected && check) NIcons.Check else null
    ButtonSurface(
        onClick = onClick,
        shape = NShapes.chip,
        container = container,
        content = ink,
        border = if (selected) null else BorderStroke(1.dp, colors.outlineVariant),
        modifier = modifier
            .height(32.dp)
            .semantics {
                this.role = role
                this.selected = selected
            },
        padding = PaddingValues(
            start = if (icon != null) 8.dp else 12.dp,
            end = if (trailing != null) 6.dp else 12.dp,
        ),
    ) {
        if (icon != null) {
            NIcon(icon, size = 18.dp, tint = leadingTint ?: ink)
            Box(Modifier.width(6.dp))
        }
        Text(
            label,
            style = text(14, if (bold || (selected && leading == null && trailing != null)) FontWeight.Bold else FontWeight.SemiBold),
            maxLines = 1,
        )
        if (trailing != null) {
            Box(Modifier.width(2.dp))
            NIcon(trailing, size = 18.dp, tint = if (selected) ink else colors.onSurfaceVariant)
        }
    }
}

/** An active filter: its label opens its picker, the cross clears it. */
@Composable
fun ActiveChip(
    label: String,
    onOpen: () -> Unit,
    onClear: () -> Unit,
    openDescription: String,
    clearDescription: String,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier
            .height(32.dp)
            .clip(NShapes.chip)
            .background(colors.secondaryContainer),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CompositionLocalProvider(LocalContentColor provides colors.onSecondaryContainer) {
            Box(
                Modifier
                    .fillMaxHeight()
                    .clickable(role = Role.Button, onClick = onOpen)
                    .semantics { contentDescription = openDescription }
                    .padding(start = 12.dp, end = 2.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(label, style = text(14, FontWeight.Bold), maxLines = 1)
            }
            NIconButton(
                NIcons.Close,
                clearDescription,
                onClear,
                size = 32.dp,
                iconSize = 16.dp,
            )
        }
    }
}

/** How a [PillButton] fills. */
enum class PillStyle { FILLED, TONAL, OUTLINED, TEXT, ERROR }

/**
 * A fully rounded button: filled in the primary container, tonal in the secondary one, outlined
 * or text only.
 */
@Composable
fun PillButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    style: PillStyle = PillStyle.FILLED,
    height: Dp = 48.dp,
    icon: ImageVector? = null,
    iconSize: Dp = 18.dp,
    textStyle: TextStyle = text(15, FontWeight.Bold),
    padding: PaddingValues = PaddingValues(horizontal = 24.dp),
    enabled: Boolean = true,
    outline: Color = colors.outline,
) {
    val (container, ink) = when (style) {
        PillStyle.FILLED -> colors.primaryContainer to colors.onPrimaryContainer
        PillStyle.TONAL -> colors.secondaryContainer to colors.onSecondaryContainer
        PillStyle.OUTLINED -> Color.Transparent to colors.onSurface
        PillStyle.TEXT -> Color.Transparent to colors.primary
        PillStyle.ERROR -> colors.error to colors.background
    }
    ButtonSurface(
        onClick = onClick,
        shape = RoundedCornerShape(height / 2),
        container = container,
        content = ink,
        modifier = modifier.height(height),
        border = if (style == PillStyle.OUTLINED) BorderStroke(1.dp, outline) else null,
        enabled = enabled,
        padding = padding,
    ) {
        if (icon != null) {
            NIcon(icon, size = iconSize)
            Box(Modifier.width(8.dp))
        }
        Text(text, style = textStyle, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

/**
 * Buttons sharing a row, each widening by 15% while pressed as the others make room, as in
 * Material's button groups.
 */
@Composable
fun PressGroup(
    count: Int,
    modifier: Modifier = Modifier,
    gap: Dp = 8.dp,
    item: @Composable RowScope.(index: Int, interaction: MutableInteractionSource, weight: Modifier) -> Unit,
) {
    val sources = remember(count) { List(count) { MutableInteractionSource() } }
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(gap)) {
        for (index in 0 until count) {
            val pressed by sources[index].collectIsPressedAsState()
            val weight by animateFloatAsState(if (pressed) 1.15f else 1f, NMotion.spatialFast(), label = "press")
            item(index, sources[index], Modifier.weight(weight))
        }
    }
}

/**
 * Connected buttons picking one of [options], the picked one filled with a check: the end of the
 * queue, the theme, ReplayGain. [weights] widen the options with longer labels.
 */
@Composable
fun <T> Segmented(
    options: List<T>,
    selected: T,
    label: @Composable (T) -> String,
    onSelect: (T) -> Unit,
    modifier: Modifier = Modifier,
    height: Dp = 40.dp,
    weights: List<Float> = options.map { 1f },
    textSize: Int = 14,
    checkSize: Dp = 18.dp,
) {
    Row(modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(2.dp)) {
        options.forEachIndexed { index, option ->
            val picked = option == selected
            val outer = height / 2
            val shape = when (index) {
                0 -> NShapes.first(outer, 6.dp)
                options.lastIndex -> NShapes.last(outer, 6.dp)
                else -> NShapes.middle(6.dp)
            }
            val container by animateColorAsState(
                if (picked) colors.secondaryContainer else colors.surfaceHigh,
                NMotion.effectsFast(),
                label = "segment",
            )
            val text = label(option)
            ButtonSurface(
                onClick = { onSelect(option) },
                shape = shape,
                container = container,
                content = if (picked) colors.onSecondaryContainer else colors.onSurface,
                role = Role.RadioButton,
                modifier = Modifier
                    .weight(weights[index])
                    .height(height)
                    .semantics { this.selected = picked },
            ) {
                if (picked) {
                    NIcon(NIcons.Check, size = checkSize)
                    Box(Modifier.width(if (checkSize < 18.dp) 4.dp else 6.dp))
                }
                Text(
                    text,
                    style = text(textSize, if (picked) FontWeight.Bold else FontWeight.SemiBold),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}
