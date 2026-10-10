package com.enn3developer.n_music.ui.dialogs

import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
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
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.webSource
import com.enn3developer.n_music.ui.LocalApp
import com.enn3developer.n_music.ui.Page
import com.enn3developer.n_music.ui.Snack
import com.enn3developer.n_music.ui.components.DialogCancel
import com.enn3developer.n_music.ui.components.DialogConfirm
import com.enn3developer.n_music.ui.components.DialogFrame
import com.enn3developer.n_music.ui.components.OutlinedField
import com.enn3developer.n_music.ui.formatCount
import com.enn3developer.n_music.ui.quantity
import com.enn3developer.n_music.ui.sources.WelcomeDraft
import com.enn3developer.n_music.ui.sources.addSource
import com.enn3developer.n_music.ui.sources.defaultName
import com.enn3developer.n_music.ui.sources.isLocal
import com.enn3developer.n_music.ui.sources.removeSource
import com.enn3developer.n_music.ui.sources.sourcePlace
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text

/** A dialog the app shows over everything. */
sealed interface AppDialog {
    /** Names a new playlist, which starts with [tracks]. */
    data class NewPlaylist(val tracks: List<Locator>) : AppDialog

    /** Gives playlist [id], now called [name], another name. */
    data class RenamePlaylist(val id: Long, val name: String) : AppDialog

    /** Asks before deleting playlist [id], called [name]. */
    data class DeletePlaylist(val id: Long, val name: String) : AppDialog

    /** Adds a web playlist to the library, or to the sources Welcome holds while [draft]. */
    data class AddWebPlaylist(val draft: Boolean = false) : AppDialog

    /** Gives source [root], now called [name] if it was named, another name. */
    data class RenameSource(val root: Locator, val name: String?) : AppDialog

    /** Asks before taking source [root], called [name], and its [tracks] out of the library. */
    data class RemoveSource(val root: Locator, val name: String, val tracks: UInt) : AppDialog
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
                is AppDialog.RenamePlaylist -> RenamePlaylistDialog(dialog.id, dialog.name, open, dismiss, gone)
                is AppDialog.DeletePlaylist -> DeletePlaylistDialog(dialog.id, dialog.name, open, dismiss, gone)
                is AppDialog.AddWebPlaylist -> AddWebPlaylistDialog(dialog.draft, open, dismiss, gone)
                is AppDialog.RenameSource -> RenameSourceDialog(dialog.root, dialog.name, open, dismiss, gone)
                is AppDialog.RemoveSource -> RemoveSourceDialog(dialog.root, dialog.name, dialog.tracks, open, dismiss, gone)
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
                        onAction = {
                            // The newest playlist of that name is the one this made.
                            CoreRepository.playlists.value.filter { it.name == chosen }.maxByOrNull { it.created }?.let {
                                CoreRepository.send(Command.DeletePlaylist(it.id))
                            }
                        },
                    )
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

/** A playlist's name field, filled with [name] to change. */
@Composable
private fun RenamePlaylistDialog(id: Long, name: String, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    var chosen by rememberSaveable { mutableStateOf(name) }
    val rename = {
        val trimmed = chosen.trim()
        if (trimmed.isNotEmpty()) {
            if (trimmed != name) CoreRepository.send(Command.RenamePlaylist(id, trimmed))
            onDismissRequest()
        }
    }
    DialogFrame(
        open,
        stringResource(R.string.rename_playlist),
        onDismissRequest,
        onGone,
        buttons = {
            DialogCancel(onDismissRequest)
            DialogConfirm(stringResource(R.string.rename), rename, enabled = chosen.isNotBlank())
        },
    ) {
        OutlinedField(
            chosen,
            { chosen = it },
            stringResource(R.string.name),
            Modifier.padding(top = 20.dp),
            focus = true,
            onDone = rename,
        )
    }
}

/**
 * Asks before deleting playlist [id]; deleting it leaves its page, if it is the one showing, and
 * says it is gone.
 */
@Composable
private fun DeletePlaylistDialog(id: Long, name: String, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val app = LocalApp.current
    val resources = LocalResources.current
    val delete = {
        CoreRepository.send(Command.DeletePlaylist(id))
        onDismissRequest()
        if (app.navigator.current.page == Page.Playlist(id)) app.back()
        app.snack(Snack(resources.getString(R.string.deleted_playlist, name)))
    }
    DialogFrame(
        open,
        stringResource(R.string.delete_playlist_title, name),
        onDismissRequest,
        onGone,
        buttons = {
            DialogCancel(onDismissRequest)
            DialogConfirm(stringResource(R.string.delete_playlist), delete, danger = true)
        },
    ) {
        Text(
            stringResource(R.string.delete_playlist_hint),
            style = text(14, lineHeight = 20.sp),
            color = colors.onSurfaceVariant,
            modifier = Modifier.padding(top = 16.dp),
        )
    }
}

/**
 * The address of an M3U or PLS playlist, and a name for it if wanted. https:// goes in front of
 * an address without a scheme; one that still isn't a web address says so.
 */
@Composable
private fun AddWebPlaylistDialog(draft: Boolean, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    var address by rememberSaveable { mutableStateOf("") }
    var name by rememberSaveable { mutableStateOf("") }
    var invalid by rememberSaveable { mutableStateOf(false) }
    val add = {
        val typed = address.trim()
        val root = webSource(if ("://" in typed) typed else "https://$typed")
        when {
            typed.isEmpty() -> {}
            root == null -> invalid = true
            else -> {
                if (draft) WelcomeDraft.add(root, name) else addSource(root, name)
                onDismissRequest()
            }
        }
    }
    DialogFrame(
        open,
        stringResource(R.string.add_web_playlist),
        onDismissRequest,
        onGone,
        buttons = {
            DialogCancel(onDismissRequest)
            DialogConfirm(stringResource(R.string.add), add, enabled = address.isNotBlank())
        },
    ) {
        Text(
            stringResource(R.string.add_web_playlist_hint),
            style = text(14, lineHeight = 20.sp),
            color = colors.onSurfaceVariant,
            modifier = Modifier.padding(top = 12.dp),
        )
        OutlinedField(
            address,
            {
                address = it
                invalid = false
            },
            stringResource(R.string.address),
            Modifier.padding(top = 20.dp),
            helper = stringResource(if (invalid) R.string.address_invalid else R.string.address_hint),
            error = invalid,
            focus = true,
            keyboardType = KeyboardType.Uri,
            imeAction = ImeAction.Next,
        )
        OutlinedField(
            name,
            { name = it },
            stringResource(R.string.display_name),
            Modifier.padding(top = 18.dp),
            helper = stringResource(R.string.optional),
            onDone = add,
        )
    }
}

/** A source's name field, filled with the name it was given; left empty, it goes by its own. */
@Composable
private fun RenameSourceDialog(root: Locator, name: String?, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    var chosen by rememberSaveable { mutableStateOf(name.orEmpty()) }
    val rename = {
        CoreRepository.send(Command.RenameLibrary(root, chosen.trim().ifEmpty { null }))
        onDismissRequest()
    }
    DialogFrame(
        open,
        stringResource(R.string.rename_source),
        onDismissRequest,
        onGone,
        buttons = {
            DialogCancel(onDismissRequest)
            DialogConfirm(stringResource(R.string.rename), rename)
        },
    ) {
        OutlinedField(
            chosen,
            { chosen = it },
            stringResource(R.string.name),
            Modifier.padding(top = 20.dp),
            helper = stringResource(R.string.rename_source_hint, defaultName(root)),
            focus = true,
            onDone = rename,
        )
    }
}

/**
 * Asks before taking source [root] out of the library: its tracks leave, its files stay.
 * Removing it leaves its page, if that is the one showing.
 */
@Composable
private fun RemoveSourceDialog(
    root: Locator,
    name: String,
    tracks: UInt,
    open: Boolean,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
) {
    val app = LocalApp.current
    val resources = LocalResources.current
    val remove = {
        removeSource(root)
        app.releaseFolder(root)
        onDismissRequest()
        if (app.navigator.current.page == Page.Source(root)) app.back()
        app.snack(Snack(resources.getString(R.string.removed_source, name)))
    }
    val count = tracks.toLong()
    DialogFrame(
        open,
        stringResource(R.string.remove_source_title, name),
        onDismissRequest,
        onGone,
        buttons = {
            DialogCancel(onDismissRequest)
            DialogConfirm(stringResource(R.string.remove), remove, danger = true)
        },
    ) {
        Text(
            pluralStringResource(
                if (root.isLocal) R.plurals.remove_source_local else R.plurals.remove_source_web,
                quantity(count),
                formatCount(count),
                sourcePlace(root),
            ),
            style = text(14, lineHeight = 20.sp),
            color = colors.onSurfaceVariant,
            modifier = Modifier.padding(top = 16.dp),
        )
    }
}
