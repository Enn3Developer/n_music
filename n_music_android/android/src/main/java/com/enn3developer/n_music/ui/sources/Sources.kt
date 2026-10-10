package com.enn3developer.n_music.ui.sources

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.core.net.toUri
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.SourceRow
import com.enn3developer.n_music.core.TelegramStatus
import com.enn3developer.n_music.core.defaultSourceName
import com.enn3developer.n_music.ui.theme.NIcons

/** A source on this phone, rather than one streamed from the web or Telegram. */
val Locator.isLocal: Boolean
    get() = this is Locator.Local || this is Locator.DocumentTree

/** What kind of source [root] is, as an icon: a folder, a link or Telegram's plane. */
fun sourceIcon(root: Locator): ImageVector = when {
    root.isLocal -> NIcons.Sources
    root is Locator.TelegramChat -> NIcons.Telegram
    else -> NIcons.Web
}

/** What kind of source [root] is, in words. */
fun sourceKind(root: Locator): Int = when {
    root.isLocal -> R.string.local_folder
    root is Locator.TelegramChat -> R.string.telegram_chat
    else -> R.string.web_playlist
}

/**
 * [root] is a Telegram chat, and can't be read for want of signing in to Telegram, which is at
 * [telegram]: `null` without Telegram at all.
 */
fun signedOut(root: Locator, telegram: TelegramStatus?): Boolean =
    root is Locator.TelegramChat && telegram != null && telegram !is TelegramStatus.SignedIn

/** What a source is called: the name it was given, or its folder's or playlist's. */
val SourceRow.title: String
    get() = name ?: defaultName(root)

/**
 * What a source goes by without a name of its own: its folder's name, as the core has it, or
 * what the core calls a playlist.
 */
fun defaultName(root: Locator): String = when (root) {
    is Locator.DocumentTree -> treeParts(root.v1)?.let { (volume, folders) -> folders.lastOrNull() ?: volume }
    is Locator.Local -> root.v1.trimEnd('/').substringAfterLast('/').ifEmpty { null }
    else -> null
} ?: defaultSourceName(root)

/**
 * Where a source is, the way people know it: Internal storage › Music, SD card › Rips, a web
 * address without its scheme, or Telegram.
 */
@Composable
fun sourcePlace(root: Locator): String {
    val parts = when (root) {
        is Locator.DocumentTree -> treeParts(root.v1)
        is Locator.Local -> localParts(root.v1)
        is Locator.Web -> return webAddress(root.v1)
        is Locator.TelegramChat -> return stringResource(R.string.telegram)
        else -> return defaultSourceName(root)
    } ?: return defaultSourceName(root)
    val (volume, folders) = parts
    val first = when (volume) {
        null -> null
        PRIMARY -> stringResource(R.string.internal_storage)
        else -> stringResource(R.string.sd_card)
    }
    return (listOfNotNull(first) + folders).joinToString(" › ")
}

/** A web address without its scheme or a trailing slash: mixes.example.net/live.pls. */
fun webAddress(url: String): String = url.substringAfter("://").trimEnd('/')

/** The server a web source is on, for when it does not answer. */
fun webHost(url: String): String = url.toUri().host ?: webAddress(url)

private const val PRIMARY = "primary"

/** The volume and folders of a Storage Access Framework tree, as `primary:Music/Jazz` tells. */
private fun treeParts(uri: String): Pair<String?, List<String>>? {
    val encoded = uri.substringAfter("/tree/", "").substringBefore('/').ifEmpty { return null }
    val id = Uri.decode(encoded)
    val volume = id.substringBefore(':', "")
    val path = id.substringAfter(':')
    return volume.ifEmpty { null } to path.split('/').filter(String::isNotEmpty)
}

/** The volume and folders of a path under /storage, or just its folders elsewhere. */
private fun localParts(path: String): Pair<String?, List<String>> {
    val segments = path.split('/').filter(String::isNotEmpty)
    return when {
        segments.take(3) == listOf("storage", "emulated", "0") -> PRIMARY to segments.drop(3)
        segments.firstOrNull() == "sdcard" -> PRIMARY to segments.drop(1)
        segments.firstOrNull() == "storage" && segments.size >= 2 -> segments[1] to segments.drop(2)
        else -> null to segments
    }
}

/** Adds [root] to the library, which reads it, and names it [name] when there is one. */
fun addSource(root: Locator, name: String? = null) {
    CoreRepository.editRoots({ roots -> if (root in roots) roots else roots + root }) {
        if (!name.isNullOrBlank()) CoreRepository.send(Command.RenameLibrary(root, name.trim()))
    }
}

/** Takes [root] and its tracks out of the library; its files stay where they are. */
fun removeSource(root: Locator) {
    CoreRepository.editRoots({ roots -> roots - root })
}

/** A source picked on Welcome, which the library gets with Build my library. */
data class DraftSource(val root: Locator, val name: String? = null)

/** The sources picked on Welcome so far: they outlive the screen turning, not the process. */
object WelcomeDraft {
    val sources = mutableStateListOf<DraftSource>()

    fun add(root: Locator, name: String? = null) {
        if (sources.none { it.root == root }) sources += DraftSource(root, name?.trim()?.ifEmpty { null })
    }

    fun remove(root: Locator) {
        sources.removeAll { it.root == root }
    }
}
