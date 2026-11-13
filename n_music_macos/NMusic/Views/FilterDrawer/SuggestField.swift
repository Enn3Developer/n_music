import AppKit
import SwiftUI

/// The rule editor's text box (`qml/TextBox.qml`): 36 points, `input` ground, radius 8, a
/// 1-point `border` (`text3` under the pointer, the accent while focused), 10 points in. For a
/// library field it offers up to 8 of the library's values containing what is typed (those
/// starting with it first, never the exact one) in a list under it: ↑/↓ move, Return or a click
/// picks, Escape closes it, as does leaving the field; Escape without the list closes the
/// drawer. `numeric` takes only digits, at most 9.
struct SuggestField: View {
    @Binding var text: String
    var values: ValueField? = nil
    var numeric = false
    var label: String

    @State private var hovering = false
    @State private var focused = false

    var body: some View {
        SuggestTextField(text: $text, values: values, numeric: numeric, label: label) { focused = $0 }
            .padding(.horizontal, 10)
            .frame(height: 36)
            .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(Theme.input))
            .overlay(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .strokeBorder(focused ? Theme.accent : hovering ? Theme.text3 : Theme.border, lineWidth: 1)
            )
            .animation(Motion.standard(Motion.fade), value: focused)
            .animation(Motion.standard(Motion.fade), value: hovering)
            .onHover { hovering = $0 }
    }
}

/// The field inside a `SuggestField`: borderless, on the box's ground.
private struct SuggestTextField: NSViewRepresentable {
    @Binding var text: String
    var values: ValueField?
    var numeric: Bool
    var label: String
    var onFocus: (Bool) -> Void

    func makeCoordinator() -> SuggestController { SuggestController() }

    func makeNSView(context: Context) -> NSTextField {
        let field = SuggestNSTextField(string: text)
        field.isBezeled = false
        field.isBordered = false
        field.drawsBackground = false
        field.focusRingType = .none
        field.textColor = NSColor(Theme.text)
        field.onFocus = { [weak coordinator = context.coordinator] in coordinator?.focusChanged(true) }
        field.font = numeric ? .monospacedDigitSystemFont(ofSize: 14, weight: .regular) : .systemFont(ofSize: 14)
        field.lineBreakMode = .byTruncatingTail
        field.cell?.isScrollable = true
        field.cell?.wraps = false
        field.delegate = context.coordinator
        field.setAccessibilityLabel(label)
        field.setContentHuggingPriority(.defaultLow, for: .horizontal)
        field.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        context.coordinator.field = field
        return field
    }

    func updateNSView(_ field: NSTextField, context: Context) {
        let coordinator = context.coordinator
        coordinator.text = $text
        coordinator.values = values
        coordinator.numeric = numeric
        coordinator.onFocus = onFocus
        if field.stringValue != text, field.currentEditor() == nil || coordinator.lastTyped != text {
            field.stringValue = text
        }
        field.setAccessibilityLabel(label)
    }

    static func dismantleNSView(_ field: NSTextField, coordinator: SuggestController) {
        coordinator.close()
    }
}

@MainActor
final class SuggestController: NSObject, NSTextFieldDelegate {
    weak var field: NSTextField?
    var text: Binding<String>?
    var values: ValueField?
    var numeric = false
    /// What the user typed last, so an update doesn't fight the field editor.
    var lastTyped: String?
    var onFocus: ((Bool) -> Void)?

    func focusChanged(_ focused: Bool) {
        onFocus?(focused)
    }

    private var library: [String] = []
    private var matches: [String] = []
    private var current = -1
    private var panel: SuggestPanel?

    private var loaded: ValueField?

    func controlTextDidBeginEditing(_ notification: Notification) {
        load()
    }

    /// Reads the library's values as the field is edited, then offers them for what is typed.
    private func load() {
        guard let values, loaded != values else { return }
        loaded = values
        Task {
            library = await FilterCatalog.shared.values(values)
            if let field, field.currentEditor() != nil { suggest(field.stringValue) }
        }
    }

    func controlTextDidChange(_ notification: Notification) {
        guard let field else { return }
        var value = field.stringValue
        if numeric {
            let digits = String(value.filter(\.isASCII).filter(\.isNumber).prefix(9))
            if digits != value {
                field.stringValue = digits
                value = digits
            }
        }
        lastTyped = value
        text?.wrappedValue = value
        load()
        suggest(value)
    }

    func controlTextDidEndEditing(_ notification: Notification) {
        close()
        focusChanged(false)
    }

    func control(_ control: NSControl, textView: NSTextView, doCommandBy selector: Selector) -> Bool {
        guard panel != nil, !matches.isEmpty else {
            // Escape with no list open goes on to the drawer, which closes.
            if selector == #selector(NSResponder.cancelOperation(_:)) || selector == #selector(NSResponder.complete(_:)) {
                Navigation.shared.filterDrawer = nil
                return true
            }
            return false
        }
        switch selector {
        case #selector(NSResponder.moveDown(_:)):
            current = min(matches.count - 1, current + 1)
            panel?.highlight(current)
            return true
        case #selector(NSResponder.moveUp(_:)):
            current = max(0, current - 1)
            panel?.highlight(current)
            return true
        case #selector(NSResponder.insertNewline(_:)):
            guard current >= 0 else { return false }
            pick(matches[current])
            return true
        case #selector(NSResponder.cancelOperation(_:)):
            close()
            return true
        default:
            return false
        }
    }

    private func suggest(_ typed: String) {
        let typed = typed.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard values != nil, !typed.isEmpty else {
            close()
            return
        }
        var starting: [String] = []
        var containing: [String] = []
        for value in library {
            let lower = value.lowercased()
            if lower == typed { continue }
            if lower.hasPrefix(typed) {
                starting.append(value)
            } else if lower.contains(typed) {
                containing.append(value)
            }
            if starting.count >= 8 { break }
        }
        matches = Array((starting + containing).prefix(8))
        current = -1
        guard !matches.isEmpty else {
            close()
            return
        }
        show()
    }

    private func pick(_ value: String) {
        guard let field else { return }
        field.stringValue = value
        if let editor = field.currentEditor() {
            editor.selectedRange = NSRange(location: (value as NSString).length, length: 0)
        }
        lastTyped = value
        text?.wrappedValue = value
        close()
    }

    private func show() {
        guard let field, let window = field.window else { return }
        let panel = self.panel ?? SuggestPanel { [weak self] index in
            guard let self, self.matches.indices.contains(index) else { return }
            self.pick(self.matches[index])
        }
        self.panel = panel
        panel.show(matches, highlighted: current)
        // Under the box (the field sits 10 in, centred in 36).
        var rect = window.convertToScreen(field.convert(field.bounds, to: nil))
        rect = rect.insetBy(dx: -10, dy: -max(0, (36 - rect.height) / 2))
        let width = max(rect.width, 220)
        let height = panel.contentHeight
        panel.setFrame(NSRect(x: rect.minX, y: rect.minY - 4 - height, width: width, height: height), display: true)
        if panel.parent == nil {
            window.addChildWindow(panel, ordered: .above)
        }
        panel.orderFront(nil)
    }

    func close() {
        guard let panel else { return }
        panel.parent?.removeChildWindow(panel)
        panel.orderOut(nil)
        self.panel = nil
        matches = []
        current = -1
    }
}

/// The list of suggestions: a borderless panel that never takes the focus from the field.
final class SuggestPanel: NSPanel {
    private let stack = NSStackView()
    private let pick: (Int) -> Void
    private var rows: [SuggestRow] = []

    init(pick: @escaping (Int) -> Void) {
        self.pick = pick
        super.init(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: true)
        isOpaque = false
        backgroundColor = .clear
        hasShadow = true
        level = .popUpMenu
        hidesOnDeactivate = true
        let content = SuggestBackground()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 0
        stack.edgeInsets = NSEdgeInsets(top: 6, left: 6, bottom: 6, right: 6)
        stack.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            stack.topAnchor.constraint(equalTo: content.topAnchor),
        ])
        contentView = content
    }

    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }

    var contentHeight: CGFloat { CGFloat(rows.count) * 30 + 12 }

    func show(_ values: [String], highlighted: Int) {
        rows.forEach { $0.removeFromSuperview() }
        rows = values.enumerated().map { index, value in
            let row = SuggestRow(text: value) { [weak self] in self?.pick(index) }
            stack.addArrangedSubview(row)
            row.widthAnchor.constraint(equalTo: stack.widthAnchor, constant: -12).isActive = true
            return row
        }
        highlight(highlighted)
    }

    func highlight(_ index: Int) {
        for (offset, row) in rows.enumerated() {
            row.highlighted = offset == index
        }
    }
}

private final class SuggestBackground: NSView {
    override var wantsUpdateLayer: Bool { true }

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layer?.cornerRadius = 10
        layer?.borderWidth = 1
    }

    required init?(coder: NSCoder) { fatalError() }

    override func updateLayer() {
        layer?.backgroundColor = NSColor(Theme.menu).cgColor
        layer?.borderColor = NSColor(Theme.line2).cgColor
    }
}

/// One suggestion: 30 points, `menuHover` while highlighted or under the pointer.
private final class SuggestRow: NSView {
    private let label = NSTextField(labelWithString: "")
    private let action: () -> Void
    var highlighted = false {
        didSet { needsDisplay = true }
    }
    private var hovering = false {
        didSet { needsDisplay = true }
    }

    init(text: String, action: @escaping () -> Void) {
        self.action = action
        super.init(frame: .zero)
        label.stringValue = text
        label.font = .systemFont(ofSize: 13, weight: .medium)
        label.textColor = NSColor(Theme.text)
        label.lineBreakMode = .byTruncatingTail
        label.translatesAutoresizingMaskIntoConstraints = false
        addSubview(label)
        translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            heightAnchor.constraint(equalToConstant: 30),
            label.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 10),
            label.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -10),
            label.centerYAnchor.constraint(equalTo: centerYAnchor),
        ])
        addTrackingArea(NSTrackingArea(rect: .zero, options: [.mouseEnteredAndExited, .activeAlways, .inVisibleRect], owner: self))
    }

    required init?(coder: NSCoder) { fatalError() }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
    override func mouseEntered(with event: NSEvent) { hovering = true }
    override func mouseExited(with event: NSEvent) { hovering = false }
    override func mouseDown(with event: NSEvent) { action() }

    override func draw(_ dirtyRect: NSRect) {
        guard highlighted || hovering else { return }
        NSColor(Theme.menuHover).setFill()
        NSBezierPath(roundedRect: bounds, xRadius: 6, yRadius: 6).fill()
    }
}

/// Tells when it takes the focus (the end comes as `controlTextDidEndEditing`).
private final class SuggestNSTextField: NSTextField {
    var onFocus: (() -> Void)?

    override func becomeFirstResponder() -> Bool {
        let became = super.becomeFirstResponder()
        if became { onFocus?() }
        return became
    }
}
