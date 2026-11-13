import AppKit
import MediaPlayer

/// Control Center's Now Playing, the media keys, AirPods and the Touch Bar, through
/// MediaPlayer's info and command centers.
///
/// The system advances the elapsed time by itself from the playback rate, so the info is set
/// again only on discontinuities: a new track or cover, play/pause, a seek. Never for the
/// position reports in between.
@MainActor
final class NowPlaying {
    private let store: Store
    private var artwork: (locator: Locator, image: NSImage)?
    private var artworkTask: Task<Void, Never>?
    /// The length last published: the decoder's length replaces the tags' once it knows it.
    private var publishedLength: Double = 0

    init(store: Store) {
        self.store = store
        Self.installCommands()
        store.observers.append { [weak self] event in
            self?.changed(event)
        }
        // A session resumed at launch has its track already.
        if let track = store.current?.track { loadArtwork(for: track) }
        update()
    }

    // MediaPlayer calls the command handlers and the artwork's request handler on its own
    // queues. A closure written in this main-actor class would be main-actor isolated, and
    // Swift 6 checks that when it runs: off the main thread, the app traps. So they are made in
    // nonisolated functions, and the handlers hop to the main actor themselves.

    nonisolated private static func installCommands() {
        let center = MPRemoteCommandCenter.shared()
        handle(center.playCommand) { $0.play() }
        handle(center.pauseCommand) { $0.pause() }
        handle(center.togglePlayPauseCommand) { $0.togglePause() }
        handle(center.nextTrackCommand) { $0.next() }
        handle(center.previousTrackCommand) { $0.previous() }
        handle(center.stopCommand) { $0.pause() }
        center.changePlaybackPositionCommand.addTarget { event in
            guard let event = event as? MPChangePlaybackPositionCommandEvent else { return .commandFailed }
            let position = event.positionTime
            Task { @MainActor in Store.shared.seek(to: position) }
            return .success
        }
        center.changeRepeatModeCommand.addTarget { event in
            guard let event = event as? MPChangeRepeatModeCommandEvent else { return .commandFailed }
            let status: LoopStatus = switch event.repeatType {
            case .off: .off
            case .one: .file
            default: .playlist
            }
            Task { @MainActor in Store.shared.setLoopStatus(status) }
            return .success
        }
        center.changeShuffleModeCommand.addTarget { event in
            guard let event = event as? MPChangeShuffleModeCommandEvent else { return .commandFailed }
            let enabled = event.shuffleType != .off
            Task { @MainActor in Store.shared.setShuffle(enabled) }
            return .success
        }
    }

    nonisolated private static func handle(
        _ command: MPRemoteCommand, _ action: @escaping @MainActor @Sendable (Store) -> Void
    ) {
        command.addTarget { event in
            #if DEBUG
            NSLog("N Music: remote command %@", String(describing: type(of: event.command)))
            #endif
            Task { @MainActor in action(Store.shared) }
            return .success
        }
    }

    /// The image is only read, from whichever queue MediaPlayer asks on.
    nonisolated private static func artwork(_ image: NSImage) -> MPMediaItemArtwork {
        let sendable = SendableImage(image: image)
        return MPMediaItemArtwork(boundsSize: image.size) { _ in sendable.image }
    }

    private struct SendableImage: @unchecked Sendable {
        let image: NSImage
    }

    private func changed(_ event: CoreEvent) {
        switch event {
        case .trackChanged(_, let track):
            if artwork?.locator != track.locator {
                loadArtwork(for: track)
            }
            update()
        case .playbackChanged, .loopStatusChanged, .shuffleChanged:
            update()
        case .positionChanged(_, _, _, let discontinuity):
            if discontinuity || abs(store.length - publishedLength) > 0.5 { update() }
        default:
            break
        }
    }

    private func loadArtwork(for track: TrackRow) {
        artworkTask?.cancel()
        artwork = nil
        let locator = track.locator
        let cover = track.cover
        let core = store.core
        artworkTask = Task {
            var picture = await Artwork.shared.full(locator, pixels: 600, core: core)
            if picture == nil, let cover {
                picture = await Artwork.shared.thumbnail(path: cover, pixels: 600)
            }
            guard !Task.isCancelled, let picture else { return }
            artwork = (locator, NSImage(cgImage: picture.image, size: .zero))
            update()
        }
    }

    private func update() {
        let info = MPNowPlayingInfoCenter.default()
        publishedLength = store.length
        guard let track = store.current?.track else {
            info.nowPlayingInfo = nil
            info.playbackState = .stopped
            return
        }
        var values: [String: Any] = [
            MPMediaItemPropertyTitle: track.title,
            MPMediaItemPropertyArtist: track.artist,
            MPMediaItemPropertyPlaybackDuration: store.length,
            MPNowPlayingInfoPropertyElapsedPlaybackTime: store.position(),
            MPNowPlayingInfoPropertyPlaybackRate: store.playing ? 1.0 : 0.0,
            MPNowPlayingInfoPropertyDefaultPlaybackRate: 1.0,
            MPNowPlayingInfoPropertyMediaType: MPNowPlayingInfoMediaType.audio.rawValue,
        ]
        if let album = track.album { values[MPMediaItemPropertyAlbumTitle] = album }
        if let albumArtist = track.albumArtist { values[MPMediaItemPropertyAlbumArtist] = albumArtist }
        if let number = track.trackNumber { values[MPMediaItemPropertyAlbumTrackNumber] = number }
        if let artwork, artwork.locator == track.locator {
            values[MPMediaItemPropertyArtwork] = Self.artwork(artwork.image)
        }
        info.nowPlayingInfo = values
        info.playbackState = store.playing ? .playing : .paused

        let center = MPRemoteCommandCenter.shared()
        center.changeRepeatModeCommand.currentRepeatType = switch store.loopStatus {
        case .off: .off
        case .file: .one
        case .playlist: .all
        }
        center.changeShuffleModeCommand.currentShuffleType = store.shuffle ? .items : .off
    }
}
