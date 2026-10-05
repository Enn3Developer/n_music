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

    readonly property var sections: [
        {
            value: "sources",
            label: Tr.t.sources
        },
        {
            value: "playback",
            label: Tr.t.playback
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
    ]
    /// Too narrow for the sections beside the settings: they line up above them instead.
    readonly property bool stacked: width < 880
    /// The general sections, while they show.
    readonly property GeneralSettings general: content.item as GeneralSettings
    /// The section of the general ones in view as the page scrolls.
    readonly property string visibleSection: {
        if (!general)
            return section;
        const names = ["playback", "appearance", "updates", "about"];
        // At the end the last section is in view, however short.
        if (scroller.contentY >= scroller.contentHeight - scroller.height - 1)
            return names[names.length - 1];
        let shown = names[0];
        for (const name of names) {
            if (content.y + general.sectionY(name) <= scroller.contentY + 40)
                shown = name;
        }
        return shown;
    }

    /// Shows the section `name`, scrolling to it among the general ones.
    function reveal(name: string) {
        if (!general) {
            scroller.contentY = 0;
            return;
        }
        const top = name === "playback" ? 0 : content.y + general.sectionY(name) - 28;
        scroller.contentY = Math.max(0, Math.min(top, scroller.contentHeight - scroller.height));
    }

    onSectionChanged: Qt.callLater(reveal, section)
    Component.onCompleted: Qt.callLater(reveal, section)

    GridLayout {
        anchors.fill: parent
        columns: page.stacked ? 1 : 2
        rowSpacing: 0
        columnSpacing: 0

        ColumnLayout {
            Layout.fillWidth: page.stacked
            Layout.preferredWidth: page.stacked ? -1 : 200
            Layout.fillHeight: !page.stacked
            Layout.topMargin: page.stacked ? 22 : 28
            Layout.leftMargin: page.stacked ? 28 : 20
            Layout.rightMargin: page.stacked ? 28 : 12
            spacing: 2

            Label {
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
            contentHeight: content.implicitHeight + 28 + 32
            boundsBehavior: Flickable.StopAtBounds

            Loader {
                id: content
                x: 28
                y: page.stacked ? 20 : 28
                width: Math.min(scroller.width - 56, 704)
                sourceComponent: page.section === "sources" ? sourcesSection : generalSection
            }

            ScrollBar.vertical: ThinScrollBar {}
        }
    }

    Component {
        id: sourcesSection
        SourcesSettings {}
    }
    Component {
        id: generalSection
        GeneralSettings {}
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
