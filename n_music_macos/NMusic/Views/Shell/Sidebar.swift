import SwiftUI

/// The Qt Sidebar (spec §2.5), in the native collapsible sidebar column: the logo, "N Music"
/// and back/forward; LIBRARY; PLAYLISTS with "+" and the list; at the bottom Queue, Settings and
/// the scan card while a scan runs. A pill under each group's active entry slides to the next.
struct Sidebar: View {
    private let store = Store.shared
    private let navigation = Navigation.shared
    @Namespace private var pills

    var body: some View {
        VStack(alignment: .leading, spacing: 22) {
            header
            library
            playlists
            bottom
        }
        .padding(.top, 18)
        .padding(.leading, 12)
        .padding(.trailing, 13)
        .padding(.bottom, 14)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .animation(Motion.shared.reduced ? nil : Motion.standard(Motion.move), value: navigation.page)
    }

    private var header: some View {
        HStack(spacing: 10) {
            LogoView(size: 28, radius: 8)
            Text(verbatim: "N Music")
                .font(.system(size: 17, weight: .bold))
                .foregroundStyle(Theme.text)
                .lineLimit(1)
            Spacer(minLength: 0)
            HistoryButtons()
        }
        .padding(.leading, 10)
    }

    private var library: some View {
        VStack(alignment: .leading, spacing: 2) {
            SectionHeader(text: tr("library"))
                .padding(.leading, 12)
                .padding(.bottom, 6)
            item("tracks", "music.note", active: section == "tracks", group: "library")
            item("albums", "opticaldisc", active: section == "albums" || section == "album", group: "library")
            item("artists", "person", active: section == "artists" || section == "artist", group: "library")
            item("genres", "tag", active: section == "genres" || section == "genre", group: "library")
            item("sources", "folder", active: section == "sources" || section == "source", group: "library")
        }
    }

    private var playlists: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack {
                SectionHeader(text: tr("playlists"))
                Spacer()
                IconButton(symbol: "plus", label: tr("new_playlist"), size: 28, iconSize: 16, tint: Theme.text2) {
                    navigation.requestNewPlaylist()
                }
            }
            .padding(.leading, 12)
            .padding(.bottom, 2)
            ScrollView {
                VStack(alignment: .leading, spacing: 2) {
                    ForEach(store.playlists) { playlist in
                        NavItem(
                            title: playlist.name,
                            symbol: playlist.smart ? "line.3.horizontal.decrease" : "music.note.list",
                            active: navigation.page == playlist.page,
                            pill: pill("playlists", active: navigation.page == playlist.page)
                        ) {
                            navigation.go(playlist.page)
                        }
                    }
                }
            }
            .scrollIndicators(.automatic)
        }
        .frame(maxHeight: .infinity, alignment: .top)
    }

    private var bottom: some View {
        VStack(alignment: .leading, spacing: 2) {
            Rectangle().fill(Theme.line).frame(height: 1).padding(.bottom, 8)
            item("queue", "text.line.first.and.arrowtriangle.forward", active: section == "queue", group: "bottom")
            item("settings", "slider.horizontal.3", active: section == "settings", group: "bottom")
            if let scan = store.scan {
                ScanCard(scan: scan)
                    .padding(.top, 8)
                    .transition(.opacity)
            }
        }
    }

    private var section: String { navigation.section }

    private func item(_ page: String, _ symbol: String, active: Bool, group: String) -> some View {
        NavItem(title: tr(page), symbol: symbol, active: active, pill: pill(group, active: active)) {
            navigation.go(page)
        }
    }

    @ViewBuilder
    private func pill(_ group: String, active: Bool) -> some View {
        if active {
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(Theme.raised)
                .matchedGeometryEffect(id: group, in: pills)
                .transition(.opacity)
        }
    }
}

/// A sidebar entry: 36 points tall, a 20-point icon, 12 points, the label at 14 Medium.
private struct NavItem<Pill: View>: View {
    let title: String
    let symbol: String
    let active: Bool
    let pill: Pill
    let action: () -> Void

    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Image(systemName: symbol)
                    .font(.system(size: 15, weight: .medium))
                    .frame(width: 20, height: 20)
                    .foregroundStyle(active ? Theme.accentText : hovering ? Theme.text : Theme.text2)
                Text(title)
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(active || hovering ? Theme.text : Theme.text2)
                    .lineLimit(1)
                    .truncationMode(.tail)
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 12)
            .frame(height: 36)
            .contentShape(Rectangle())
            .background {
                ZStack {
                    if hovering && !active {
                        RoundedRectangle(cornerRadius: 8, style: .continuous).fill(Theme.hover)
                    }
                    pill
                }
            }
        }
        .buttonStyle(NavPressStyle())
        .onHover { hovering = $0 }
        .accessibilityAddTraits(active ? .isSelected : [])
    }
}

/// Pressing shrinks the icon row a little, as the Qt PressScale.
private struct NavPressStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .opacity(configuration.isPressed ? 0.8 : 1)
    }
}

/// Back and forward through the history: 28-point buttons with 16-point chevrons, 2 apart;
/// disabled at 45 %.
struct HistoryButtons: View {
    private let navigation = Navigation.shared

    var body: some View {
        HStack(spacing: 2) {
            IconButton(symbol: "chevron.left", label: tr("go_back"), size: 28, iconSize: 16) { navigation.back() }
                .disabled(!navigation.canGoBack)
            IconButton(symbol: "chevron.right", label: tr("go_forward"), size: 28, iconSize: 16) { navigation.forward() }
                .disabled(!navigation.canGoForward)
        }
    }
}

/// "Updating library" with "read / found" and a progress bar, while a scan runs; opens Sources.
private struct ScanCard: View {
    let scan: ScanState
    @State private var hovering = false

    var body: some View {
        Button {
            Navigation.shared.go(PageID.sources)
        } label: {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text(tr("updating_library"))
                        .font(.system(size: 12))
                        .foregroundStyle(Theme.text)
                    Spacer(minLength: 4)
                    if scan.found > 0 {
                        Text(tr("read_of_found", Format.number(scan.read), Format.number(scan.found)))
                            .font(.system(size: 12).monospacedDigit())
                            .foregroundStyle(Theme.text2)
                    }
                }
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        Capsule().fill(Theme.line2)
                        Capsule().fill(Theme.accent).frame(width: geometry.size.width * scan.progress)
                    }
                }
                .frame(height: 4)
            }
            .padding(.vertical, 10)
            .padding(.horizontal, 12)
            .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(hovering ? Theme.raised : Theme.surface))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(tr("updating_library") + ", " + tr("read_of_found", Format.number(scan.read), Format.number(scan.found)))
        .accessibilityAddTraits(.isButton)
    }
}
