import AppKit
import SwiftUI

// The settings' select box and segmented control as the AppKit controls themselves, so they
// fill the width the Qt app gives them (SelectBox 140/220/300, SegmentedControl 300, ReplayGain
// up to 420): SwiftUI's own Picker keeps its intrinsic width on macOS whatever it is offered.
// Both follow the app's appearance (NSApp.appearance), take keyboard focus with its focus ring
// and are the native pop-up button / radio group for VoiceOver.

/// One choice of a settings control: its value and what it shows.
struct SettingsOption<Value: Hashable>: Hashable {
    let value: Value
    let title: String
}

/// A native pop-up button as wide as it is offered (Qt's SelectBox). A `selection` none of the
/// options has shows the box blank, as Qt's box does. `onOpen` runs each time its menu opens.
struct SettingsPopUp<Value: Hashable>: NSViewRepresentable {
    let label: String
    let options: [SettingsOption<Value>]
    let selection: Value?
    let onSelect: (Value) -> Void
    var onOpen: (() -> Void)? = nil

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeNSView(context: Context) -> NSPopUpButton {
        let button = NSPopUpButton(frame: .zero, pullsDown: false)
        button.controlSize = .large
        button.target = context.coordinator
        button.action = #selector(Coordinator.picked(_:))
        button.menu?.delegate = context.coordinator
        button.setContentHuggingPriority(.defaultLow, for: .horizontal)
        button.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        return button
    }

    func updateNSView(_ button: NSPopUpButton, context: Context) {
        let coordinator = context.coordinator
        coordinator.parent = self
        button.setAccessibilityLabel(label)
        if coordinator.shown != options.map(\.title) {
            button.removeAllItems()
            for (index, option) in options.enumerated() {
                button.addItem(withTitle: "")
                let item = button.lastItem
                // Titles may repeat (two devices of one name): set each one's own.
                item?.title = option.title
                item?.tag = index
            }
            coordinator.shown = options.map(\.title)
        }
        if let selection, let index = options.firstIndex(where: { $0.value == selection }) {
            if button.indexOfSelectedItem != index { button.selectItem(at: index) }
        } else {
            button.select(nil)
        }
    }

    func sizeThatFits(_ proposal: ProposedViewSize, nsView: NSPopUpButton, context: Context) -> CGSize? {
        let fitting = nsView.intrinsicContentSize
        return CGSize(width: proposal.width ?? fitting.width, height: fitting.height)
    }

    @MainActor
    final class Coordinator: NSObject, NSMenuDelegate {
        var parent: SettingsPopUp?
        var shown: [String]?

        @objc func picked(_ sender: NSPopUpButton) {
            guard let parent, sender.indexOfSelectedItem >= 0, sender.indexOfSelectedItem < parent.options.count else { return }
            parent.onSelect(parent.options[sender.indexOfSelectedItem].value)
        }

        // The menu opens on the main thread, from the button's own tracking.
        nonisolated func menuWillOpen(_ menu: NSMenu) {
            MainActor.assumeIsolated { parent?.onOpen?() }
        }
    }
}

/// A native segmented control whose options share the width it is offered equally (Qt's
/// SegmentedControl); a `selection` none of the options has selects none.
struct SettingsSegmented<Value: Hashable>: NSViewRepresentable {
    let label: String
    let options: [SettingsOption<Value>]
    let selection: Value?
    var large = false
    let onSelect: (Value) -> Void

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeNSView(context: Context) -> NSSegmentedControl {
        let control = NSSegmentedControl(
            labels: options.map(\.title),
            trackingMode: .selectOne,
            target: context.coordinator,
            action: #selector(Coordinator.picked(_:))
        )
        control.segmentDistribution = .fillEqually
        // Qt's picked option sits on a neutral `line2` ground, never on the accent.
        control.selectedSegmentBezelColor = Theme.dynamic(dark: Theme.rgb(0x2A2E35), light: Theme.rgb(0xD3D8DF))
        control.controlSize = large ? .large : .regular
        control.setContentHuggingPriority(.defaultLow, for: .horizontal)
        control.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        return control
    }

    func updateNSView(_ control: NSSegmentedControl, context: Context) {
        context.coordinator.parent = self
        control.setAccessibilityLabel(label)
        if control.segmentCount != options.count { control.segmentCount = options.count }
        for (index, option) in options.enumerated() where control.label(forSegment: index) != option.title {
            control.setLabel(option.title, forSegment: index)
        }
        let index = selection.flatMap { selection in options.firstIndex { $0.value == selection } } ?? -1
        if control.selectedSegment != index { control.selectedSegment = index }
    }

    func sizeThatFits(_ proposal: ProposedViewSize, nsView: NSSegmentedControl, context: Context) -> CGSize? {
        let fitting = nsView.intrinsicContentSize
        return CGSize(width: max(proposal.width ?? fitting.width, 0), height: fitting.height)
    }

    @MainActor
    final class Coordinator: NSObject {
        var parent: SettingsSegmented?

        @objc func picked(_ sender: NSSegmentedControl) {
            guard let parent, sender.selectedSegment >= 0, sender.selectedSegment < parent.options.count else { return }
            parent.onSelect(parent.options[sender.selectedSegment].value)
        }
    }
}

/// A plain button that draws Qt's 2-point `text` ring while it has keyboard focus
/// (`visualFocus`) instead of the system's ring, and shrinks to `pressScale` while pressed.
struct SettingsButtonStyle<S: InsettableShape>: ButtonStyle {
    let shape: S
    var pressScale: CGFloat = 1

    func makeBody(configuration: Configuration) -> some View {
        Ring(shape: shape, label: configuration.label)
            .scaleEffect(configuration.isPressed ? pressScale : 1)
            .animation(Motion.standard(Motion.move), value: configuration.isPressed)
    }

    private struct Ring<Label: View>: View {
        let shape: S
        let label: Label
        @Environment(\.isFocused) private var focused

        var body: some View {
            label.overlay(shape.strokeBorder(Theme.text, lineWidth: 2).opacity(focused ? 1 : 0))
        }
    }
}

extension Button {
    /// `SettingsButtonStyle`, the system's focus ring off.
    func settingsStyle<S: InsettableShape>(_ shape: S, pressScale: CGFloat = 1) -> some View {
        buttonStyle(SettingsButtonStyle(shape: shape, pressScale: pressScale)).focusEffectDisabled()
    }
}
