pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import NMusic

// Column titles of a track table; clicking one sorts by it, again to reverse.
Item {
    id: header

    required property TrackList list
    required property TrackColumns columns

    /// The sort key that decides first, like `-year`.
    readonly property string first: list.sort.split(",")[0]

    /// Sorts by `field`, then by `then` for ties.
    function sortBy(field: string, then: string) {
        const descending = first === field;
        list.sort = (descending ? "-" : "") + field + (then === "" ? "" : "," + then);
    }

    implicitHeight: 36

    component HeaderCell: AbstractButton {
        id: cell

        property string field
        property string then
        property bool alignRight: false
        readonly property bool sorted: field !== "" && header.first.replace("-", "") === field

        height: parent.height
        enabled: field !== ""
        hoverEnabled: true
        focusPolicy: Qt.NoFocus
        onClicked: header.sortBy(field, then)

        contentItem: Item {
            Row {
                anchors.verticalCenter: parent.verticalCenter
                anchors.right: cell.alignRight ? parent.right : undefined
                layoutDirection: cell.alignRight ? Qt.RightToLeft : Qt.LeftToRight
                spacing: 4

                Label {
                    text: cell.text
                    color: cell.sorted || cell.hovered ? Theme.text : Theme.text3
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.48
                    font.capitalization: Font.AllUppercase
                }
                Icon {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: cell.sorted
                    name: header.first.startsWith("-") ? "arrow-down" : "arrow-up"
                    size: 12
                    stroke: 2.4
                    color: Theme.text
                }
            }
        }
    }

    Row {
        x: 12
        height: parent.height

        HeaderCell {
            width: header.columns.number
            rightPadding: 14
            alignRight: true
            text: "#"
        }
        HeaderCell {
            width: header.columns.title
            text: Tr.t.column_title
            field: "title"
        }
        HeaderCell {
            width: header.columns.album
            text: Tr.t.column_album
            field: "album"
        }
        HeaderCell {
            width: header.columns.year
            alignRight: true
            text: Tr.t.column_year
            field: "year"
            then: "album"
        }
        HeaderCell {
            width: header.columns.plays
            alignRight: true
            text: Tr.t.column_plays
            field: "plays"
            then: "title"
        }
        HeaderCell {
            width: header.columns.time
            alignRight: true
            text: Tr.t.column_time
            field: "length"
        }
    }

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: Theme.line
    }
}
