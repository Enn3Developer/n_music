pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// Settings › Playback, Appearance, Updates and About, one under the other.
ColumnLayout {
    id: settings

    /// Where the section `name` starts.
    function sectionY(name: string): real {
        const section = ({
                playback: playback,
                appearance: appearance,
                updates: updates,
                about: about
            })[name];
        return section ? section.y : 0;
    }

    spacing: 34

    ColumnLayout {
        id: playback
        Layout.fillWidth: true
        Layout.topMargin: 6
        spacing: 12

        Heading {
            text: Tr.t.playback
        }
        SettingsGroup {
            Layout.fillWidth: true

            ColumnLayout {
                Layout.fillWidth: true
                Layout.margins: 16
                spacing: 12

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    Label {
                        Layout.fillWidth: true
                        text: Tr.t.replay_gain
                        wrapMode: Text.Wrap
                        color: Theme.text
                        font.pixelSize: 14
                        font.weight: Font.DemiBold
                    }
                    Label {
                        Layout.fillWidth: true
                        text: Tr.t.replay_gain_hint
                        wrapMode: Text.Wrap
                        color: Theme.text2
                        font.pixelSize: 13
                    }
                }
                SegmentedControl {
                    Layout.fillWidth: true
                    Layout.maximumWidth: 420
                    implicitHeight: 44
                    color: Theme.bg
                    options: [Tr.t.replay_gain_off, Tr.t.replay_gain_track, Tr.t.replay_gain_album]
                    current: AppState.replayGain
                    Accessible.name: Tr.t.replay_gain
                    onActivated: index => AppState.replayGain = index
                }
                Label {
                    Layout.fillWidth: true
                    text: [Tr.t.replay_gain_off_note, Tr.t.replay_gain_track_note, Tr.t.replay_gain_album_note][AppState.replayGain] ?? ""
                    wrapMode: Text.Wrap
                    color: Theme.text3
                    font.pixelSize: 13
                }
            }
        }
    }

    ColumnLayout {
        id: appearance
        Layout.fillWidth: true
        spacing: 12

        Heading {
            text: Tr.t.appearance
        }
        SettingsGroup {
            Layout.fillWidth: true

            SettingRow {
                title: Tr.t.theme
                divider: true

                SegmentedControl {
                    // System, dark, light; the setting counts system, light, dark.
                    readonly property var themes: [0, 2, 1]

                    Layout.preferredWidth: 300
                    color: Theme.bg
                    options: [Tr.t.theme_system, Tr.t.theme_dark, Tr.t.theme_light]
                    current: themes.indexOf(AppState.theme)
                    Accessible.name: Tr.t.theme
                    onActivated: index => AppState.theme = themes[index]
                }
            }
            SettingRow {
                title: Tr.t.language
                divider: true

                SelectBox {
                    Layout.preferredWidth: 300
                    options: Translations.languages.map((label, index) => ({
                                value: String(index),
                                label: label
                            }))
                    value: String(Translations.language)
                    Accessible.name: Tr.t.language
                    onActivated: value => Translations.language = Number(value)
                }
            }
            SettingRow {
                title: Tr.t.window_size
                description: Tr.t.window_size_hint
                divider: true

                ToggleSwitch {
                    checked: AppState.saveWindowSize
                    Accessible.name: Tr.t.window_size
                    onToggled: AppState.saveWindowSize = checked
                }
            }
            SettingRow {
                title: Tr.t.mini_on_top
                description: Tr.t.mini_on_top_hint

                ToggleSwitch {
                    checked: AppState.miniOnTop
                    Accessible.name: Tr.t.mini_on_top
                    onToggled: AppState.miniOnTop = checked
                }
            }
        }
    }

    ColumnLayout {
        id: updates
        Layout.fillWidth: true
        spacing: 12

        Heading {
            text: Tr.t.updates
        }
        SettingsGroup {
            Layout.fillWidth: true

            SettingRow {
                id: release

                readonly property string status: Updates.status

                title: "N Music " + AppState.version
                divider: automatic.visible
                description: {
                    switch (status) {
                    case "unsupported":
                        return Tr.t.update_unsupported;
                    case "checking":
                        return Tr.t.update_checking;
                    case "current":
                        return Tr.t.update_current.arg(new Date(Updates.checked * 1000).toLocaleTimeString(Qt.locale(), Locale.ShortFormat));
                    case "available":
                        return Tr.t.update_available.arg(Updates.latest);
                    case "downloading":
                        return Tr.t.update_downloading.arg(Updates.latest).arg(Updates.progress);
                    case "ready":
                        return Tr.t.update_ready.arg(Updates.latest);
                    case "failed":
                        return Tr.t.update_failed;
                    }
                    return Tr.t.update_idle;
                }
                descriptionIcon: ({
                        current: "check",
                        available: "arrow-down",
                        ready: "check",
                        failed: "alert"
                    })[status] ?? ""
                descriptionIconColor: status === "current" ? Theme.success : status === "failed" ? Theme.danger : Theme.accentText

                PillButton {
                    small: true
                    visible: release.status !== "unsupported"
                    primary: release.status === "available" || release.status === "ready"
                    enabled: release.status !== "checking" && release.status !== "downloading"
                    text: release.status === "available" || release.status === "downloading" ? Tr.t.update_download : release.status === "ready" ? Tr.t.update_restart : Tr.t.check_now
                    onClicked: {
                        if (release.status === "available")
                            Updates.download();
                        else if (release.status === "ready") {
                            if (Updates.apply())
                                Qt.quit();
                        } else
                            Updates.check();
                    }
                }
            }
            SettingRow {
                id: automatic
                // Copies the installer did not install update the way they were installed.
                visible: release.status !== "unsupported"
                title: Tr.t.update_auto
                description: Tr.t.update_auto_hint

                ToggleSwitch {
                    checked: AppState.checkUpdates
                    Accessible.name: Tr.t.update_auto
                    onToggled: AppState.checkUpdates = checked
                }
            }
        }
    }

    ColumnLayout {
        id: about
        Layout.fillWidth: true
        spacing: 12

        Heading {
            text: Tr.t.about
        }
        SettingsGroup {
            Layout.fillWidth: true

            SettingRow {
                title: Tr.t.logs
                description: Tr.t.logs_hint
                divider: true

                PillButton {
                    small: true
                    text: Tr.t.open_logs
                    onClicked: Qt.openUrlExternally(AppState.logsFolder)
                }
            }
            SettingRow {
                description: Tr.t.licence_note

                AbstractButton {
                    id: link
                    hoverEnabled: true
                    text: Tr.t.source_code
                    Accessible.role: Accessible.Link
                    onClicked: Qt.openUrlExternally("https://github.com/Enn3Developer/n_music")

                    contentItem: Label {
                        text: link.text
                        color: link.hovered ? Theme.text : Theme.text2
                        font.pixelSize: 13
                        font.underline: true
                    }
                    background: Rectangle {
                        color: "transparent"
                        radius: 4
                        border.width: link.visualFocus ? 2 : 0
                        border.color: Theme.text
                    }
                }
            }
        }
    }

    component Heading: Label {
        color: Theme.text
        font.pixelSize: 20
        font.weight: Font.Bold
    }
}
