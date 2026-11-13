import CoreAudio
import Foundation

/// Asks the core for the output devices again whenever one comes or goes, from Core Audio's
/// own notification: the output menu is then right when it opens, with nothing polling. It lives
/// as long as the app. It does not send `OutputDeviceChanged`: the core's Core Audio backend
/// follows device changes itself, and that command reopens the output (an audible gap).
///
/// The listener block is made in a nonisolated function: Core Audio calls it on the queue given
/// (main here), and a block written in main-actor code would trap if that ever changed.
@MainActor
final class OutputDevices {
    private var address = AudioObjectPropertyAddress(
        mSelector: kAudioHardwarePropertyDevices,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain
    )
    private let listener: AudioObjectPropertyListenerBlock

    init(store: Store) {
        listener = Self.makeListener()
        let status = AudioObjectAddPropertyListenerBlock(
            AudioObjectID(kAudioObjectSystemObject), &address, .main, listener
        )
        if status != noErr {
            NSLog("N Music: could not watch the output devices (%d)", status)
        }
    }

    nonisolated private static func makeListener() -> AudioObjectPropertyListenerBlock {
        { _, _ in
            Task { @MainActor in
                Store.shared.listOutputDevices()
            }
        }
    }
}
