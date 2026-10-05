pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// A source as a card: the cover of one of its tracks, how it is doing when not up to date, its
// name and tracks, and a menu of what can be done with it. Clicking it opens its tracks.
AbstractButton {
    id: card

    /// The source, as `Sources.items` lists it.
    required property var source

    /// Renaming it was asked for.
    signal renameRequested
    /// Taking it out was asked for, to be confirmed first.
    signal removeRequested

    /// What is wrong with it or happening to it; empty while it is up to date.
    readonly property string status: {
        if (!source.available)
            return source.kind === "web" ? Tr.t.source_unreachable : Tr.t.source_missing;
        return source.updating ? Tr.t.source_updating : "";
    }

    /// Opens its menu under its menu button.
    function openMenu() {
        menu.popup(more, more.width - menu.width, more.height + 4);
    }

    hoverEnabled: true
    text: source.name
    Accessible.name: status === "" ? text : text + ", " + status
    Keys.onPressed: event => {
        // The menu key, or Shift+F10, opens its menu.
        if (event.key === Qt.Key_Menu || (event.key === Qt.Key_F10 && event.modifiers & Qt.ShiftModifier)) {
            event.accepted = true;
            card.openMenu();
        }
    }

    TapHandler {
        acceptedButtons: Qt.RightButton
        onTapped: point => menu.popup(card, point.position.x, point.position.y)
    }

    background: null
    contentItem: Column {
        spacing: 10

        Item {
            width: card.width
            height: card.width

            Cover {
                size: parent.width
                radius: 8
                path: card.source.cover
                iconName: SourceKinds.icon(card.source.kind)
                // Gone, its tracks cannot play.
                opacity: card.down ? 0.8 : card.source.available ? 1 : 0.5

                Rectangle {
                    anchors.fill: parent
                    radius: parent.radius
                    color: "#FFFFFF"
                    opacity: card.hovered ? 0.06 : 0
                }
            }

            Rectangle {
                x: 8
                y: parent.height - height - 8
                visible: card.status !== ""
                width: Math.min(status.implicitWidth + 17, parent.width - 16)
                height: 24
                radius: 12
                color: Theme.menu
                border.width: 1
                border.color: Theme.line2

                RowLayout {
                    id: status
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 9
                    spacing: 5

                    Icon {
                        visible: !card.source.available
                        name: "alert"
                        size: 13
                        stroke: 2
                        color: Theme.danger
                    }
                    Icon {
                        visible: card.source.available
                        name: "refresh"
                        size: 13
                        stroke: 2
                        color: Theme.accentText

                        RotationAnimator on rotation {
                            running: card.source.updating && card.source.available && card.visible
                            from: 0
                            to: 360
                            duration: 1400
                            loops: Animation.Infinite
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        text: card.status
                        elide: Text.ElideRight
                        color: card.source.available ? Theme.accentText : Theme.danger
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                    }
                }
            }
        }
        RowLayout {
            width: card.width
            spacing: 4

            Column {
                Layout.fillWidth: true
                spacing: 2

                Label {
                    width: parent.width
                    text: card.text
                    elide: Text.ElideRight
                    color: Theme.text
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                }
                Label {
                    width: parent.width
                    text: Format.count(card.source.tracks, Tr.t.track_one, Tr.t.tracks_many)
                    elide: Text.ElideRight
                    color: Theme.text2
                    font.pixelSize: 13
                    font.features: {
                        "tnum": 1
                    }
                }
            }
            IconButton {
                id: more
                size: 32
                iconSize: 16
                stroke: 1.9
                color: Theme.text3
                iconName: "more"
                text: Tr.t.source_actions.arg(card.source.name)
                onClicked: card.openMenu()
            }
        }
    }

    // Where it is, telling apart sources of the same name.
    Tip {
        visible: card.hovered && !more.hovered && !menu.visible
        delay: 800
        text: card.source.location
    }

    SourceMenu {
        id: menu
        source: card.source
        onRenameRequested: card.renameRequested()
        onRemoveRequested: card.removeRequested()
    }
}
