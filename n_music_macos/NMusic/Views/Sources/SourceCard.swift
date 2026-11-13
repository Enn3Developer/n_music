import AppKit
import SwiftUI

/// A source as a card (Qt `SourceCard.qml`, spec §5.4): a square cover (the kind's icon without
/// one), dimmed to 50 % when it can't be reached, a status pill while it isn't up to date, its
/// name, its tracks, and a "⋯" menu (also on right-click, and Shift+F10 when focused). The whole
/// card is one button opening its tracks: hovering anywhere on it veils the cover, pressing it
/// dims and shrinks the cover; hovering shows where it is.
struct SourceCard: View {
    let source: SourceInfo
    let width: CGFloat
    /// The Sources page is the one showing: the Updating… pill only turns then.
    var shown = true
    let rename: () -> Void
    let remove: () -> Void

    private let navigation = Navigation.shared
    @State private var anchor = SourceMenuAnchor()
    @State private var hovering = false

    var body: some View {
        let kind = SourceKind(source.kind)
        let entries = Self.menuEntries(source, rename: rename, remove: remove)
        Button { navigation.go(PageID.source(source.root)) } label: {
            VStack(alignment: .leading, spacing: 10) {
                SourceCover(source: source, kind: kind, width: width, hovering: hovering)
                    .overlay(alignment: .bottomLeading) {
                        // Over the cover, not dimmed with it.
                        if let status {
                            StatusPill(
                                text: status,
                                available: source.available,
                                spinning: source.updating && source.available && shown
                            )
                            .frame(maxWidth: width - 16, alignment: .leading)
                            .padding(8)
                        }
                    }
                VStack(alignment: .leading, spacing: 2) {
                    Text(source.name)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(Theme.text)
                        .lineLimit(1)
                    Text(Format.count(source.tracks, "track_one", "tracks_many"))
                        .font(.system(size: 13).monospacedDigit())
                        .foregroundStyle(Theme.text2)
                        .lineLimit(1)
                }
                // Room for the "⋯" laid over the card.
                .padding(.trailing, 36)
                .frame(maxWidth: .infinity, minHeight: 32, alignment: .leading)
            }
            .frame(width: width)
            .contentShape(Rectangle())
        }
        .buttonStyle(SourceCardStyle())
        .help(source.location)
        .accessibilityLabel(status.map { "\(source.name), \($0)" } ?? source.name)
        .overlay(alignment: .bottomTrailing) {
            IconButton(symbol: "ellipsis", label: tr("source_actions", source.name), size: 32, iconSize: 16, tint: Theme.text3) {
                openMenu(entries)
            }
            .background(SourceMenuAnchorView(anchor: anchor))
        }
        // Over the "⋯" too (Qt's card holds it).
        .onHover { hovering = $0 }
        // The menu key of other keyboards; Shift+F10 here.
        .onKeyPress(KeyEquivalent(Character(UnicodeScalar(NSF10FunctionKey)!)), phases: .down) { press in
            guard press.modifiers.contains(.shift) else { return .ignored }
            openMenu(entries)
            return .handled
        }
        .contextMenu { SourceMenuItems(entries: entries) }
    }

    private func openMenu(_ entries: [SourceMenuEntry]) {
        SourceMenus.popUp(entries, under: anchor.view)
    }

    /// What is wrong with it or happening to it; `nil` while it is up to date.
    private var status: String? {
        switch source.status {
        case .ready: nil
        case .updating: tr("source_updating")
        case .missing: tr("source_missing")
        case .unreachable: tr("source_unreachable")
        case .signedOut: tr("source_signed_out")
        }
    }

    /// The Qt SourceMenu: Rename, Update now, Reload metadata, Open folder (folders only),
    /// —, Remove. The Collection page's source "⋯" uses the same entries.
    @MainActor
    static func menuEntries(_ source: SourceInfo, rename: @escaping () -> Void, remove: @escaping () -> Void) -> [SourceMenuEntry] {
        let store = Store.shared
        let root = source.root
        var entries: [SourceMenuEntry] = [
            .item(title: tr("rename"), symbol: "pencil", action: rename),
            .item(title: tr("update_now"), symbol: "arrow.clockwise", enabled: !source.updating) {
                store.scan(root, checkCache: true)
            },
            .item(title: tr("reload_metadata"), symbol: "tag", enabled: !source.updating) {
                store.scan(root, checkCache: false)
            },
        ]
        if source.kind == .folder {
            entries.append(.item(title: tr("open_folder"), symbol: "folder", enabled: source.available) {
                if let url = root.fileURL { NSWorkspace.shared.open(url) }
            })
        }
        entries.append(.separator)
        entries.append(.item(title: tr("remove"), symbol: "trash", danger: true, action: remove))
        return entries
    }
}

/// The card's press: the cover dims to 0.86 and shrinks to 0.97 (the label reads it).
private struct SourceCardStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .environment(\.sourceCardPressed, configuration.isPressed)
    }
}

private extension EnvironmentValues {
    @Entry var sourceCardPressed = false
}

/// The cover: a 6 % white veil while the card is hovered, 0.86 opacity and 0.97 scale while it
/// is pressed; 50 % opacity (animated) when the source can't be reached.
private struct SourceCover: View {
    let source: SourceInfo
    let kind: SourceKind
    let width: CGFloat
    let hovering: Bool

    @Environment(\.sourceCardPressed) private var pressed

    var body: some View {
        let available = source.available
        CoverView(path: source.cover, size: width, radius: 8, placeholder: kind.symbol)
            .overlay(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(Color.white.opacity(hovering ? 0.06 : 0))
            )
            .opacity(pressed ? 0.86 : available ? 1 : 0.5)
            .scaleEffect(pressed ? 0.97 : 1)
            .animation(Motion.standard(Motion.move), value: pressed)
            .animation(Motion.standard(available ? Motion.fade : Motion.exit), value: available)
            .animation(Motion.standard(Motion.fade), value: hovering)
    }
}

/// The status pill at the cover's bottom-left: 24 points, `menu` fill, an alert icon in red for
/// problems, or the accent refresh icon turning once in 1.4 s while it updates and the page
/// shows (held still under reduced motion, which is followed live).
private struct StatusPill: View {
    let text: String
    let available: Bool
    let spinning: Bool

    @State private var turned = false
    private let motion = Motion.shared

    private var turning: Bool { spinning && !motion.reduced }

    var body: some View {
        HStack(spacing: 5) {
            if available {
                Image(systemName: "arrow.clockwise")
                    .font(.system(size: 10, weight: .bold))
                    .foregroundStyle(Theme.accentText)
                    .rotationEffect(.degrees(turned ? 360 : 0))
                    .onAppear(perform: spin)
                    .onChange(of: turning) { spin() }
            } else {
                Image(systemName: "exclamationmark.circle")
                    .font(.system(size: 11, weight: .bold))
                    .foregroundStyle(Theme.danger)
            }
            Text(text)
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(available ? Theme.accentText : Theme.danger)
                .lineLimit(1)
                .truncationMode(.tail)
        }
        .padding(.leading, 8)
        .padding(.trailing, 9)
        .frame(height: 24)
        .background(Capsule().fill(Theme.menu))
        .overlay(PillOutline().strokeBorder(Theme.line2, lineWidth: 1))
        .fixedSize(horizontal: false, vertical: true)
    }

    private func spin() {
        guard turning else {
            var still = Transaction()
            still.disablesAnimations = true
            withTransaction(still) { turned = false }
            return
        }
        turned = false
        withAnimation(.linear(duration: 1.4).repeatForever(autoreverses: false)) { turned = true }
    }
}

/// How far the running scan got (Qt `ScanCard.qml`): "Updating library", the tracks read of
/// those found (or "Looking for music…" before any), a 6-point bar of read / found, and the
/// hint that found tracks play already.
struct SourcesScanCard: View {
    let scan: ScanState

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 10) {
                Image(systemName: "arrow.clockwise")
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(Theme.accentText)
                Text(tr("updating_library"))
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(Theme.text)
                    .lineLimit(1)
                Spacer(minLength: 8)
                Text(scan.found > 0
                    ? tr("tracks_read", Format.number(scan.read), Format.number(scan.found))
                    : tr("scanning_library"))
                    .font(.system(size: 14).monospacedDigit())
                    .foregroundStyle(Theme.text2)
            }
            GeometryReader { geometry in
                ZStack(alignment: .leading) {
                    Capsule().fill(Theme.line2)
                    Capsule().fill(Theme.accent).frame(width: geometry.size.width * scan.progress)
                }
            }
            .frame(height: 6)
            // Wraps without `fixedSize`: probed at a tiny width (pages hidden in the shell's
            // ZStack are), a fixed-height text grows tall enough to push the window apart.
            Text(tr("updating_library_hint"))
                .font(.system(size: 13))
                .foregroundStyle(Theme.text3)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 16)
        .background(RoundedRectangle(cornerRadius: 12, style: .continuous).fill(Theme.surface))
        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(Theme.line2, lineWidth: 1))
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(tr("updating_library"))
        .accessibilityValue(scan.found > 0 ? tr("tracks_read", Format.number(scan.read), Format.number(scan.found)) : tr("scanning_library"))
        .accessibilityAddTraits(.updatesFrequently)
    }
}
