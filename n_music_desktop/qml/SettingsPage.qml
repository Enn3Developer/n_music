pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// The settings: their sections, and the one picked.
Item {
    id: page

    /// The section shown, see `sections`.
    property string section: "playback"

    signal navigate(string page)

    /// The sections; Telegram only in builds that sign in to it.
    readonly property var sections: [
        {
            value: "playback",
            label: Tr.t.playback
        },
        {
            value: "telegram",
            label: Tr.t.telegram
        },
        {
            value: "appearance",
            label: Tr.t.appearance
        },
        {
            value: "updates",
            label: Tr.t.updates
        },
        {
            value: "about",
            label: Tr.t.about
        }
    ].filter(section => section.value !== "telegram" || Telegram.available)
    /// Too narrow for the sections beside the settings: they line up above them instead.
    readonly property bool stacked: width < 880
    /// The section in view as the page scrolls.
    readonly property string visibleSection: {
        const names = sections.map(section => section.value);
        // At the end the last section is in view, however short.
        if (scroller.contentY >= scroller.contentHeight - scroller.height - 1)
            return names[names.length - 1];
        let shown = names[0];
        for (const name of names) {
            if (general.y + general.sectionY(name) <= scroller.contentY + 40)
                shown = name;
        }
        return shown;
    }

    /// Shows the section `name`, scrolling to it.
    function reveal(name: string) {
        const top = name === "playback" ? 0 : general.y + general.sectionY(name) - 28;
        scroller.contentY = Math.max(0, Math.min(top, scroller.contentHeight - scroller.height));
    }

    onSectionChanged: Qt.callLater(reveal, section)
    Component.onCompleted: Qt.callLater(reveal, section)

    GridLayout {
        anchors.fill: parent
        columns: page.stacked ? 1 : 2
        rowSpacing: 0
        columnSpacing: 0

        // Narrow windows are stacked too.
        NarrowBar {
            Layout.fillWidth: true
            visible: Shell.narrow
            title: Tr.t.settings
        }

        ColumnLayout {
            Layout.fillWidth: page.stacked
            Layout.preferredWidth: page.stacked ? -1 : 200
            Layout.fillHeight: !page.stacked
            Layout.topMargin: Shell.narrow ? 12 : page.stacked ? 22 : 28
            Layout.leftMargin: Shell.narrow ? 16 : page.stacked ? 28 : 20
            Layout.rightMargin: Shell.narrow ? 16 : page.stacked ? 28 : 12
            spacing: 2

            Label {
                visible: !Shell.narrow
                Layout.leftMargin: page.stacked ? 0 : 10
                Layout.bottomMargin: page.stacked ? 12 : 14
                text: Tr.t.settings
                color: Theme.text
                font.pixelSize: 26
                font.weight: Font.Bold
                font.letterSpacing: -0.52
            }
            Repeater {
                model: page.sections

                SectionButton {
                    Layout.fillWidth: true
                    visible: !page.stacked
                }
            }
            Flow {
                Layout.fillWidth: true
                visible: page.stacked
                spacing: 4

                Repeater {
                    model: page.sections

                    SectionButton {}
                }
            }
            Item {
                Layout.fillHeight: true
                visible: !page.stacked
            }
        }

        Flickable {
            id: scroller
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: width
            contentHeight: general.implicitHeight + 28 + 32
            boundsBehavior: Flickable.StopAtBounds

            GeneralSettings {
                id: general
                x: Shell.narrow ? 16 : 28
                y: page.stacked ? 20 : 28
                width: Math.min(scroller.width - 2 * x, 704)
            }

            ScrollBar.vertical: ThinScrollBar {}
        }
    }

    // A section to go to: the one in view stands out.
    component SectionButton: AbstractButton {
        id: entry

        required property var modelData
        readonly property bool current: page.visibleSection === modelData.value

        implicitHeight: 36
        implicitWidth: implicitContentWidth + leftPadding + rightPadding
        leftPadding: 10
        rightPadding: 10
        hoverEnabled: true
        text: modelData.label
        Accessible.role: Accessible.Button
        onClicked: {
            if (page.section === modelData.value)
                page.reveal(modelData.value);
            else
                page.navigate("settings:" + modelData.value);
        }

        background: Rectangle {
            radius: 8
            color: entry.current ? Theme.raised : entry.hovered ? Theme.hover : "transparent"
            border.width: entry.visualFocus ? 2 : 0
            border.color: Theme.text
        }
        contentItem: Label {
            text: entry.text
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
            color: entry.current || entry.hovered ? Theme.text : Theme.text2
            font.pixelSize: 14
            font.weight: entry.current ? Font.DemiBold : Font.Medium
        }
    }
}
