pragma Singleton
import QtQuick
import NMusic

// How the interface moves: five durations and three curves every animation takes, and whether
// things travel at all. An animation given one of the curves as its `easing.bezierCurve` becomes
// a Bézier spline.
QtObject {
    /// Things leaving: a menu or a dialog closing, old words, a removed row, a toast.
    readonly property int exit: 80
    /// Fades and changes in place: new words, hover going out, a sorted list coming up.
    readonly property int fade: 150
    /// Things moving where they already are: rows, pills, thumbs, the playing row.
    readonly property int move: 200
    /// New surfaces arriving: menus, dialogs, drawers, and theme and accent colours.
    readonly property int enter: 250
    /// Only the cover flying between a grid card and its page's header.
    readonly property int flight: 400

    /// Fast at first, settling softly: most moves and fades in.
    readonly property var standard: [0.2, 0, 0, 1, 1, 1]
    /// Slow at first, then gone: things leaving.
    readonly property var leaving: [0.3, 0, 1, 1, 1, 1]
    /// A dropped row landing in its place.
    readonly property var landing: [0.3, 0.7, 0.4, 1, 1, 1]

    /// Things change place at once and fade in rather than travel.
    readonly property bool reduced: AppState.reduceMotion
    /// What every move multiplies its travel by: 0 under reduced motion.
    readonly property real travel: reduced ? 0 : 1

    /// A track change this soon after the one before, in ms, shows its end state at once.
    readonly property int quick: 250

    /// How far an animation on `curve`, one of the curves above, has got at `time`, both from 0
    /// to 1.
    function eased(curve: var, time: real): real {
        // The curve's x runs with time and its y with progress: find the x by halving.
        const along = (s, a, b) => 3 * (1 - s) * (1 - s) * s * a + 3 * (1 - s) * s * s * b + s * s * s;
        let low = 0;
        let high = 1;
        for (let step = 0; step < 24; ++step) {
            const s = (low + high) / 2;
            if (along(s, curve[0], curve[2]) < time)
                low = s;
            else
                high = s;
        }
        return along((low + high) / 2, curve[1], curve[3]);
    }
}
