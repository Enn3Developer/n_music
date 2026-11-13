import SwiftUI

/// The Qt TelegramSourceDialog (spec §5.7) as a sheet: signs in to Telegram when needed (phone
/// number → code → password with 2FA), then lists the account's chats to pick one, or finds a
/// public one by its @name or t.me link. It closes on Escape (and its × / Cancel) only.
///
/// `signInOnly` (Settings › Telegram) closes once signed in instead of picking a chat.
/// `chosen(chat, title)`: the title becomes the source's display name.
struct TelegramSourceDialog: View {
    var taken: Set<Locator> = []
    var signInOnly = false
    var chosen: (Locator, String) -> Void = { _, _ in }

    private let store = Store.shared
    @Environment(\.dismiss) private var dismiss
    @State private var field = ""
    @State private var search = ""
    /// Steps that ended before it opened: an error is shown only for a step ended since.
    @State private var endedBefore = Int.max
    /// The last search sent, and the answer count when it was sent.
    @State private var sent: (query: String, serial: Int)?
    @State private var marked: Int?
    @State private var typing: Task<Void, Never>?
    @State private var focusSearch = 0
    /// A step was sent and its answer hasn't come: Return may reach both the field's submit and
    /// the default button, and `busy` only comes back with the core's answer.
    @State private var pending = false
    /// The chat was picked (the same double Return).
    @State private var picked = false
    @FocusState private var fieldFocused: Bool

    private var state: TelegramState? { store.telegram }
    private var status: TelegramStatus { state?.status ?? .signedOut }
    private var busy: Bool { (state?.busy ?? false) || pending }
    private var picking: Bool {
        if case .signedIn = status { return !signInOnly }
        return false
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .top, spacing: 8) {
                Text(tr(picking ? "add_telegram_source" : "telegram_sign_in"))
                    .font(.system(size: 17, weight: .bold))
                    .foregroundStyle(Theme.text)
                    .fixedSize(horizontal: false, vertical: true)
                Spacer(minLength: 0)
                IconButton(symbol: "xmark", label: tr("cancel"), size: 30, iconSize: 16) { dismiss() }
            }
            if picking {
                pickingStep
            } else {
                signingIn
            }
        }
        .padding(20)
        .frame(width: picking ? 560 : 420)
        .frame(height: picking ? pickerHeight : nil)
        .background(Theme.surface)
        .onAppear {
            endedBefore = state?.ended ?? 0
            field = ""
            search = ""
            if picking { find("") }
            focusStep()
        }
        .onChange(of: stepKey) {
            field = ""
            if case .signedIn = status {
                if signInOnly {
                    dismiss()
                    return
                }
                find(search)
            }
            focusStep()
        }
        .onChange(of: search) { _, text in
            marked = nil
            typing?.cancel()
            typing = Task {
                try? await Task.sleep(for: .milliseconds(300))
                guard !Task.isCancelled else { return }
                find(text)
            }
        }
        .onChange(of: state) { pending = false }
        // Escape closes from anywhere but the search, which clears itself first.
        .onExitCommand { dismiss() }
        .onChange(of: store.telegramChats?.serial) {
            if let marked, marked >= chats.count { self.marked = chats.isEmpty ? nil : chats.count - 1 }
        }
    }

    // MARK: Signing in

    private var signingIn: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(message)
                .font(.system(size: 14))
                .foregroundStyle(Theme.text2)
                .fixedSize(horizontal: false, vertical: true)
            if case .passwordNeeded(let hint) = status, let hint, !hint.isEmpty {
                Text(tr("telegram_password_hint", hint))
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.text3)
                    .fixedSize(horizontal: false, vertical: true)
            }
            if !signedIn {
                Group {
                    if case .passwordNeeded = status {
                        SecureField(text: $field, prompt: Text(placeholder)) { Text(placeholder) }
                    } else {
                        TextField(text: $field, prompt: Text(placeholder)) { Text(placeholder) }
                    }
                }
                .textFieldStyle(.roundedBorder)
                .controlSize(.large)
                .disabled(busy)
                .focused($fieldFocused)
                .onSubmit(next)
            }
            if let problem = stepProblem {
                Text(problem)
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.danger)
                    .fixedSize(horizontal: false, vertical: true)
            }
            HStack(spacing: 8) {
                if codeOrPassword {
                    Button(tr("telegram_other_number")) { store.telegramSignOut() }
                        .buttonStyle(PillButtonStyle(kind: .plain, small: true))
                        .disabled(busy)
                }
                Spacer()
                Button(tr(signedIn ? "done" : "cancel")) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                    .buttonStyle(PillButtonStyle(kind: .plain, small: true))
                if !signedIn {
                    Button(primaryTitle, action: next)
                        .keyboardShortcut(.defaultAction)
                        .buttonStyle(PillButtonStyle(kind: .primary, small: true))
                        .disabled(busy || field.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }
            .padding(.top, 4)
        }
    }

    private var signedIn: Bool {
        if case .signedIn = status { return true }
        return false
    }

    private var codeOrPassword: Bool {
        switch status {
        case .codeSent, .passwordNeeded: true
        default: false
        }
    }

    /// Tells the steps apart (and the same step reached again with new values).
    private var stepKey: String {
        switch status {
        case .signedOut: "signedOut"
        case .codeSent(let phone): "codeSent:\(phone)"
        case .passwordNeeded: "passwordNeeded"
        case .signedIn: "signedIn"
        }
    }

    private var message: String {
        switch status {
        case .codeSent(let phone): tr("telegram_code_message", "+" + phone)
        case .passwordNeeded: tr("telegram_password_message")
        case .signedIn(let name): tr("telegram_signed_in_as", name)
        case .signedOut: tr("telegram_phone_message")
        }
    }

    private var placeholder: String {
        switch status {
        case .signedOut: tr("telegram_phone_placeholder")
        case .codeSent: tr("telegram_code_placeholder")
        case .passwordNeeded: tr("telegram_password_placeholder")
        case .signedIn: ""
        }
    }

    private var primaryTitle: String {
        if busy { return tr("telegram_waiting") }
        if case .signedOut = status { return tr("telegram_send_code") }
        return tr("telegram_sign_in_action")
    }

    /// The error of a step that ended while the dialog was open.
    private var stepProblem: String? {
        guard let state, state.ended > endedBefore, let error = state.error else { return nil }
        return Self.problem(error)
    }

    private func next() {
        let text = field.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !busy, !text.isEmpty else { return }
        pending = true
        // Should the core never answer, the step can be sent again.
        Task { @MainActor in
            try? await Task.sleep(for: .seconds(10))
            pending = false
        }
        switch status {
        case .signedOut: store.telegramSignIn(phone: text)
        case .codeSent: store.telegramCode(text)
        case .passwordNeeded: store.telegramPassword(text)
        case .signedIn: break
        }
    }

    private func focusStep() {
        if picking {
            focusSearch += 1
        } else {
            Task { @MainActor in fieldFocused = true }
        }
    }

    // MARK: Picking a chat

    private var account: String {
        if case .signedIn(let name) = status { return name }
        return ""
    }

    private var chats: [TelegramChatInfo] { store.telegramChats?.chats ?? [] }

    private var pickerHeight: CGFloat {
        let window = Windows.shared.main?.contentLayoutRect.height ?? 800
        return min(600, window - 32)
    }

    private var pickingStep: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(tr("telegram_pick_message", account))
                .font(.system(size: 14))
                .foregroundStyle(Theme.text2)
                .fixedSize(horizontal: false, vertical: true)
            SearchBox(
                text: $search,
                placeholder: tr("telegram_search_placeholder"),
                styled: true,
                focusRequest: focusSearch,
                onUp: {
                    guard let marked else { return false }
                    self.marked = marked > 0 ? marked - 1 : nil
                    return true
                },
                onDown: {
                    guard !chats.isEmpty else { return false }
                    marked = min((marked ?? -1) + 1, chats.count - 1)
                    return true
                },
                onSubmit: {
                    // The highlighted row, else the first result.
                    let index = marked ?? 0
                    if chats.indices.contains(index) { pick(chats[index]) }
                },
                onEscape: { dismiss() }
            )
            chatList
            HStack(spacing: 8) {
                Text(tr("telegram_pick_hint"))
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.text3)
                    .fixedSize(horizontal: false, vertical: true)
                Spacer(minLength: 0)
                // No Escape shortcut: it would beat the search, which Escape clears first.
                Button(tr("cancel")) { dismiss() }
                    .buttonStyle(PillButtonStyle(kind: .plain, small: true))
            }
        }
    }

    private var chatList: some View {
        ScrollViewReader { scroller in
            ScrollView {
                LazyVStack(spacing: 0) {
                    ForEach(Array(chats.enumerated()), id: \.offset) { index, chat in
                        ChatRow(
                            title: Self.title(chat),
                            detail: Self.detail(chat),
                            symbol: Self.symbol(chat.kind),
                            added: taken.contains(chat.locator),
                            marked: marked == index
                        ) { pick(chat) }
                        .id(index)
                    }
                }
                .padding(6)
            }
            .onChange(of: marked) { _, index in
                if let index { scroller.scrollTo(index) }
            }
        }
        .frame(maxHeight: .infinity)
        .overlay {
            if chats.isEmpty {
                Text(emptyText)
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.text3)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 24)
            }
        }
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(Theme.line, lineWidth: 1))
        .accessibilityLabel(tr("telegram_chats"))
    }

    private var searching: Bool {
        guard let sent else { return false }
        guard let answer = store.telegramChats else { return true }
        return answer.serial <= sent.serial || answer.query != sent.query
    }

    private var emptyText: String {
        if searching { return tr("telegram_searching") }
        if let error = store.telegramChats?.error { return Self.problem(error) }
        return (store.telegramChats?.query ?? "").isEmpty ? tr("telegram_no_chats") : tr("telegram_no_match")
    }

    private func find(_ query: String) {
        sent = (query, store.telegramChats?.serial ?? 0)
        store.findTelegramChats(query)
    }

    /// A chat already added does nothing, without a word (as Qt).
    private func pick(_ chat: TelegramChatInfo) {
        guard !picked, !taken.contains(chat.locator) else { return }
        picked = true
        dismiss()
        chosen(chat.locator, Self.title(chat))
    }

    // MARK: Wording

    /// Why a Telegram request failed: `telegram_error_<code>` with its detail.
    static func problem(_ error: TelegramError) -> String {
        let text = telegramErrorText(error: error)
        return tr(text.key, fallback: Tr.shared.t("telegram_error_failed"), text.detail)
    }

    /// Saved Messages has no name of its own.
    static func title(_ chat: TelegramChatInfo) -> String {
        chat.kind == .savedMessages ? tr("telegram_saved_messages") : chat.title
    }

    /// "<kind> · @username".
    static func detail(_ chat: TelegramChatInfo) -> String {
        let kind = tr(telegramChatKindKey(kind: chat.kind))
        guard let username = chat.username, !username.isEmpty else { return kind }
        return kind + " · @" + username
    }

    static func symbol(_ kind: TelegramChatKind) -> String {
        switch kind {
        case .savedMessages: "tag"
        case .user, .bot: "person"
        // Qt's queue icon, as the sidebar's Queue shows it.
        case .group: "text.line.first.and.arrowtriangle.forward"
        case .channel: "paperplane"
        }
    }
}

/// A chat to pick: 52 points, its kind's icon, the title over "<kind> · @username", "Added" and
/// disabled when it is a source already.
private struct ChatRow: View {
    let title: String
    let detail: String
    let symbol: String
    let added: Bool
    let marked: Bool
    let action: () -> Void

    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Image(systemName: symbol)
                    .font(.system(size: 17, weight: .regular))
                    .foregroundStyle(hovering || marked ? Theme.accentText : Theme.text2)
                    .frame(width: 22)
                VStack(alignment: .leading, spacing: 1) {
                    Text(title)
                        .font(.system(size: 14, weight: .medium))
                        .foregroundStyle(added ? Theme.text2 : Theme.text)
                        .lineLimit(1)
                    Text(detail)
                        .font(.system(size: 12))
                        .foregroundStyle(Theme.text3)
                        .lineLimit(1)
                }
                Spacer(minLength: 0)
                if added {
                    SourceBadge(text: tr("added"))
                }
            }
            .padding(.horizontal, 10)
            .frame(height: 52)
            .contentShape(Rectangle())
            .background(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(marked ? Theme.selected : hovering ? Theme.hover : .clear)
            )
        }
        .buttonStyle(.plain)
        .focusable(false)
        .disabled(added)
        .onHover { hovering = $0 && !added }
        .accessibilityLabel(title)
        .accessibilityHint(detail)
    }
}
