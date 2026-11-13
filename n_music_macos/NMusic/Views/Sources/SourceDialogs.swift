import AppKit
import SwiftUI

/// The Qt WebSourceDialog (spec §5.6) as a sheet: an address field (empty, focused), a Display
/// name whose placeholder is the address's default name (or "Optional" while it is none), a
/// red line when the address is not http(s) on confirm, cleared on the next edit; "Add" is
/// disabled while the address is blank. `https://` is assumed when no scheme is written.
struct WebSourceDialog: View {
    let chosen: (Locator, String) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var address = ""
    @State private var name = ""
    @State private var problem: String?
    /// Chosen already: Return may reach both a field's submit and the default button.
    @State private var done = false
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(tr("add_web_source"))
                .font(.system(size: 17, weight: .bold))
                .foregroundStyle(Theme.text)
            Text(tr("web_source_message"))
                .font(.system(size: 14))
                .foregroundStyle(Theme.text2)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.top, 8)
            TextField(text: $address, prompt: Text(verbatim: "https://example.com/music.m3u")) {
                Text(tr("add_web_source"))
            }
            .textFieldStyle(.roundedBorder)
            .controlSize(.large)
            .focused($focused)
            .onChange(of: address) { problem = nil }
            .onSubmit(confirm)
            .padding(.top, 16)
            if let problem {
                Text(problem)
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.danger)
                    .padding(.top, 8)
            }
            SourceNameField(text: $name, defaultName: normalizeWebAddress(text: address).map { defaultSourceName(root: $0) } ?? "")
                .onSubmit(confirm)
                .padding(.top, 12)
            HStack(spacing: 8) {
                Spacer()
                Button(tr("cancel")) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                    .buttonStyle(PillButtonStyle(kind: .plain, small: true))
                Button(tr("add"), action: confirm)
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(PillButtonStyle(kind: .primary, small: true))
                    .disabled(address.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            .padding(.top, 20)
        }
        .padding(20)
        .frame(width: 400)
        .background(Theme.surface)
        .onAppear { focused = true }
        #if DEBUG
        .onChange(of: SourcesDebug.shared.request) { _, _ in
            guard let command = SourcesDebug.shared.command else { return }
            switch command.name {
            case "web-address": address = command.argument
            case "web-name": name = command.argument
            case "web-confirm": confirm()
            default: break
            }
        }
        #endif
    }

    private func confirm() {
        guard !done, !address.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        guard let root = normalizeWebAddress(text: address) else {
            problem = tr("web_address_invalid")
            return
        }
        done = true
        dismiss()
        chosen(root, name.trimmingCharacters(in: .whitespacesAndNewlines))
    }
}

/// The Qt SourceNameField: "Display name" beside a field whose placeholder is the default name,
/// or "Optional" while that is unknown.
struct SourceNameField: View {
    @Binding var text: String
    var defaultName: String

    var body: some View {
        HStack(spacing: 12) {
            Text(tr("display_name"))
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Theme.text2)
            TextField(text: $text, prompt: Text(defaultName.isEmpty ? tr("optional") : defaultName)) {
                Text(tr("display_name"))
            }
            .textFieldStyle(.roundedBorder)
            .controlSize(.large)
        }
    }
}

extension View {
    /// The dialogs that add a source (web playlist, Telegram chat), shown while their flags are
    /// set; `chosen` gets the source and the name typed ("" for its default name).
    func sourceAddSheets(
        web: Binding<Bool>, telegram: Binding<Bool>, taken: Set<Locator>,
        chosen: @escaping (Locator, String) -> Void
    ) -> some View {
        self
            .sheet(isPresented: web) {
                WebSourceDialog(chosen: chosen)
            }
            .sheet(isPresented: telegram) {
                TelegramSourceDialog(taken: taken, chosen: chosen)
            }
    }

    /// The Rename source and Remove source dialogs (spec §7.3) for the source set in each;
    /// `removed` runs once a source is removed (a source's own page then goes to Sources).
    func sourceDialogs(
        renaming: Binding<SourceInfo?>, removing: Binding<SourceInfo?>, removed: (() -> Void)? = nil
    ) -> some View {
        self
            .sheet(item: SourceSheet.binding(renaming)) { sheet in
                let source = sheet.source
                PromptDialog(
                    title: tr("rename_source"),
                    message: tr("rename_source_message", source.defaultName),
                    text: source.displayName ?? "",
                    placeholder: source.defaultName,
                    confirmTitle: tr("rename"),
                    requiresName: false
                ) { name in
                    Store.shared.renameSource(source.root, to: name)
                    return nil
                }
            }
            .sheet(item: SourceSheet.binding(removing)) { sheet in
                let source = sheet.source
                PromptDialog(
                    title: tr("remove_source_title", source.name),
                    message: tr("remove_source_message"),
                    text: nil,
                    confirmTitle: tr("remove"),
                    danger: true
                ) { _ in
                    let root = source.root
                    Task { await Store.shared.removeSource(root) }
                    removed?()
                    return nil
                }
            }
    }
}

/// A source a dialog is about (`SourceInfo` itself isn't `Identifiable`: other pages may make
/// it so, and two conformances wouldn't build).
private struct SourceSheet: Identifiable {
    let source: SourceInfo
    var id: String { source.root.key }

    @MainActor
    static func binding(_ source: Binding<SourceInfo?>) -> Binding<SourceSheet?> {
        Binding(get: { source.wrappedValue.map(SourceSheet.init) }, set: { source.wrappedValue = $0?.source })
    }
}

/// A search field (`NSSearchField`) that also tells ↑, ↓ and Return, for lists driven from
/// their search (the Telegram chat picker). Escape clears it while it holds text (Qt's
/// SearchField), else runs `onEscape` (or goes on up, to close a dialog).
///
/// `styled`: Qt's SearchField look around the native field: 38 points, radius 10, the `field`
/// fill, a `line2` border (`border` while it has focus), the text at 14.
struct SearchBox: View {
    @Binding var text: String
    let placeholder: String
    var styled = false
    var focusRequest = 0
    var onUp: (() -> Bool)? = nil
    var onDown: (() -> Bool)? = nil
    var onSubmit: (() -> Void)? = nil
    var onEscape: (() -> Void)? = nil

    @State private var focused = false

    var body: some View {
        let field = SearchFieldView(
            text: $text, focused: $focused, placeholder: placeholder, styled: styled,
            focusRequest: focusRequest, onUp: onUp, onDown: onDown, onSubmit: onSubmit, onEscape: onEscape
        )
        if styled {
            field
                .padding(.leading, 6)
                .padding(.trailing, 4)
                .frame(maxHeight: .infinity)
                .background(RoundedRectangle(cornerRadius: 10, style: .continuous).fill(Theme.field))
                .overlay(
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .strokeBorder(focused ? Theme.border : Theme.line2, lineWidth: 1)
                )
                .frame(height: 38)
        } else {
            field
        }
    }
}

private struct SearchFieldView: NSViewRepresentable {
    @Binding var text: String
    @Binding var focused: Bool
    let placeholder: String
    let styled: Bool
    let focusRequest: Int
    let onUp: (() -> Bool)?
    let onDown: (() -> Bool)?
    let onSubmit: (() -> Void)?
    let onEscape: (() -> Void)?

    func makeNSView(context: Context) -> FocusSearchField {
        let field = FocusSearchField()
        field.placeholderString = placeholder
        field.delegate = context.coordinator
        field.sendsSearchStringImmediately = true
        field.setAccessibilityLabel(placeholder)
        if styled {
            field.isBezeled = false
            field.isBordered = false
            field.drawsBackground = false
            field.focusRingType = .none
            field.font = .systemFont(ofSize: 14)
        } else {
            field.controlSize = .large
            field.font = .systemFont(ofSize: 13)
        }
        let coordinator = context.coordinator
        field.focusChanged = { [weak coordinator] in coordinator?.setFocused($0) }
        return field
    }

    func updateNSView(_ field: FocusSearchField, context: Context) {
        context.coordinator.parent = self
        if field.stringValue != text { field.stringValue = text }
        if field.placeholderString != placeholder { field.placeholderString = placeholder }
        if context.coordinator.focused != focusRequest {
            context.coordinator.focused = focusRequest
            DispatchQueue.main.async { field.window?.makeFirstResponder(field) }
        }
    }

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    @MainActor
    final class Coordinator: NSObject, NSSearchFieldDelegate {
        var parent: SearchFieldView
        var focused = 0

        init(_ parent: SearchFieldView) {
            self.parent = parent
        }

        func setFocused(_ value: Bool) {
            if parent.focused != value { parent.focused = value }
        }

        func controlTextDidChange(_ notification: Notification) {
            guard let field = notification.object as? NSSearchField else { return }
            parent.text = field.stringValue
        }

        func controlTextDidEndEditing(_ notification: Notification) {
            setFocused(false)
        }

        func control(_ control: NSControl, textView: NSTextView, doCommandBy selector: Selector) -> Bool {
            switch selector {
            case #selector(NSResponder.moveUp(_:)): return parent.onUp?() ?? false
            case #selector(NSResponder.moveDown(_:)): return parent.onDown?() ?? false
            case #selector(NSResponder.insertNewline(_:)):
                guard let onSubmit = parent.onSubmit else { return false }
                onSubmit()
                return true
            case #selector(NSResponder.cancelOperation(_:)):
                if !textView.string.isEmpty {
                    textView.string = ""
                    (control as? NSSearchField)?.stringValue = ""
                    parent.text = ""
                    return true
                }
                guard let onEscape = parent.onEscape else { return false }
                onEscape()
                return true
            default: return false
            }
        }
    }
}

/// Tells when it takes the focus (its field editor tells when it gives it up).
final class FocusSearchField: NSSearchField {
    var focusChanged: ((Bool) -> Void)?

    override func becomeFirstResponder() -> Bool {
        let became = super.becomeFirstResponder()
        if became { focusChanged?(true) }
        return became
    }
}
