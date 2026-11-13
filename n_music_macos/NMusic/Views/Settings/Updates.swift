import AppKit
import Foundation
import Observation

/// Settings › Updates (brief §11): looks for a newer release of N Music on GitHub
/// (Enn3Developer/n_music, pre-releases included, as the Qt app's Velopack feed does) than the
/// version this bundle carries (`NMusicVersion`, the workspace's version), among the releases
/// that have a Mac build (as the feed offers only its platform's). "Download" opens the
/// release's page: this copy doesn't replace itself, so there is no downloading or ready state.
///
/// Created when Settings first shows; it then checks once if `UISettings.checkUpdates` is on
/// ("Once per launch, when Settings opens.").
@MainActor
@Observable
final class Updates {
    static let shared = Updates()

    enum Status: Equatable {
        case idle
        case checking
        /// Nothing newer, as of `checked`.
        case current(checked: Date)
        /// `version` is newer; `page` is its release page.
        case available(version: String, page: URL)
        case failed
    }

    private(set) var status: Status = .idle
    /// This copy's version, like `2.0.0-alpha.23`.
    let version: String

    @ObservationIgnored private var opened = false

    nonisolated static let releases = URL(string: "https://api.github.com/repos/Enn3Developer/n_music/releases?per_page=30")!

    private init() {
        version = Bundle.main.object(forInfoDictionaryKey: "NMusicVersion") as? String
            ?? Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
            ?? "0"
    }

    /// Settings showed: the first time in this launch, checks when automatic checks are on.
    func settingsOpened() {
        guard !opened else { return }
        opened = true
        if UISettings.shared.checkUpdates { check() }
    }

    func check() {
        guard status != .checking else { return }
        status = .checking
        let installed = version
        Task {
            let answer = await Self.newest()
            switch answer {
            case .none:
                status = .failed
            case .some(let release):
                if let release, let newest = SemanticVersion(release.tag), let mine = SemanticVersion(installed),
                   newest > mine {
                    status = .available(version: newest.text, page: release.page)
                } else {
                    status = .current(checked: Date())
                }
            }
        }
    }

    #if DEBUG
    /// Sets the status by hand (`SettingsDebug`).
    var debugStatus: Status {
        get { status }
        set { status = newValue }
    }
    #endif

    /// Opens the newer release's page.
    func download() {
        if case .available(_, let page) = status {
            NSWorkspace.shared.open(page)
        }
    }

    /// The newest published release (`.some(nil)` when there is none), or `nil` when GitHub
    /// couldn't be asked.
    nonisolated private static func newest() async -> UpdateRelease?? {
        var request = URLRequest(url: releases, cachePolicy: .reloadIgnoringLocalCacheData, timeoutInterval: 20)
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        request.setValue("N-Music-macOS", forHTTPHeaderField: "User-Agent")
        guard let (data, response) = try? await URLSession.shared.data(for: request),
              (response as? HTTPURLResponse)?.statusCode == 200,
              let entries = try? JSONDecoder().decode([GitHubRelease].self, from: data)
        else { return nil }
        let best = entries
            // As Velopack's feed for this platform: only releases with a build for the Mac.
            .filter { !$0.draft && $0.assets.contains(where: \.isMac) }
            .compactMap { entry -> (SemanticVersion, UpdateRelease)? in
                guard let version = SemanticVersion(entry.tag_name), let page = URL(string: entry.html_url) else { return nil }
                return (version, UpdateRelease(tag: entry.tag_name, page: page))
            }
            .max { $0.0 < $1.0 }
        return .some(best?.1)
    }
}

/// A release on GitHub: its tag and its page.
struct UpdateRelease: Sendable {
    let tag: String
    let page: URL
}

/// A release as GitHub's API lists it.
private struct GitHubRelease: Decodable, Sendable {
    let tag_name: String
    let html_url: String
    let draft: Bool
    let assets: [Asset]

    struct Asset: Decodable, Sendable {
        let name: String

        /// A Mac build (`…-osx-arm64-…`, a `.dmg`).
        var isMac: Bool {
            let name = name.lowercased()
            return name.contains("osx") || name.contains("macos") || name.hasSuffix(".dmg")
        }
    }
}

/// A semantic version (`v2.0.0-alpha.23`), ordered by SemVer's precedence: a pre-release comes
/// before its release; identifiers compare numerically when both are numbers.
struct SemanticVersion: Comparable, Sendable {
    let core: [Int]
    let prerelease: [String]
    /// As written, without a leading `v` or build metadata.
    let text: String

    init?(_ string: String) {
        var text = string.trimmingCharacters(in: .whitespaces)
        if text.first == "v" || text.first == "V" { text.removeFirst() }
        if let plus = text.firstIndex(of: "+") { text = String(text[..<plus]) }
        let parts = text.split(separator: "-", maxSplits: 1, omittingEmptySubsequences: false)
        guard let first = parts.first else { return nil }
        let numbers = first.split(separator: ".").map { Int($0) }
        guard !numbers.isEmpty, numbers.allSatisfy({ $0 != nil }) else { return nil }
        core = numbers.map { $0! }
        prerelease = parts.count > 1 ? parts[1].split(separator: ".").map(String.init) : []
        self.text = text
    }

    static func == (lhs: SemanticVersion, rhs: SemanticVersion) -> Bool {
        !(lhs < rhs) && !(rhs < lhs)
    }

    static func < (lhs: SemanticVersion, rhs: SemanticVersion) -> Bool {
        for index in 0..<max(lhs.core.count, rhs.core.count) {
            let left = index < lhs.core.count ? lhs.core[index] : 0
            let right = index < rhs.core.count ? rhs.core[index] : 0
            if left != right { return left < right }
        }
        switch (lhs.prerelease.isEmpty, rhs.prerelease.isEmpty) {
        case (true, true): return false
        case (true, false): return false
        case (false, true): return true
        case (false, false): break
        }
        for (left, right) in zip(lhs.prerelease, rhs.prerelease) where left != right {
            switch (Int(left), Int(right)) {
            case let (l?, r?): return l < r
            case (.some, nil): return true
            case (nil, .some): return false
            case (nil, nil): return left < right
            }
        }
        return lhs.prerelease.count < rhs.prerelease.count
    }
}
