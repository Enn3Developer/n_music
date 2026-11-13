import AppKit
import SwiftUI

/// One entry of a menu, rendered either as a native `NSMenu` popped up under a button (the
/// "⋯" and Add source buttons, which Qt opens right-aligned under themselves and which the File
/// menu and the empty state open from code) or as SwiftUI items of a `.contextMenu`
/// (right-click). One list, so both menus always say the same.
enum SourceMenuEntry {
    case item(title: String, symbol: String?, enabled: Bool = true, danger: Bool = false, checked: Bool = false, action: () -> Void)
    /// A kind of source: its name over its detail, with a "Later" badge when not available.
    case kind(SourceKind, action: () -> Void)
    case separator
    /// A note that can't be chosen (Qt's `MenuEntry { note: true }`).
    case note(String)
}

/// Builds and pops up native menus from `SourceMenuEntry` lists.
@MainActor
enum SourceMenus {
    /// The menu shown now, so a Debug hook can close it.
    static weak var shown: NSMenu?
    /// Tells the kinds' rows which one is highlighted (a menu holds its delegate weakly).
    private static var highlighter: KindRowHighlighter?

    static func menu(_ entries: [SourceMenuEntry], minimumWidth: CGFloat = 220) -> NSMenu {
        let menu = NSMenu()
        menu.autoenablesItems = false
        menu.minimumWidth = minimumWidth
        var rows: [KindMenuItemView] = []
        for entry in entries {
            switch entry {
            case .item(let title, let symbol, let enabled, let danger, let checked, let action):
                let item = actionItem(title, action)
                if let symbol {
                    // Qt's entries: a 16-point icon in `text2`, red for danger.
                    let configuration = NSImage.SymbolConfiguration(pointSize: 14, weight: .regular)
                        .applying(.init(paletteColors: [danger ? .systemRed : .secondaryLabelColor]))
                    item.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)?
                        .withSymbolConfiguration(configuration)
                    showImage(item)
                }
                item.isEnabled = enabled
                item.state = checked ? .on : .off
                if danger {
                    item.attributedTitle = NSAttributedString(string: title, attributes: [
                        .foregroundColor: NSColor.systemRed,
                        .font: NSFont.menuFont(ofSize: 0),
                    ])
                }
                menu.addItem(item)
            case .kind(let kind, let action):
                // Qt's SourceKindEntry, drawn by SwiftUI: a native item can't hold the icon
                // well, the two lines and the badge (and has no subtitle before macOS 14.4).
                let item = actionItem(kind.name, action)
                let row = KindMenuItemView(kind: kind, width: minimumWidth)
                item.view = row
                item.isEnabled = !kind.later
                item.setAccessibilityHelp(kind.detail)
                rows.append(row)
                menu.addItem(item)
            case .separator:
                menu.addItem(.separator())
            case .note(let text):
                let item = NSMenuItem(title: text, action: nil, keyEquivalent: "")
                item.attributedTitle = NSAttributedString(string: text, attributes: [
                    .foregroundColor: NSColor.secondaryLabelColor,
                    .font: NSFont.systemFont(ofSize: 12),
                ])
                item.isEnabled = false
                menu.addItem(item)
            }
        }
        if !rows.isEmpty {
            let highlighter = KindRowHighlighter(rows)
            Self.highlighter = highlighter
            menu.delegate = highlighter
        }
        return menu
    }

    /// macOS 27 hides menu item images unless asked (Qt's entries always show their icon).
    private static func showImage(_ item: NSMenuItem) {
        if #available(macOS 27, *) {
            item.preferredImageVisibility = .visible
        }
    }

    /// Pops `entries` up under `view`, its right edge on the view's (Qt's right-aligned menus),
    /// `gap` points below it, then calls `closed` once the menu closed (also when it could not
    /// open). It returns at once: the menu opens from the run loop (`MainRunLoop`), never from
    /// inside a task, whose main-actor job its tracking loop would otherwise keep running, and
    /// every other main-actor job waiting, while the menu shows.
    static func popUp(
        _ entries: [SourceMenuEntry], under view: NSView?, minimumWidth: CGFloat = 220, gap: CGFloat = 4,
        closed: @escaping @MainActor () -> Void = {}
    ) {
        guard let view else {
            closed()
            return
        }
        MainRunLoop.perform { [weak view] in
            defer { closed() }
            guard let view, view.window != nil else { return }
            let menu = menu(entries, minimumWidth: minimumWidth)
            shown = menu
            let x = view.bounds.width - menu.size.width
            let y = view.isFlipped ? view.bounds.height + gap : -gap
            menu.popUp(positioning: nil, at: NSPoint(x: x, y: y), in: view)
            highlighter = nil
        }
    }

    fileprivate static func actionItem(_ title: String, _ action: @escaping () -> Void) -> NSMenuItem {
        let target = MenuTarget(action)
        let item = NSMenuItem(title: title, action: #selector(MenuTarget.run(_:)), keyEquivalent: "")
        item.target = target
        // The item holds its target (an item's target is weak).
        item.representedObject = target
        return item
    }
}

/// A kind of source as a menu row (Qt `SourceKindEntry.qml`): a 32-point well with the kind's
/// icon (accent, `text3` when later; the well inverted while highlighted), the name (14
/// DemiBold) over the detail (12, wrapping), a "Later" badge; at least 50 points tall, the
/// highlight `menuHover` with radius 8.
@MainActor
@Observable
private final class KindRowState {
    var highlighted = false
}

private struct KindMenuRow: View {
    let kind: SourceKind
    let state: KindRowState
    let width: CGFloat

    var body: some View {
        let later = kind.later
        let well = state.highlighted ? Theme.menu : Theme.menuHover
        HStack(spacing: 12) {
            Image(systemName: kind.symbol)
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(later ? Theme.text3 : Theme.accentText)
                .frame(width: 32, height: 32)
                .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(well))
            VStack(alignment: .leading, spacing: 1) {
                Text(kind.name)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(later ? Theme.text2 : Theme.text)
                    .lineLimit(1)
                Text(kind.detail)
                    .font(.system(size: 12))
                    .foregroundStyle(Theme.text2)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if later {
                Text(tr("later"))
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(Theme.text2)
                    .padding(.horizontal, 7)
                    .frame(height: 20)
                    .background(Capsule().fill(well))
                    .fixedSize()
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .frame(minHeight: 50)
        .background(
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(state.highlighted ? Theme.menuHover : .clear)
        )
        .padding(.horizontal, 5)
        .frame(width: width)
        .accessibilityElement(children: .combine)
    }
}

/// The row's view in the menu: takes the clicks (a menu doesn't run a view item's action on
/// its own) and runs the item's action as a click on a native item would.
@MainActor
private final class KindMenuItemView: NSView {
    fileprivate let state = KindRowState()

    init(kind: SourceKind, width: CGFloat) {
        let host = NSHostingView(rootView: KindMenuRow(kind: kind, state: state, width: width))
        let height = max(50, host.fittingSize.height)
        super.init(frame: NSRect(x: 0, y: 0, width: width, height: height))
        host.frame = bounds
        host.autoresizingMask = [.width, .height]
        addSubview(host)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError() }

    override func hitTest(_ point: NSPoint) -> NSView? {
        frame.contains(point) ? self : nil
    }

    override func mouseUp(with event: NSEvent) {
        guard let item = enclosingMenuItem, item.isEnabled, let menu = item.menu else { return }
        let index = menu.index(of: item)
        menu.cancelTracking()
        menu.performActionForItem(at: index)
    }
}

/// Highlights the kind row under the pointer or the keyboard (Qt's `highlighted`).
@MainActor
private final class KindRowHighlighter: NSObject, NSMenuDelegate {
    private let rows: [KindMenuItemView]

    init(_ rows: [KindMenuItemView]) {
        self.rows = rows
    }

    func menu(_ menu: NSMenu, willHighlight item: NSMenuItem?) {
        for row in rows {
            row.state.highlighted = row.enclosingMenuItem === item && item != nil
        }
    }

    func menuDidClose(_ menu: NSMenu) {
        for row in rows {
            row.state.highlighted = false
        }
    }
}

/// Runs a closure when its menu item is chosen (on the main thread, as AppKit does).
@MainActor
private final class MenuTarget: NSObject {
    let action: () -> Void

    init(_ action: @escaping () -> Void) {
        self.action = action
    }

    @objc func run(_ sender: Any?) {
        action()
    }
}

/// The same entries as SwiftUI menu content, for `.contextMenu`.
struct SourceMenuItems: View {
    let entries: [SourceMenuEntry]

    var body: some View {
        ForEach(Array(entries.enumerated()), id: \.offset) { _, entry in
            switch entry {
            case .item(let title, let symbol, let enabled, let danger, _, let action):
                Button(role: danger ? .destructive : nil, action: action) {
                    if let symbol {
                        Label(title, systemImage: symbol)
                    } else {
                        Text(title)
                    }
                }
                .disabled(!enabled)
            case .kind(let kind, let action):
                Button(action: action) {
                    Label(kind.later ? "\(kind.name) (\(tr("later")))" : kind.name, systemImage: kind.symbol)
                }
                .disabled(kind.later)
            case .separator:
                Divider()
            case .note(let text):
                Text(text)
            }
        }
    }
}

/// Where a menu pops up from: an empty `NSView` behind a SwiftUI button.
///
///     @State private var anchor = SourceMenuAnchor()
///     Button { SourceMenus.popUp(entries, under: anchor.view) } label: { … }
///         .background(SourceMenuAnchorView(anchor: anchor))
@MainActor
final class SourceMenuAnchor {
    weak var view: NSView?
}

struct SourceMenuAnchorView: NSViewRepresentable {
    let anchor: SourceMenuAnchor

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        anchor.view = view
        return view
    }

    func updateNSView(_ view: NSView, context: Context) {
        anchor.view = view
    }
}

extension SourceKind {
    /// The Add source button's menu: every kind, those not available disabled with "Later".
    @MainActor
    static func menuEntries(choose: @escaping (SourceKind) -> Void) -> [SourceMenuEntry] {
        allCases.map { kind in .kind(kind) { choose(kind) } }
    }
}

/// The Add source button (Qt `AddSourceButton.qml`): "Add source" (13 DemiBold) after a
/// 14-point plus, 6 apart, padding 12; a pill with the `input` fill and a 1-point accent border
/// (38 points tall on the Sources page); hover `raised`, pressed or while its menu shows
/// `selected`. It opens the kinds menu, 300 points wide, right-aligned 8 points below it.
/// `request` going up opens the menu too (File › Add Source…, the empty state).
struct AddSourceButton: View {
    var height: CGFloat = 38
    var request: Int = 0
    let choose: (SourceKind) -> Void

    @State private var anchor = SourceMenuAnchor()
    @State private var open = false

    var body: some View {
        Button(action: openMenu) {
            HStack(spacing: 6) {
                Image(systemName: "plus")
                    .font(.system(size: 13, weight: .semibold))
                Text(tr("add_source"))
                    .font(.system(size: 13, weight: .semibold))
            }
            .foregroundStyle(Theme.text)
            .padding(.horizontal, 12)
            .frame(height: height)
            .contentShape(Capsule())
        }
        .buttonStyle(AddSourceButtonStyle(open: open))
        .background(SourceMenuAnchorView(anchor: anchor))
        .accessibilityLabel(tr("add_source"))
        .accessibilityAddTraits(.isButton)
        .onChange(of: request) { _, _ in
            // After the page arrived, so the menu opens under a button on screen. `popUp`
            // hands the menu to the run loop: it never tracks inside this task.
            Task { @MainActor in
                try? await Task.sleep(for: .milliseconds(250))
                openMenu()
            }
        }
    }

    private func openMenu() {
        open = true
        SourceMenus.popUp(SourceKind.menuEntries(choose: choose), under: anchor.view, minimumWidth: 300, gap: 8) {
            open = false
        }
    }
}

private struct AddSourceButtonStyle: ButtonStyle {
    let open: Bool
    @State private var hovering = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(Capsule().fill(configuration.isPressed || open ? Theme.selected : hovering ? Theme.raised : Theme.input))
            .overlay(PillOutline().strokeBorder(Theme.accent, lineWidth: 1))
            .onHover { hovering = $0 }
            .animation(Motion.standard(Motion.fade), value: hovering)
            .animation(Motion.standard(Motion.fade), value: open)
    }
}

/// Qt's MenuButton look (the Sources page's sort): no fill at rest, `hover` on hover,
/// `selected` while pressed or while its menu shows; radius 8.
struct SourceMenuButtonStyle: ButtonStyle {
    let open: Bool
    @Environment(\.isEnabled) private var enabled
    @State private var hovering = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(configuration.isPressed || open ? Theme.selected : hovering ? Theme.hover : .clear)
            )
            .opacity(enabled ? 1 : 0.45)
            .onHover { hovering = $0 && enabled }
            .animation(Motion.standard(Motion.fade), value: hovering)
            .animation(Motion.standard(Motion.fade), value: open)
    }
}

#if DEBUG
extension SourceMenus {
    /// Draws the kinds menu's rows (the first highlighted) on the menu's ground into a PNG at
    /// `path`, for checking them without popping the menu up; logs the rows' heights.
    static func renderKindRows(to path: String) {
        let menu = menu(SourceKind.menuEntries { _ in }, minimumWidth: 300)
        let rows = menu.items.compactMap { $0.view as? KindMenuItemView }
        NSLog("N Music kinds rows: %@", rows.map { "\($0.frame.height)" }.joined(separator: " "))
        let states = SourceKind.allCases.map { _ in KindRowState() }
        states[0].highlighted = true
        let list = VStack(spacing: 0) {
            ForEach(Array(SourceKind.allCases.enumerated()), id: \.offset) { index, kind in
                KindMenuRow(kind: kind, state: states[index], width: 300)
            }
        }
        .padding(.vertical, 5)
        .background(Theme.menu)
        let host = NSHostingView(rootView: list)
        let size = host.fittingSize
        let window = NSWindow(contentRect: NSRect(origin: .zero, size: size), styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = host
        window.alphaValue = 0
        window.orderFrontRegardless()
        Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(800))
            host.layoutSubtreeIfNeeded()
            if let bitmap = host.bitmapImageRepForCachingDisplay(in: host.bounds) {
                host.cacheDisplay(in: host.bounds, to: bitmap)
                try? bitmap.representation(using: .png, properties: [:])?.write(to: URL(fileURLWithPath: path))
            }
            window.orderOut(nil)
        }
    }
}
#endif
