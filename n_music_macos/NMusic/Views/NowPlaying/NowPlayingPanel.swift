import SwiftUI

/// The 360-point Now Playing panel on the right of wide windows (≥ 1600), except on the queue
/// page (spec §4.3). The shell gives it its width, the `panel` ground and the left line.
///
/// Top to bottom, 20/22 margins, 18 apart: the cover (min(panel − 40, max(120, height − 400)),
/// sharpened to the file's picture), the title (20 bold, 2 lines), "artist · album · year", the
/// audio line (format · ReplayGain), "Up next" with "Open queue", and what plays next
/// (double-click plays), or "Nothing plays after this."
struct NowPlayingPanel: View {
    private let store = Store.shared
    private let model = QueueModel.shared
    private let facts = CurrentFacts.shared
    private let settings = UISettings.shared

    private struct Words: Equatable {
        let track: TrackRow?
        let audio: String
    }

    var body: some View {
        GeometryReader { geometry in
            let cover = min(geometry.size.width - 40, max(120, geometry.size.height - 400))
            VStack(alignment: .leading, spacing: 18) {
                SwapCover(size: cover, radius: 12, full: true)
                    .frame(maxWidth: .infinity)
                let audio = [facts.audio, facts.gain].filter { !$0.isEmpty }.joined(separator: " · ")
                SwapWords(Words(track: store.current?.track, audio: audio)) { words in
                    VStack(alignment: .leading, spacing: 4) {
                        if let track = words.track {
                            Text(track.title)
                                .font(.system(size: 20, weight: .bold))
                                .foregroundStyle(Theme.text)
                                .lineLimit(2)
                            let second = track.artistAlbumYear
                            if !second.isEmpty {
                                Text(second)
                                    .font(.system(size: 14))
                                    .foregroundStyle(Theme.text2)
                                    .lineLimit(1)
                            }
                            if !words.audio.isEmpty {
                                Text(words.audio)
                                    .font(.system(size: 12))
                                    .foregroundStyle(Theme.text3)
                                    .lineLimit(1)
                                    .padding(.top, 4)
                            }
                        } else {
                            Text(tr("not_playing"))
                                .font(.system(size: 20, weight: .bold))
                                .foregroundStyle(Theme.text3)
                        }
                    }
                    .truncationMode(.tail)
                    .fixedSize(horizontal: false, vertical: true)
                }
                HStack {
                    Text(tr("up_next"))
                        .font(.system(size: 15, weight: .bold))
                        .foregroundStyle(Theme.text)
                        .accessibilityAddTraits(.isHeader)
                    Spacer(minLength: 8)
                    UnderlinedLink(title: tr("open_queue")) {
                        Navigation.shared.go(PageID.queue)
                    }
                    .frame(height: 20)
                }
                ZStack(alignment: .top) {
                    QueueTable(mode: .panel, rows: model.panel?.rows ?? [], compact: settings.compactRows)
                    if let panel = model.panel, panel.rows.isEmpty {
                        Text(tr("nothing_up_next"))
                            .font(.system(size: 13))
                            .foregroundStyle(Theme.text3)
                            .padding(.top, 12)
                    }
                }
                .padding(.horizontal, -6)
                .frame(maxHeight: .infinity)
            }
            .padding(.horizontal, 20)
            .padding(.top, 22)
            .frame(width: geometry.size.width, height: geometry.size.height, alignment: .top)
        }
        .onAppear { model.start() }
        .task(id: "\(store.current?.track.locator.key ?? "")\u{1}\(store.libraryVersion)") {
            facts.refresh()
        }
    }
}
