import AppKit
import SwiftUI

/// A button of any look that opens a native menu below it (SwiftUI's `Menu` flattens a custom
/// label on the Mac). `trailing` right-aligns the menu with the button, as Qt's sort menu.
///
///     MenuButton(trailing: true) { menu in
///         menu.addItem(TrackActionItem("Title") { … })
///     } label: { Text("Sort") }
struct MenuButton<Label: View>: View {
    var trailing = false
    /// Set while the menu is open (for a `selected` look).
    var isOpen: Binding<Bool>? = nil
    let fill: @MainActor (NSMenu) -> Void
    @ViewBuilder var label: () -> Label

    @State private var anchor = TrackMenuAnchor()

    var body: some View {
        Button {
            isOpen?.wrappedValue = true
            anchor.open(trailing: trailing, fill: fill)
            isOpen?.wrappedValue = false
        } label: {
            label()
        }
        .buttonStyle(.plain)
        .background(TrackMenuAnchorView(anchor: anchor))
    }
}

/// Where a `MenuButton`'s menu opens from: its button's view.
@MainActor
final class TrackMenuAnchor {
    weak var view: NSView?

    func open(trailing: Bool, fill: (NSMenu) -> Void) {
        guard let view else { return }
        let menu = NSMenu()
        menu.autoenablesItems = false
        fill(menu)
        guard menu.numberOfItems > 0 else { return }
        let width = menu.size.width
        let x = trailing ? view.bounds.maxX - width : view.bounds.minX
        let y = view.isFlipped ? view.bounds.maxY + 4 : view.bounds.minY - 4
        menu.popUp(positioning: nil, at: NSPoint(x: x, y: y), in: view)
    }
}

private struct TrackMenuAnchorView: NSViewRepresentable {
    let anchor: TrackMenuAnchor

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        anchor.view = view
        return view
    }

    func updateNSView(_ view: NSView, context: Context) {
        anchor.view = view
    }
}

extension NSMenu {
    /// Adds a choice, checked when it is the current one.
    @discardableResult
    func addChoice(_ title: String, checked: Bool, enabled: Bool = true, action: @escaping @MainActor () -> Void) -> NSMenuItem {
        let item = TrackActionItem(title, handler: action)
        item.state = checked ? .on : .off
        item.isEnabled = enabled
        addItem(item)
        return item
    }

    /// Adds a heading: small capitals, not selectable.
    func addHeading(_ title: String) {
        addItem(NSMenuItem.sectionHeader(title: title.uppercased()))
    }
}
