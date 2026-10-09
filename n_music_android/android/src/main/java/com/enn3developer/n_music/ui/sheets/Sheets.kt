package com.enn3developer.n_music.ui.sheets

import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.LibraryTab
import com.enn3developer.n_music.ui.library.FilterField
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** A sheet the app shows over its pages. */
sealed interface Sheet {
    /** How one of the library's lists is sorted. */
    data class Sort(val list: LibraryTab) : Sheet

    /** The filters on the library's tracks, opened at [focus]. */
    data class Filters(val focus: FilterField? = null) : Sheet
}

/**
 * The open sheet, and those still on their way out: each stays until it has left the screen.
 * [onDismiss] closes the open one.
 */
@Composable
fun SheetHost(current: Sheet?, onDismiss: () -> Unit) {
    val shown = remember { mutableStateListOf<Sheet>() }
    SideEffect {
        if (current != null && current !in shown) shown.add(current)
    }
    for (sheet in shown) {
        key(sheet) {
            val open = sheet == current
            val dismiss = { if (open) onDismiss() }
            val gone: () -> Unit = { shown.remove(sheet) }
            when (sheet) {
                is Sheet.Sort -> SortSheet(sheet.list, open, dismiss, gone)
                is Sheet.Filters -> FilterSheet(sheet.focus, open, dismiss, gone)
            }
        }
    }
}

/** A sheet's title, under its handle. */
@Composable
fun ColumnScope.SheetTitle(title: String) {
    Text(
        title,
        style = text(22, FontWeight.Bold),
        color = colors.onSurface,
        modifier = Modifier.padding(start = 24.dp, end = 24.dp, top = 4.dp, bottom = 12.dp),
    )
}
