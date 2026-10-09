pragma Singleton
import QtQuick

// How much room the window has, deciding how the interface lays out: `wide` adds a panel of
// what plays, `regular` has the sidebar, `compact` a rail of icons, and `narrow` a drawer.
QtObject {
    /// The window's width, set by the window.
    property real width: 1280

    /// A page asked for the navigation, which narrow windows keep in a drawer.
    signal navigationRequested
    /// The track called `title` was queued: to play `next`, or after what is queued.
    signal queued(string title, bool next)

    readonly property string size: width >= 1600 ? "wide" : width >= 1100 ? "regular" : width >= 720 ? "compact" : "narrow"
    readonly property bool wide: size === "wide"
    readonly property bool regular: size === "regular"
    readonly property bool compact: size === "compact"
    readonly property bool narrow: size === "narrow"
}
