import SwiftUI

/// One choice of a sort menu: its label key and the sort it writes.
struct SortOption: Identifiable, Hashable, Sendable {
    let key: String
    let sort: String
    var id: String { sort }

    /// The track lists' choices (`qml/SortButton.qml:16-53`).
    static let tracks: [SortOption] = [
        SortOption(key: "sort_artist_album", sort: "artist,album"),
        SortOption(key: "sort_title", sort: "title"),
        SortOption(key: "sort_album", sort: "album"),
        SortOption(key: "sort_newest", sort: "-year,album"),
        SortOption(key: "sort_oldest", sort: "year,album"),
        SortOption(key: "sort_most_played", sort: "-plays,title"),
        SortOption(key: "sort_recently_played", sort: "-lastPlayed"),
        SortOption(key: "sort_length", sort: "length"),
        SortOption(key: "sort_location", sort: "location"),
    ]

    /// A regular playlist's two more, first (`qml/PlaylistPage.qml:280-292`).
    static let playlist: [SortOption] = [
        SortOption(key: "sort_added_newest", sort: "-added"),
        SortOption(key: "sort_added_oldest", sort: "added"),
    ] + tracks
}

/// The Qt SortButton (spec §3.1): the sort icon, the label (the matching choice's name, else the
/// keys in short words with arrows, like "Year ↑, Album ↑") and a chevron; icon-only as a 36-point
/// bordered square. The menu checks the choice matching exactly.
struct SortButton: View {
    @Binding var sort: String
    var options: [SortOption] = SortOption.tracks
    var iconOnly = false

    @State private var hovering = false
    @State private var open = false

    var body: some View {
        let label = Self.label(sort, options: options)
        // `selected` while pressed or open, `hover` under the pointer (spec §3.1).
        let ground = open ? Theme.selected : hovering ? Theme.hover : Color.clear
        MenuButton(trailing: true, isOpen: $open) { menu in
            for option in options {
                menu.addChoice(tr(option.key), checked: option.sort == sort) { sort = option.sort }
            }
        } label: {
            if iconOnly {
                Image(systemName: "arrow.up.arrow.down")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(Theme.text2)
                    .frame(width: 36, height: 36)
                    .background(RoundedRectangle(cornerRadius: 9, style: .continuous).fill(ground))
                    .overlay(RoundedRectangle(cornerRadius: 9, style: .continuous).strokeBorder(Theme.line2, lineWidth: 1))
                    .contentShape(Rectangle())
            } else {
                HStack(spacing: 6) {
                    Image(systemName: "arrow.up.arrow.down")
                        .font(.system(size: 12, weight: .medium))
                    Text(label)
                        .font(.system(size: 13, weight: .medium))
                        .lineLimit(1)
                        .truncationMode(.tail)
                        .frame(maxWidth: 220, alignment: .leading)
                    Image(systemName: "chevron.down")
                        .font(.system(size: 10, weight: .semibold))
                }
                .foregroundStyle(Theme.text2)
                .padding(.horizontal, 12)
                .frame(height: 32)
                .background(RoundedRectangle(cornerRadius: 8, style: .continuous).fill(ground))
                .contentShape(Rectangle())
            }
        }
        .fixedSize()
        .onHover { hovering = $0 }
        .help(tr("sort_by") + ": " + label)
        .accessibilityLabel(tr("sort_by") + ": " + label)
    }

    /// The choice matching `sort` exactly, else its keys in short words.
    static func label(_ sort: String, options: [SortOption]) -> String {
        if let option = options.first(where: { $0.sort == sort }) {
            return tr(option.key)
        }
        if sort == "-added" { return tr("sort_added_newest") }
        if sort == "added" { return tr("sort_added_oldest") }
        return Filters.sortLabel(sort)
    }
}
