pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// The settings: their sections, and the one picked.
Item {
    id: page

    /// The section shown, see `sections`.
    property string section: "sources"

    signal navigate(string page)

    readonly property var sections: [
        {
            value: "sources",
            label: Tr.t.sources
        }
    ]

    RowLayout {
        anchors.fill: parent
        spacing: 0

        ColumnLayout {
            // Layouts fill by default.
            Layout.fillWidth: false
            Layout.preferredWidth: 200
            Layout.fillHeight: true
            Layout.topMargin: 28
            Layout.leftMargin: 20
            Layout.rightMargin: 12
            spacing: 2

            Label {
                Layout.leftMargin: 10
                Layout.bottomMargin: 14
                text: Tr.t.settings
                color: Theme.text
                font.pixelSize: 26
                font.weight: Font.Bold
                font.letterSpacing: -0.52
            }
            Repeater {
                model: page.sections

                AbstractButton {
                    id: entry

                    required property var modelData
                    readonly property bool current: page.section === modelData.value

                    Layout.fillWidth: true
                    implicitHeight: 36
                    leftPadding: 10
                    rightPadding: 10
                    hoverEnabled: true
                    text: modelData.label
                    Accessible.role: Accessible.Button
                    onClicked: page.navigate("settings:" + modelData.value)

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
            Item {
                Layout.fillHeight: true
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
                y: 28
                width: Math.min(scroller.width - 56, 704)
                sourceComponent: ({
                        sources: sourcesSection
                    })[page.section] ?? sourcesSection
            }

            ScrollBar.vertical: ThinScrollBar {}
        }
    }

    Component {
        id: sourcesSection
        SourcesSettings {}
    }
}
