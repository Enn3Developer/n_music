package com.enn3developer.n_music.ui.dialogs

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.layout
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.enn3developer.n_music.CoreRepository
import com.enn3developer.n_music.R
import com.enn3developer.n_music.TelegramChats
import com.enn3developer.n_music.TelegramState
import com.enn3developer.n_music.core.Command
import com.enn3developer.n_music.core.Locator
import com.enn3developer.n_music.core.TelegramChatInfo
import com.enn3developer.n_music.core.TelegramChatKind
import com.enn3developer.n_music.core.TelegramError
import com.enn3developer.n_music.core.TelegramStatus
import com.enn3developer.n_music.ui.components.DialogCancel
import com.enn3developer.n_music.ui.components.DialogConfirm
import com.enn3developer.n_music.ui.components.DialogFrame
import com.enn3developer.n_music.ui.components.NIcon
import com.enn3developer.n_music.ui.components.OutlinedField
import com.enn3developer.n_music.ui.components.SearchField
import com.enn3developer.n_music.ui.components.TextAction
import com.enn3developer.n_music.ui.components.tappable
import com.enn3developer.n_music.ui.dotted
import com.enn3developer.n_music.ui.sources.WelcomeDraft
import com.enn3developer.n_music.ui.sources.addSource
import com.enn3developer.n_music.ui.theme.NIcons
import com.enn3developer.n_music.ui.theme.colors
import com.enn3developer.n_music.ui.theme.text
import kotlinx.coroutines.delay

/** What the Telegram dialog asks of the core. */
interface TelegramActions {
    fun signIn(phone: String)

    fun code(code: String)

    fun password(password: String)

    /** Stops signing in, to start again with another number. */
    fun signOut()

    fun find(query: String)

    /** Adds [chat], called [name], as a source. */
    fun pick(chat: Locator, name: String)
}

/**
 * Signs in to Telegram a step at a time, as the core asks, then picks a chat to add as a source,
 * or to the sources Welcome holds while [draft]. With [signInOnly], it closes once signed in.
 */
@Composable
fun TelegramDialog(draft: Boolean, signInOnly: Boolean, open: Boolean, onDismissRequest: () -> Unit, onGone: () -> Unit) {
    val state by CoreRepository.telegram.collectAsStateWithLifecycle()
    val chats by CoreRepository.telegramChats.collectAsStateWithLifecycle()
    val roots by CoreRepository.roots.collectAsStateWithLifecycle()
    val taken = if (draft) WelcomeDraft.sources.map { it.root }.toSet() else roots.orEmpty().toSet()
    val actions = remember(draft) {
        object : TelegramActions {
            override fun signIn(phone: String) = CoreRepository.send(Command.TelegramSignIn(phone))

            override fun code(code: String) = CoreRepository.send(Command.TelegramCode(code))

            override fun password(password: String) = CoreRepository.send(Command.TelegramPassword(password))

            override fun signOut() = CoreRepository.send(Command.TelegramSignOut)

            override fun find(query: String) = CoreRepository.send(Command.FindTelegramChats(query))

            override fun pick(chat: Locator, name: String) = if (draft) WelcomeDraft.add(chat, name) else addSource(chat, name)
        }
    }
    val shown = state
    if (shown == null) {
        // No Telegram to sign in to: the dialog has nothing to show.
        LaunchedEffect(Unit) {
            onDismissRequest()
            onGone()
        }
        return
    }
    TelegramDialog(shown, chats, taken, signInOnly, actions, open, onDismissRequest, onGone)
}

/**
 * The dialog itself, at the step [state] is at: the phone number, the code Telegram sent, the
 * account's password, then [chats] to pick from, those [taken] already added. An error shows
 * only for a step the dialog asked for, until something is typed again.
 */
@Composable
fun TelegramDialog(
    state: TelegramState,
    chats: TelegramChats?,
    taken: Set<Locator>,
    signInOnly: Boolean,
    actions: TelegramActions,
    open: Boolean,
    onDismissRequest: () -> Unit,
    onGone: () -> Unit,
) {
    val status = state.status
    val picking = status is TelegramStatus.SignedIn && !signInOnly
    if (signInOnly && status is TelegramStatus.SignedIn) {
        LaunchedEffect(Unit) { onDismissRequest() }
    }
    // Where the steps the dialog asked for had got to when it asked: their errors show after.
    var askedAt by rememberSaveable { mutableStateOf<Int?>(null) }
    val asked = askedAt
    val error = state.error.takeIf { !state.busy && asked != null && state.ended > asked }
    var typed by rememberSaveable(status::class.simpleName) { mutableStateOf("") }
    val edit = { value: String ->
        typed = value
        askedAt = null
    }
    val ready = typed.isNotBlank() && !state.busy
    val confirm = {
        if (ready) {
            askedAt = state.ended
            when (status) {
                TelegramStatus.SignedOut -> actions.signIn(typed.trim())
                is TelegramStatus.CodeSent -> actions.code(typed.trim())
                // A password is sent as typed: spaces count.
                is TelegramStatus.PasswordNeeded -> actions.password(typed)
                is TelegramStatus.SignedIn -> {}
            }
        }
    }
    DialogFrame(
        open,
        stringResource(if (picking) R.string.add_telegram_chat else R.string.telegram_sign_in),
        onDismissRequest,
        onGone,
        buttons = {
            DialogCancel(onDismissRequest)
            if (status !is TelegramStatus.SignedIn) {
                val label = when {
                    state.busy -> R.string.telegram_waiting
                    status == TelegramStatus.SignedOut -> R.string.telegram_send_code
                    else -> R.string.sign_in
                }
                DialogConfirm(stringResource(label), confirm, enabled = ready)
            }
        },
    ) {
        val problem = error?.let { telegramError(it) }
        when (status) {
            TelegramStatus.SignedOut -> {
                Message(stringResource(R.string.telegram_phone_message))
                OutlinedField(
                    typed,
                    edit,
                    stringResource(R.string.phone_number),
                    Modifier.padding(top = 20.dp),
                    helper = problem ?: stringResource(R.string.telegram_phone_example),
                    error = problem != null,
                    focus = true,
                    keyboardType = KeyboardType.Phone,
                    onDone = confirm,
                )
            }
            is TelegramStatus.CodeSent -> {
                Message(stringResource(R.string.telegram_code_message, "+" + status.phone))
                OutlinedField(
                    typed,
                    edit,
                    stringResource(R.string.code),
                    Modifier.padding(top = 20.dp),
                    helper = problem,
                    error = problem != null,
                    focus = true,
                    keyboardType = KeyboardType.Number,
                    onDone = confirm,
                )
                OtherNumber(actions::signOut, enabled = !state.busy)
            }
            is TelegramStatus.PasswordNeeded -> {
                Message(stringResource(R.string.telegram_password_message))
                OutlinedField(
                    typed,
                    edit,
                    stringResource(R.string.password),
                    Modifier.padding(top = 20.dp),
                    helper = problem ?: status.hint?.let { stringResource(R.string.telegram_password_hint, it) },
                    error = problem != null,
                    focus = true,
                    onDone = confirm,
                    secret = true,
                )
                OtherNumber(actions::signOut, enabled = !state.busy)
            }
            is TelegramStatus.SignedIn -> if (picking) Chats(status.name, chats, taken, actions, onDismissRequest)
        }
    }
}

@Composable
private fun Message(message: String) {
    Text(
        message,
        style = text(14, lineHeight = 20.sp),
        color = colors.onSurfaceVariant,
        modifier = Modifier.padding(top = 12.dp),
    )
}

/** Starts signing in again, for a code sent to the wrong number or one that never came. */
@Composable
private fun OtherNumber(onClick: () -> Unit, enabled: Boolean) {
    TextAction(
        stringResource(R.string.telegram_other_number),
        { if (enabled) onClick() },
        Modifier
            .padding(top = 8.dp)
            .bleed(12.dp),
        color = if (enabled) colors.primary else colors.onSurfaceQuiet,
        padding = PaddingValues(horizontal = 12.dp),
    )
}

/**
 * The chats of the account called [name], narrowed by a search. Picking one adds it and closes
 * the dialog; those [taken] already are greyed.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ColumnScope.Chats(
    name: String,
    chats: TelegramChats?,
    taken: Set<Locator>,
    actions: TelegramActions,
    onDismissRequest: () -> Unit,
) {
    var query by rememberSaveable { mutableStateOf("") }
    val search = query.trim()
    // The search asked last, and the answers there were by then: its own comes after them.
    var asked by rememberSaveable { mutableStateOf<String?>(null) }
    var askedAfter by rememberSaveable { mutableIntStateOf(0) }
    val latest by rememberUpdatedState(chats)
    LaunchedEffect(search) {
        // Typing settles for a moment before Telegram is asked again; the first search goes at once.
        if (asked != null) delay(SEARCH_SETTLE_MS)
        askedAfter = latest?.serial ?: 0
        asked = search
        actions.find(search)
    }
    val answer = chats?.takeIf { it.serial > askedAfter && it.query == asked }
    val found = chats?.chats.orEmpty()
    Message(stringResource(R.string.telegram_pick_message, name))
    SearchField(
        query,
        { query = it },
        stringResource(R.string.telegram_search),
        Modifier.padding(top = 16.dp),
        fill = colors.surfaceHighest,
    )
    val failed = answer?.error
    if (found.isEmpty() || failed != null) {
        Row(
            Modifier
                .fillMaxWidth()
                .heightIn(min = 96.dp)
                .padding(vertical = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            if (answer == null) LoadingIndicator(Modifier.size(32.dp), color = colors.primary)
            Text(
                when {
                    answer == null -> stringResource(R.string.telegram_searching)
                    failed != null -> telegramError(failed)
                    search.isNotEmpty() -> stringResource(R.string.telegram_no_match)
                    else -> stringResource(R.string.telegram_no_chats)
                },
                style = text(14, lineHeight = 20.sp),
                color = if (failed != null) colors.error else colors.onSurfaceVariant,
                modifier = Modifier.weight(1f),
            )
        }
    } else {
        LazyColumn(
            Modifier
                .weight(1f, fill = false)
                .padding(top = 8.dp)
                .bleed(8.dp),
        ) {
            items(found, key = { it.locator.toString() }) { chat ->
                val title = chatTitle(chat)
                ChatRow(chat, title, added = chat.locator in taken) {
                    actions.pick(chat.locator, title)
                    onDismissRequest()
                }
            }
        }
    }
    Text(
        stringResource(R.string.telegram_pick_hint),
        style = text(12, lineHeight = 16.sp),
        color = colors.onSurfaceVariant,
        modifier = Modifier.padding(top = 12.dp),
    )
}

/** A chat to pick: what kind it is, its name, and Added once it is a source. */
@Composable
private fun ChatRow(chat: TelegramChatInfo, title: String, added: Boolean, onPick: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 60.dp)
            .clip(RoundedCornerShape(16.dp))
            .then(if (added) Modifier.semantics { disabled() } else Modifier.tappable(onPick))
            .padding(horizontal = 8.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(
            Modifier
                .size(40.dp)
                .background(if (added) colors.surfaceHighest else colors.secondaryContainer, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            NIcon(kindIcon(chat.kind), size = 20.dp, tint = if (added) colors.onSurfaceQuiet else colors.onSecondaryContainer)
        }
        Column(Modifier.weight(1f)) {
            Text(
                title,
                style = text(15, FontWeight.SemiBold),
                color = if (added) colors.onSurfaceVariant else colors.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                dotted(stringResource(kindLabel(chat.kind)), chat.username?.let { "@$it" }),
                style = text(13),
                color = colors.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(top = 1.dp),
            )
        }
        if (added) {
            Text(stringResource(R.string.added), style = text(12, FontWeight.SemiBold), color = colors.onSurfaceQuiet)
        }
    }
}

/** What a chat goes by: its title, or Saved Messages for the account's own. */
@Composable
fun chatTitle(chat: TelegramChatInfo): String = when {
    chat.kind == TelegramChatKind.SAVED_MESSAGES -> stringResource(R.string.telegram_saved_messages)
    else -> chat.title.ifEmpty { stringResource(R.string.telegram) }
}

private fun kindLabel(kind: TelegramChatKind): Int = when (kind) {
    TelegramChatKind.SAVED_MESSAGES -> R.string.telegram_kind_saved
    TelegramChatKind.USER -> R.string.telegram_kind_user
    TelegramChatKind.BOT -> R.string.telegram_kind_bot
    TelegramChatKind.GROUP -> R.string.telegram_kind_group
    TelegramChatKind.CHANNEL -> R.string.telegram_kind_channel
}

private fun kindIcon(kind: TelegramChatKind): ImageVector = when (kind) {
    TelegramChatKind.SAVED_MESSAGES -> NIcons.Saved
    TelegramChatKind.USER, TelegramChatKind.BOT -> NIcons.Artist
    TelegramChatKind.GROUP -> NIcons.Group
    TelegramChatKind.CHANNEL -> NIcons.Telegram
}

/** Why a request to Telegram failed, the way people say it. */
@Composable
fun telegramError(error: TelegramError): String = when (error) {
    TelegramError.PhoneInvalid -> stringResource(R.string.telegram_error_phone_invalid)
    TelegramError.PhoneBanned -> stringResource(R.string.telegram_error_phone_banned)
    TelegramError.CodeInvalid -> stringResource(R.string.telegram_error_code_invalid)
    TelegramError.PasswordInvalid -> stringResource(R.string.telegram_error_password_invalid)
    TelegramError.SignUpRequired -> stringResource(R.string.telegram_error_sign_up_required)
    is TelegramError.Wait -> {
        val minutes = ((error.seconds.toInt() + 59) / 60).coerceAtLeast(1)
        pluralStringResource(R.plurals.telegram_error_wait, minutes, minutes)
    }
    TelegramError.SignedOut -> stringResource(R.string.telegram_error_signed_out)
    TelegramError.NotFound -> stringResource(R.string.telegram_error_not_found)
    is TelegramError.Offline -> stringResource(R.string.telegram_error_offline)
    is TelegramError.Failed -> stringResource(R.string.telegram_error_failed, error.v1)
}

/** Lays this out [by] wider on both sides, so its ripples reach past the text it lines up with. */
private fun Modifier.bleed(by: Dp): Modifier = layout { measurable, constraints ->
    val extra = (by * 2).roundToPx()
    val placeable = measurable.measure(
        constraints.copy(
            minWidth = constraints.minWidth + extra,
            maxWidth = if (constraints.hasBoundedWidth) constraints.maxWidth + extra else constraints.maxWidth,
        )
    )
    layout(placeable.width - extra, placeable.height) { placeable.place(-by.roundToPx(), 0) }
}

/** How long typing settles before the chats are looked for again. */
private const val SEARCH_SETTLE_MS = 300L
