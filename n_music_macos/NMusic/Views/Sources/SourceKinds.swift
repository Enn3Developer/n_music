import SwiftUI

/// The kinds of sources a library can have (Qt `SourceKinds.qml`, spec §5.3): the core reads
/// local folders, web playlists and Telegram chats, the last only in builds that sign in to
/// Telegram; the others are "Later".
enum SourceKind: String, CaseIterable, Identifiable, Sendable {
    case folder, web, telegram, spotify, youtube, deezer

    var id: String { rawValue }

    /// Like "Local folder".
    @MainActor var name: String {
        switch self {
        case .folder: tr("source_folder")
        case .web: tr("source_web")
        case .telegram: tr("source_telegram")
        case .spotify: "Spotify"
        case .youtube: "YouTube"
        case .deezer: "Deezer"
        }
    }

    /// The Sources page's section title, like "Local folders".
    @MainActor var group: String {
        switch self {
        case .folder: tr("local_folders")
        case .web: tr("web_playlists")
        case .telegram: tr("telegram_chats")
        case .spotify, .youtube, .deezer: name
        }
    }

    @MainActor var detail: String {
        switch self {
        case .folder: tr("source_folder_detail")
        case .web: tr("source_web_detail")
        case .telegram: tr("source_telegram_detail")
        case .spotify: tr("source_spotify_detail")
        case .youtube: tr("source_youtube_detail")
        case .deezer: tr("source_deezer_detail")
        }
    }

    /// The SF Symbol standing for Qt's kind icon (folder, link, send, cloud).
    var symbol: String {
        switch self {
        case .folder: "folder"
        case .web: "link"
        case .telegram: "paperplane"
        case .spotify, .youtube, .deezer: "cloud"
        }
    }

    /// Not available yet: Telegram without Telegram in this build, and the streaming services.
    @MainActor var later: Bool {
        switch self {
        case .folder, .web: false
        case .telegram: Store.shared.telegram == nil
        case .spotify, .youtube, .deezer: true
        }
    }

    init(_ type: SourceType) {
        switch type {
        case .folder: self = .folder
        case .web: self = .web
        case .telegram: self = .telegram
        }
    }

    /// The kind of `root`: a web playlist, a Telegram chat, else a folder (Qt `SourceKinds.of`).
    init(_ root: Locator) {
        switch root {
        case .web: self = .web
        case .telegramChat, .telegramAudio: self = .telegram
        case .local, .documentTree, .document: self = .folder
        }
    }
}

/// A small rounded tag beside a name (Qt `Badge.qml`): 20 points tall, 11 DemiBold `text2` on
/// `raised`.
struct SourceBadge: View {
    let text: String
    var height: CGFloat = 20
    var padding: CGFloat = 7
    var font: Font = .system(size: 11, weight: .semibold)

    var body: some View {
        Text(text)
            .font(font)
            .foregroundStyle(Theme.text2)
            .lineLimit(1)
            .padding(.horizontal, padding)
            .frame(height: height)
            .background(Capsule().fill(Theme.raised))
            .fixedSize()
    }
}

/// A 36-point `raised` well holding a kind's icon, as the import page's cards and rows show it.
struct KindWell: View {
    let symbol: String
    var size: CGFloat = 36
    var iconSize: CGFloat = 18
    var tint: Color = Theme.accentText

    var body: some View {
        Image(systemName: symbol)
            .font(.system(size: iconSize * 0.85, weight: .medium))
            .foregroundStyle(tint)
            .frame(width: size, height: size)
            .background(RoundedRectangle(cornerRadius: size / 4, style: .continuous).fill(Theme.raised))
            .accessibilityHidden(true)
    }
}
