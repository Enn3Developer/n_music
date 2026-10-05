import QtQuick
import NMusic

// Going back and forward through the pages shown, see `History`.
Row {
    id: buttons

    property real size: 28

    spacing: 2

    IconButton {
        size: buttons.size
        iconSize: 16
        stroke: 2
        iconName: "chevron-left"
        text: Tr.t.go_back
        enabled: History.canGoBack
        onClicked: History.back()
    }
    IconButton {
        size: buttons.size
        iconSize: 16
        stroke: 2
        iconName: "chevron-right"
        text: Tr.t.go_forward
        enabled: History.canGoForward
        onClicked: History.forward()
    }
}
