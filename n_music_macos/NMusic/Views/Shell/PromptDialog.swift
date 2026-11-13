import SwiftUI

/// The Qt PromptDialog (spec §7.2) as a sheet: a title (17 Bold), an optional message, an
/// optional name field, a red problem line after a failed check, and Cancel + the confirm
/// button (primary or danger) on the right. Return confirms; Escape cancels.
///
/// `onConfirm` gets the trimmed name ("" without a field) and returns a problem to show, or
/// `nil` to close. With `requiresName`, confirm is disabled while the name is blank.
///
///     .sheet(item: $renaming) { playlist in
///         PromptDialog(title: tr("rename_playlist"), text: playlist.name,
///                      placeholder: tr("playlist_name"), confirmTitle: tr("rename")) { name in
///             store.renamePlaylist(playlist.id, to: name); return nil
///         }
///     }
struct PromptDialog: View {
    let title: String
    var message: String? = nil
    /// The field's initial text (selected on open); `nil` shows no field.
    var text: String? = ""
    var placeholder: String = ""
    let confirmTitle: String
    var danger = false
    var requiresName = true
    /// Shown under the field: e.g. the web dialog's second field. Optional.
    var extra: AnyView? = nil
    let onConfirm: (String) -> String?

    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var problem: String?
    @FocusState private var fieldFocused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(title)
                .font(.system(size: 17, weight: .bold))
                .foregroundStyle(Theme.text)
            if let message {
                Text(message)
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.text2)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.top, 8)
            }
            if text != nil {
                TextField(placeholder, text: $name)
                    .textFieldStyle(.roundedBorder)
                    .controlSize(.large)
                    .focused($fieldFocused)
                    .onChange(of: name) { problem = nil }
                    .onSubmit(confirm)
                    .padding(.top, 16)
            }
            if let problem {
                Text(problem)
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.danger)
                    .padding(.top, 8)
            }
            if let extra {
                extra.padding(.top, 12)
            }
            HStack(spacing: 8) {
                Spacer()
                Button(tr("cancel")) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                    .buttonStyle(PillButtonStyle(kind: .plain, small: true))
                Button(confirmTitle, action: confirm)
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(PillButtonStyle(kind: danger ? .danger : .primary, small: true))
                    .disabled(text != nil && requiresName && trimmed.isEmpty)
            }
            .padding(.top, 20)
        }
        .padding(20)
        .frame(width: 400)
        .background(Theme.surface)
        .onAppear {
            name = text ?? ""
            fieldFocused = text != nil
        }
    }

    private var trimmed: String { name.trimmingCharacters(in: .whitespacesAndNewlines) }

    private func confirm() {
        guard text == nil || !requiresName || !trimmed.isEmpty else { return }
        if let problem = onConfirm(trimmed) {
            self.problem = problem
        } else {
            dismiss()
        }
    }
}
