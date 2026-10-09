package com.enn3developer.n_music.ui.library

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.animation.core.VisibilityThreshold
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.ui.components.ActiveChip
import com.enn3developer.n_music.ui.components.ButtonSurface
import com.enn3developer.n_music.ui.components.NChip
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.NMotion
import com.enn3developer.n_music.ui.theme.NShapes
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/**
 * The tracks' filter chips: Filters, which opens the filter sheet, then a chip for each part that
 * is set, then one for each that is not. A chip opens the sheet at its part; the cross on a set
 * one clears it.
 */
@Composable
fun FilterRow(
    filters: TrackFilters,
    sourceName: (Locator) -> String,
    onOpen: (FilterField?) -> Unit,
    onClear: (FilterField) -> Unit,
    modifier: Modifier = Modifier,
) {
    val active = filters.active
    LazyRow(
        modifier.fillMaxWidth(),
        contentPadding = PaddingValues(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        item(key = "filters") {
            if (active.isEmpty()) {
                NChip(
                    stringResource(R.string.filters),
                    { onOpen(null) },
                    leading = NIcons.Filter,
                    leadingTint = colors.primary,
                    role = Role.Button,
                )
            } else {
                val description = pluralStringResource(R.plurals.filters_active, active.size, active.size)
                ButtonSurface(
                    onClick = { onOpen(null) },
                    shape = NShapes.chip,
                    container = colors.secondaryContainer,
                    content = colors.onSecondaryContainer,
                    modifier = Modifier
                        .height(32.dp)
                        .semantics { contentDescription = description },
                    padding = PaddingValues(start = 8.dp, end = 10.dp),
                    arrangement = Arrangement.spacedBy(4.dp),
                ) {
                    NIcon(NIcons.Filter, size = 18.dp)
                    BasicText(
                        active.size.toString(),
                        style = text(14, FontWeight.Bold, tabular = true).copy(color = colors.onSecondaryContainer),
                    )
                }
            }
        }
        items(active, key = { "set " + it.name }) { field ->
            val named = stringResource(field.named)
            ActiveChip(
                filters.label(field, sourceName),
                onOpen = { onOpen(field) },
                onClear = { onClear(field) },
                openDescription = stringResource(R.string.change_filter, named),
                clearDescription = stringResource(R.string.clear_filter, named),
                modifier = Modifier.animateItem(
                    fadeInSpec = NMotion.effectsDefault(),
                    placementSpec = NMotion.spatialDefault(IntOffset.VisibilityThreshold),
                    fadeOutSpec = NMotion.effectsFast(),
                ),
            )
        }
        items(FilterField.entries - active.toSet(), key = { "unset " + it.name }) { field ->
            NChip(
                stringResource(field.label),
                { onOpen(field) },
                trailing = NIcons.Collapse,
                role = Role.Button,
                modifier = Modifier.animateItem(
                    fadeInSpec = NMotion.effectsDefault(),
                    placementSpec = NMotion.spatialDefault(IntOffset.VisibilityThreshold),
                    fadeOutSpec = NMotion.effectsFast(),
                ),
            )
        }
    }
}
