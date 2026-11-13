import SwiftUI

/// A choice of a `RuleSelectBox`, or a heading (small capitals, not selectable) above choices.
struct RuleSelectOption: Identifiable {
    let value: String
    let label: String
    var heading = false
    var id: String { heading ? "heading:" + label : value }

    static func heading(_ label: String) -> RuleSelectOption {
        RuleSelectOption(value: "", label: label, heading: true)
    }
}

/// The Qt RuleSelectBox: a box showing the choice (or `display`), opening a native menu of the
/// choices with the current one checked. `muted` draws it without a fill, for conditions and
/// directions.
struct RuleSelectBox: View {
    let options: [RuleSelectOption]
    @Binding var value: String
    /// What the box shows instead of the choice's label (the field's short name).
    var display: String? = nil
    var muted = false
    var height: CGFloat = 36
    var radius: CGFloat = 8
    var weight: Font.Weight = .regular
    var fontSize: CGFloat = 14
    let label: String

    @State private var hovering = false

    var body: some View {
        let shown = display ?? options.first { !$0.heading && $0.value == value }?.label ?? ""
        MenuButton { menu in
            for option in options {
                if option.heading {
                    menu.addHeading(option.label)
                } else {
                    menu.addChoice(option.label, checked: option.value == value) { value = option.value }
                }
            }
        } label: {
            HStack(spacing: 6) {
                Text(shown)
                    .font(.system(size: fontSize, weight: weight))
                    .foregroundStyle(muted ? Theme.text2 : Theme.text)
                    .lineLimit(1)
                    .truncationMode(.tail)
                Spacer(minLength: 0)
                Image(systemName: "chevron.up.chevron.down")
                    .font(.system(size: 9, weight: .semibold))
                    .foregroundStyle(Theme.text3)
            }
            .padding(.horizontal, height < 32 ? 8 : 10)
            .frame(height: height)
            .background(
                RoundedRectangle(cornerRadius: radius, style: .continuous)
                    .fill(muted ? (hovering ? Theme.hover : Color.clear) : Theme.input)
            )
            .overlay(
                RoundedRectangle(cornerRadius: radius, style: .continuous)
                    .strokeBorder(muted ? Theme.line2 : hovering ? Theme.text3 : Theme.border, lineWidth: 1)
            )
            .contentShape(Rectangle())
        }
        .onHover { hovering = $0 }
        .help(label)
        .accessibilityLabel(label)
        .accessibilityValue(shown)
    }
}

/// The dashed "+ Add rule" / "+ Add group" buttons.
struct RuleDashedButton: View {
    let title: String
    let action: () -> Void
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            Label(title, systemImage: "plus")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Theme.text2)
                .padding(.horizontal, 12)
                .frame(height: 32)
                .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(hovering ? Theme.hover : .clear))
                .overlay(
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .strokeBorder(Theme.border, style: StrokeStyle(lineWidth: 1, dash: [4, 3]))
                )
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
    }
}

/// A flat button with a plus: "+ Then by…", "+ Add rule to group".
struct RuleFlatButton: View {
    let title: String
    var symbol = "plus"
    let action: () -> Void
    @Environment(\.isEnabled) private var enabled
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            Label(title, systemImage: symbol)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Theme.text2)
                .padding(.horizontal, 10)
                .frame(height: 30)
                .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(hovering ? Theme.hover : .clear))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .opacity(enabled ? 1 : 0.45)
        .onHover { hovering = $0 && enabled }
    }
}

/// A small "×" in `text3`.
struct RuleRemoveButton: View {
    let label: String
    let action: () -> Void

    var body: some View {
        IconButton(symbol: "xmark", label: label, size: 32, iconSize: 14, tint: Theme.text3, action: action)
    }
}
