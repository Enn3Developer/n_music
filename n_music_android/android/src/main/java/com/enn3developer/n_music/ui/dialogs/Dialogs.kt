package com.enn3developer.n_music.ui.dialogs

import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.Snack
import com.enn3developer.n_music.ui.components.DialogCancel
import com.enn3developer.n_music.ui.components.DialogConfirm
import com.enn3developer.n_music.ui.components.DialogFrame
import com.enn3developer.n_music.ui.components.OutlinedField
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.quantity

/** A dialog the app shows over everything. */
sealed interface AppDialog {
    /** Names a new playlist, which starts with [tracks]. */
    data class NewPlaylist(val tracks: List<Locator>) : AppDialog
}

/** The open dialog, and one still fading out. [onDismiss] closes the open one. */
@Composable
fun DialogHost(current: AppDialog?, onDismiss: () -> Unit) {
    val shown = remember { mutableStateListOf<AppDialog>() }
    SideEffect {
        if (current != null && current !in shown) shown.add(current)
    }
    for (dialog in shown) {
        key(dialog) {
            val open = dialog == current
            val dismiss = { if (open) onDismiss() }
            val gone: () -> Unit = { shown.remove(dialog) }
            when (dialog) {
                is AppDialog.NewPlaylist -> NewPlaylistDialog(dialog.tracks, open, dismiss, gone)
            }
        }
    }
}

/** Names a new playlist holding [tracks]; Create makes it, and Undo deletes it again. */
@Composable
private fun NewPlaylistDialog(tracks: List<Locator>, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val app = LocalApp.current
    val resources = LocalResources.current
    var name by rememberSaveable { mutableStateOf("") }
    val create = {
        val chosen = name.trim()
        if (chosen.isNotEmpty()) {
            CoreRepository.send(Command.CreatePlaylist(chosen, null, emptyList(), tracks))
            onDismissRequest()
            if (tracks.isNotEmpty()) {
                val count = tracks.size
                app.snack(
                    Snack(
                        resources.getQuantityString(R.plurals.added_to_playlist, quantity(count), formatCount(count), chosen),
                        resources.getString(R.string.undo),
                    ) {
                        // The newest playlist of that name is the one this made.
                        CoreRepository.playlists.value.filter { it.name == chosen }.maxByOrNull { it.created }?.let {
                            CoreRepository.send(Command.DeletePlaylist(it.id))
                        }
                    }
                )
            }
        }
    }
    DialogFrame(
        open,
        stringResource(R.string.new_playlist),
        onDismissRequest,
        onGone,
        buttons = {
            DialogCancel(onDismissRequest)
            DialogConfirm(stringResource(R.string.create), create, enabled = name.isNotBlank())
        },
    ) {
        OutlinedField(
            name,
            { name = it },
            stringResource(R.string.playlist_name),
            Modifier.padding(top = 20.dp),
            focus = true,
            onDone = create,
        )
    }
}
