package com.enn3developer.n_music.ui.sheets

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.enn3developer.n_music.R
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.SheetClose
import com.enn3developer.n_music.ui.components.SheetFrame
import com.enn3developer.n_music.ui.components.inSideSheet
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.dialogs.AppDialog
import com.enn3developer.n_music.ui.sources.addSource
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** The kinds of source to add: a folder on this phone or a web playlist, and those to come. */
@Composable
fun AddSourceSheet(open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val app = LocalApp.current
    AddSourceSheet(
        onFolder = {
            onDismissRequest()
            app.pickFolder { addSource(it) }
        },
        onWeb = {
            onDismissRequest()
            app.show(AppDialog.AddWebPlaylist())
        },
        open = open,
        onDismissRequest = onDismissRequest,
        onGone = onGone,
    )
}

/** The sheet itself; [onFolder] and [onWeb] add those kinds. */
@Composable
fun AddSourceSheet(
    onFolder: () -> Unit,
    onWeb: () -> Unit,
    open: Boolean,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
) {
    val title = stringResource(R.string.add_a_source)
    SheetFrame(
        open, title, onDismissRequest, onGone,
        header = {
            val side = inSideSheet
            Column(Modifier.padding(start = 24.dp, end = if (side) 12.dp else 24.dp, top = if (side) 0.dp else 4.dp, bottom = 12.dp)) {
                Row(
                    if (side) Modifier.height(64.dp) else Modifier,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(title, style = text(22, FontWeight.Bold), color = colors.onSurface, modifier = Modifier.weight(1f))
                    SheetClose()
                }
                Text(
                    stringResource(R.string.add_source_hint),
                    style = text(14, lineHeight = 20.sp),
                    color = colors.onSurfaceVariant,
                    modifier = Modifier.padding(top = if (side) 0.dp else 4.dp, end = if (side) 12.dp else 0.dp),
                )
            }
        },
    ) {
        SourceKind(NIcons.Sources, stringResource(R.string.local_folder), stringResource(R.string.local_folder_hint), onFolder)
        SourceKind(NIcons.Web, stringResource(R.string.web_playlist), stringResource(R.string.web_playlist_hint), onWeb)
        Box(
            Modifier
                .padding(horizontal = 24.dp, vertical = 8.dp)
                .fillMaxWidth()
                .height(1.dp)
                .background(colors.outlineVariant)
        )
        Later(stringResource(R.string.spotify), stringResource(R.string.spotify_hint))
        Later(stringResource(R.string.youtube), stringResource(R.string.youtube_hint))
        Later(stringResource(R.string.deezer), stringResource(R.string.deezer_hint))
    }
}

/** A kind of source the sheet adds: its icon, name, what it is, and the way on. */
@Composable
fun SourceKind(icon: ImageVector, name: String, detail: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    Row(
        modifier
            .fillMaxWidth()
            .height(72.dp)
            .tappable(onClick)
            .padding(start = 24.dp, end = 20.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Box(
            Modifier
                .size(48.dp)
                .background(colors.secondaryContainer, RoundedCornerShape(14.dp)),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(icon, tint = colors.onSecondaryContainer)
        }
        Column(Modifier.weight(1f)) {
            Text(name, style = text(16, FontWeight.Bold), color = colors.onSurface)
            Text(detail, style = text(14), color = colors.onSurfaceVariant, modifier = Modifier.padding(top = 2.dp))
        }
        NIcon(NIcons.Open, tint = colors.onSurfaceVariant)
    }
}

/** A kind of source N Music does not add yet, greyed, saying it comes later. */
@Composable
private fun Later(name: String, detail: String) {
    val quiet = colors.onSurfaceQuiet
    Row(
        Modifier
            .fillMaxWidth()
            .height(64.dp)
            .semantics { disabled() }
            .padding(horizontal = 24.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Box(
            Modifier
                .size(48.dp)
                .border(1.dp, colors.outlineVariant, RoundedCornerShape(14.dp)),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(NIcons.Later, tint = quiet)
        }
        Column(Modifier.weight(1f)) {
            Text(name, style = text(16, FontWeight.SemiBold), color = colors.onSurfaceVariant)
            Text(detail, style = text(14), color = quiet, modifier = Modifier.padding(top = 2.dp))
        }
        Text(stringResource(R.string.coming_later), style = text(12, FontWeight.SemiBold), color = quiet)
    }
}
