package com.enn3developer.n_music.ui.settings

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.components.PageBar
import com.enn3developer.n_music.ui.theme.NType
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** The GPL that N Music comes under, and its typeface's licence, in full. */
@Composable
fun LicencePage() {
    val app = LocalApp.current
    val resources = LocalResources.current
    // A paragraph to a row, so the long text scrolls without laying it all out at once.
    val gpl = remember(resources) { paragraphs(resources.openRawResource(R.raw.gpl3).bufferedReader().readText()) }
    val font = remember(resources) { paragraphs(resources.openRawResource(R.raw.figtree_ofl).bufferedReader().readText()) }
    Column(Modifier.fillMaxSize()) {
        PageBar(app::back)
        LazyColumn(
            Modifier.fillMaxSize(),
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, bottom = 24.dp),
        ) {
            item(key = "title") {
                Text(
                    stringResource(R.string.licence),
                    style = NType.headline,
                    color = colors.onSurface,
                    modifier = Modifier
                        .padding(top = 4.dp)
                        .semantics { heading() },
                )
            }
            item(key = "intro") {
                Text(
                    stringResource(R.string.licence_intro),
                    style = text(15, lineHeight = 22.sp),
                    color = colors.onSurfaceVariant,
                    modifier = Modifier.padding(top = 12.dp, bottom = 8.dp),
                )
            }
            items(gpl) { Paragraph(it) }
            item(key = "font") {
                Text(
                    stringResource(R.string.licence_font),
                    style = text(16, FontWeight.Bold),
                    color = colors.onSurface,
                    modifier = Modifier
                        .padding(top = 24.dp, bottom = 4.dp)
                        .semantics { heading() },
                )
            }
            items(font) { Paragraph(it) }
            item(key = "end") { Column(Modifier.windowInsetsPadding(WindowInsets.navigationBars)) {} }
        }
    }
}

@Composable
private fun Paragraph(paragraph: String) {
    Text(
        paragraph,
        style = text(13, lineHeight = 19.sp),
        color = colors.onSurfaceVariant,
        modifier = Modifier.padding(top = 10.dp),
    )
}

/** [text]'s paragraphs, each one's lines joined back into one. */
private fun paragraphs(text: String): List<String> =
    text.split(Regex("\\n\\s*\\n"))
        .map { paragraph -> paragraph.lines().joinToString(" ") { it.trim() }.trim() }
        .filter(String::isNotEmpty)
