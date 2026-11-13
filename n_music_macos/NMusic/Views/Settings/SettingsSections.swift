import SwiftUI

// Every row of Settings (spec §6.3), section by section, in the Qt app's order and wording.
// Playback rows go to the core (they are saved in `core.playback` / `core.stream_cache`); the
// others are `UISettings` (`desktop.ui`), saved as they change.

/// One section of the page: its heading, 12 points, its group.
struct SettingsSection<Content: View>: View {
    let title: String
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            SettingsHeading(text: title)
            SettingsGroup { content }
        }
    }
}

// MARK: - Playback

struct PlaybackSettings: View {
    private let store = Store.shared
    /// The cache as read once, until the core reports it (`store.streamCache`).
    @State private var readCache: StreamCacheState?
    /// Whether the core has listed the devices since the page showed.
    @State private var devicesListed = false

    private static let crossfades = [0, 2, 4, 6, 8, 10, 12]
    private static let cacheSizes: [UInt64] = [512 << 20, 1 << 30, 2 << 30, 5 << 30, 10 << 30, 20 << 30, 50 << 30]

    private var cache: StreamCacheState? { store.streamCache ?? readCache }
    private var cacheEnabled: Bool { cache?.enabled ?? false }

    var body: some View {
        SettingsSection(title: tr("playback")) {
            replayGain
            SettingsDivider()
            SettingRow(title: tr("crossfade"), description: tr("crossfade_hint"), divider: true) {
                SettingsPopUp(
                    label: tr("crossfade"),
                    options: Self.crossfades.map { seconds in
                        SettingsOption(value: seconds, title: seconds == 0 ? tr("crossfade_off") : tr("crossfade_seconds", seconds))
                    },
                    selection: crossfade,
                    onSelect: { store.setCrossfade(Double($0)) }
                )
                .frame(width: 140)
            }
            SettingRow(title: tr("output_device"), description: tr("output_device_hint"), divider: true) {
                SettingsPopUp(
                    label: tr("output_device"),
                    options: [SettingsOption(value: "", title: tr("system_default"))] + outputChoices.map { choice in
                        SettingsOption(
                            value: choice.device.id,
                            title: choice.connected ? choice.device.name : tr("device_missing", choice.device.name)
                        )
                    },
                    selection: store.outputDevice?.id ?? "",
                    onSelect: chooseOutputDevice,
                    // Devices come and go: they are listed again on each opening (as Qt).
                    onOpen: { store.listOutputDevices() }
                )
                .frame(width: 220)
            }
            SettingRow(
                title: tr("match_rate", fallback: "Match the output's sample rate"),
                description: tr(
                    "match_rate_hint",
                    fallback: "Plays each track at its own sample rate when the device can, so nothing is resampled. A change of rate needs a short gap."
                ),
                divider: true
            ) {
                SettingsSwitch(label: tr("match_rate", fallback: "Match the output's sample rate"), isOn: Binding(
                    get: { store.options.matchRate },
                    set: { store.setMatchRate($0) }
                ))
            }
            SettingRow(title: tr("resume"), description: tr("resume_hint"), divider: true) {
                SettingsSwitch(label: tr("resume"), isOn: Binding(
                    get: { store.options.resume },
                    set: { store.setResume($0) }
                ))
            }
            SettingRow(title: tr("cache_streams"), description: tr("cache_streams_hint"), divider: cacheEnabled) {
                SettingsSwitch(label: tr("cache_streams"), isOn: Binding(
                    get: { cacheEnabled },
                    set: { changeCache(enabled: $0, limit: cache?.limit ?? (1 << 30)) }
                ))
            }
            if let cache, cache.enabled {
                SettingRow(
                    title: tr("cache_size"),
                    description: tr("cache_used", Format.size(Double(cache.used)), Format.size(Double(cache.limit)))
                ) {
                    SettingsPopUp(
                        label: tr("cache_size"),
                        options: cacheSizes(cache.limit).map { SettingsOption(value: $0, title: Format.size(Double($0))) },
                        selection: cache.limit,
                        onSelect: { changeCache(enabled: true, limit: $0) }
                    )
                    .frame(width: 140)
                }
            }
        }
        .onChange(of: store.outputDevices) { devicesListed = true }
        .task {
            // Listed when the page shows (as Qt's box when it is made), and on each opening.
            store.listOutputDevices()
            if store.streamCache == nil { readCache = await store.streamCacheState() }
        }
    }

    private var replayGain: some View {
        VStack(alignment: .leading, spacing: 12) {
            VStack(alignment: .leading, spacing: 3) {
                Text(tr("replay_gain"))
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(Theme.text)
                Text(tr("replay_gain_hint"))
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.text2)
            }
            .fixedSize(horizontal: false, vertical: true)
            SettingsSegmented(
                label: tr("replay_gain"),
                options: [
                    SettingsOption(value: ReplayGainMode.off, title: tr("replay_gain_off")),
                    SettingsOption(value: ReplayGainMode.track, title: tr("replay_gain_track")),
                    SettingsOption(value: ReplayGainMode.album, title: tr("replay_gain_album")),
                ],
                selection: store.options.replayGain,
                large: true,
                onSelect: { store.setReplayGain($0) }
            )
            .frame(maxWidth: 420)
            Text(replayGainNote)
                .font(.system(size: 13))
                .foregroundStyle(Theme.text3)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var replayGainNote: String {
        switch store.options.replayGain {
        case .off: tr("replay_gain_off_note")
        case .track: tr("replay_gain_track_note")
        case .album: tr("replay_gain_album_note")
        }
    }

    /// The saved crossfade rounded to whole seconds (as Qt reads it); one outside the list
    /// leaves the box blank.
    private var crossfade: Int {
        let seconds = store.options.crossfade
        return seconds.isFinite ? Int(min(max(seconds.rounded(), 0), Double(Int32.max))) : 0
    }

    /// The chosen device first while it is not among those listed ("not connected" only once
    /// the core has listed them, as Qt's `show_devices`), then those listed.
    private var outputChoices: [OutputChoice] {
        let listed = store.outputDevices
        var choices: [OutputChoice] = []
        if let chosen = store.outputDevice, !listed.contains(where: { $0.id == chosen.id }) {
            choices.append(OutputChoice(device: chosen, connected: !(devicesListed || !listed.isEmpty)))
        }
        choices += listed.map { OutputChoice(device: $0, connected: true) }
        return choices
    }

    private func chooseOutputDevice(_ id: String) {
        if id.isEmpty {
            store.setOutputDevice(nil)
        } else if let choice = outputChoices.first(where: { $0.device.id == id }) {
            store.setOutputDevice(choice.device)
        }
    }

    /// 512 MB to 50 GB, plus a limit set by hand.
    private func cacheSizes(_ current: UInt64) -> [UInt64] {
        var sizes = Self.cacheSizes
        if !sizes.contains(current) { sizes.append(current) }
        return sizes.sorted()
    }

    /// Turning the cache off deletes it, with no confirmation (as in Qt).
    private func changeCache(enabled: Bool, limit: UInt64) {
        store.setStreamCache(enabled: enabled, limit: limit)
        if store.streamCache == nil {
            readCache = StreamCacheState(enabled: enabled, limit: limit, used: readCache?.used ?? 0)
        }
    }
}

// MARK: - Telegram

struct TelegramSettings: View {
    let state: TelegramState
    private let store = Store.shared
    @State private var signingIn = false
    @State private var signingOut = false

    private var account: String? {
        if case .signedIn(let name) = state.status { return name }
        return nil
    }

    var body: some View {
        SettingsSection(title: tr("telegram")) {
            SettingRow(
                title: account.map { tr("telegram_signed_in_as", $0) } ?? tr("telegram_signed_out"),
                description: tr("telegram_account_hint")
            ) {
                Button(account != nil ? tr("telegram_sign_out") : tr("telegram_sign_in_action")) {
                    if account != nil { signingOut = true } else { signingIn = true }
                }
                .buttonStyle(PillButtonStyle(kind: .plain, small: true))
                .disabled(state.busy)
            }
        }
        .sheet(isPresented: $signingIn) {
            TelegramSourceDialog(signInOnly: true)
        }
        .sheet(isPresented: $signingOut) {
            PromptDialog(
                title: tr("telegram_sign_out_title"),
                message: tr("telegram_sign_out_message"),
                text: nil,
                confirmTitle: tr("telegram_sign_out"),
                danger: true
            ) { _ in
                store.telegramSignOut()
                return nil
            }
        }
    }
}

// MARK: - Appearance

struct AppearanceSettings: View {
    private let settings = UISettings.shared
    private let strings = Tr.shared

    /// In the order of the table: the setting's name and the chip's key.
    private static let columns: [(name: String, key: String)] = [
        ("number", "column_number"), ("album", "column_album"), ("genre", "column_genre"),
        ("year", "column_year"), ("added", "column_added"), ("plays", "column_plays"),
        ("lastPlayed", "column_last_played"), ("format", "column_format"), ("time", "column_time"),
    ]

    var body: some View {
        @Bindable var settings = settings
        SettingsSection(title: tr("appearance")) {
            SettingRow(title: tr("theme"), divider: true) {
                SettingsSegmented(
                    label: tr("theme"),
                    options: [
                        SettingsOption(value: UISettings.ThemeChoice.system, title: tr("theme_system")),
                        SettingsOption(value: UISettings.ThemeChoice.dark, title: tr("theme_dark")),
                        SettingsOption(value: UISettings.ThemeChoice.light, title: tr("theme_light")),
                    ],
                    selection: settings.theme,
                    onSelect: { settings.theme = $0 }
                )
                .frame(width: 300)
            }
            SettingRow(title: tr("accent"), divider: true) {
                HStack(spacing: 6) {
                    ForEach(Accent.presets) { accent in
                        AccentSwatch(accent: accent, picked: Accent.current.name == accent.name) {
                            settings.accent = accent.name
                        }
                    }
                }
                // A radio group for VoiceOver, as Qt's swatches (Accessible.RadioButton).
                .accessibilityRepresentation {
                    Picker(tr("accent"), selection: Binding(
                        get: { Accent.current.name },
                        set: { settings.accent = $0 }
                    )) {
                        ForEach(Accent.presets) { accent in
                            Text(tr(accent.labelKey)).tag(accent.name)
                        }
                    }
                    .pickerStyle(.radioGroup)
                }
            }
            SettingRow(title: tr("compact_rows"), description: tr("compact_rows_hint"), divider: true) {
                SettingsSwitch(label: tr("compact_rows"), isOn: $settings.compactRows)
            }
            SettingRow(title: tr("reduce_motion"), description: tr("reduce_motion_hint"), divider: true) {
                SettingsSwitch(label: tr("reduce_motion"), isOn: $settings.reduceMotion)
            }
            SettingRow(title: tr("track_columns"), description: tr("track_columns_hint"), divider: true, controlsBelow: true) {
                FlowLayout(spacing: 6) {
                    ForEach(Self.columns, id: \.name) { column in
                        let on = settings.isColumnVisible(column.name)
                        ColumnChip(label: tr(column.key), on: on) {
                            settings.setColumn(column.name, visible: !on)
                        }
                    }
                }
                .accessibilityElement(children: .contain)
                .accessibilityLabel(tr("track_columns"))
            }
            SettingRow(title: tr("language"), divider: true) {
                SettingsPopUp(
                    label: tr("language"),
                    options: [SettingsOption(value: "", title: strings.systemChoiceLabel)]
                        + strings.languages.map { SettingsOption(value: $0.code, title: $0.name) },
                    selection: strings.language ?? "",
                    onSelect: { settings.locale = $0.isEmpty ? nil : $0 }
                )
                .frame(width: 300)
            }
            SettingRow(title: tr("window_size"), description: tr("window_size_hint"), divider: true) {
                SettingsSwitch(label: tr("window_size"), isOn: $settings.saveWindowSize)
            }
            SettingRow(title: tr("mini_on_top"), description: tr("mini_on_top_hint")) {
                SettingsSwitch(label: tr("mini_on_top"), isOn: $settings.miniOnTop)
            }
        }
    }
}

// MARK: - Updates

struct UpdatesSettings: View {
    private let updates = Updates.shared
    private let settings = UISettings.shared

    var body: some View {
        @Bindable var settings = settings
        SettingsSection(title: tr("updates")) {
            SettingRow(
                title: "N Music " + updates.version,
                description: description,
                descriptionIcon: icon?.symbol,
                descriptionIconColor: icon?.color ?? Theme.text2,
                divider: true
            ) {
                Button(available ? tr("update_download") : tr("check_now")) {
                    if available { updates.download() } else { updates.check() }
                }
                .buttonStyle(PillButtonStyle(kind: available ? .primary : .plain, small: true))
                .disabled(updates.status == .checking)
            }
            SettingRow(title: tr("update_auto"), description: tr("update_auto_hint")) {
                SettingsSwitch(label: tr("update_auto"), isOn: $settings.checkUpdates)
            }
        }
    }

    private var available: Bool {
        if case .available = updates.status { return true }
        return false
    }

    private var description: String {
        switch updates.status {
        case .idle: tr("update_idle")
        case .checking: tr("update_checking")
        case .current(let checked): tr("update_current", checked.formatted(date: .omitted, time: .shortened))
        case .available(let version, _): tr("update_available", version)
        case .failed: tr("update_failed")
        }
    }

    private var icon: (symbol: String, color: Color)? {
        switch updates.status {
        case .current: ("checkmark", Theme.success)
        case .available: ("arrow.down", Theme.accentText)
        case .failed: ("exclamationmark.triangle", Theme.danger)
        default: nil
        }
    }
}

// MARK: - About

struct AboutSettings: View {
    private let store = Store.shared

    var body: some View {
        SettingsSection(title: tr("about")) {
            SettingRow(title: tr("logs"), description: tr("logs_hint"), divider: true) {
                Button(tr("open_logs")) { openLogs() }
                    .buttonStyle(PillButtonStyle(kind: .plain, small: true))
            }
            SettingRow(description: tr("licence_note")) {
                SettingsLink(title: tr("source_code"), url: URL(string: "https://github.com/Enn3Developer/n_music")!)
            }
        }
    }

    /// The logs' folder in Finder (the data folder when there are none yet).
    private func openLogs() {
        let logs = store.logsDirectory
        let folder = FileManager.default.fileExists(atPath: logs.path) ? logs : store.dataDirectory
        NSWorkspace.shared.open(folder)
    }
}

/// A native switch, its label kept for accessibility only.
struct SettingsSwitch: View {
    let label: String
    @Binding var isOn: Bool

    var body: some View {
        Toggle(label, isOn: $isOn)
            .labelsHidden()
            .toggleStyle(.switch)
    }
}
