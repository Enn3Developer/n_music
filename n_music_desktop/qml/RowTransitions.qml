import QtQuick
import NMusic

// How a list of the queue's rows animates its changes; its view takes each transition. A new row
// waits 80 ms for the room, then fades in over 150 ms. A row coming back from more than one place
// away fades in over 200 ms from 4 px up once the rows around it are half way, 180 ms in. A row
// leaving fades out in 80 ms, and the rows after it close the gap once it has gone. Rows making
// way move over 200 ms. A new list fades in a row after another, 15 ms apart. Under reduced
// motion rows change place at once. The view's delegates reset their opacity when reused; a
// `tint()` of theirs marks a newly queued row. No Animators here: in a view transition they find
// no row to animate.
QtObject {
    id: rows

    required property ListView view
    required property QueueList queue
    /// How far apart rows stand.
    property real pitch: 54
    /// The row a new list likely shows first, where its rows start coming in.
    property int first: 0
    /// A track change takes the first row out, the one that starts playing: it moves up 0.6
    /// rows as it fades, and the rows after it move up at once.
    property bool slidesOnSkip: false
    /// A track change came a moment ago.
    property bool skipping: false
    /// The view swaps its whole list after fading it out: the rows laid out meanwhile come in
    /// as a new list does.
    property bool replacing: false

    /// The view shows a new list, after fading the old one out.
    signal swapped

    /// Fades the list out over 80 ms, then shows the change waiting for it.
    function fadeThrough() {
        through.restart();
    }

    /// Tints `row`, a delegate of the view, for a moment.
    function tint(row: var) {
        if (typeof row.tint === "function")
            row.tint();
    }

    property SequentialAnimation through: SequentialAnimation {
        // On this thread, so it ends while the window is hidden too.
        NumberAnimation {
            target: rows.view.contentItem
            property: "opacity"
            to: 0
            duration: Motion.exit
        }
        ScriptAction {
            // The old rows go at once and the new ones come in, a row after another.
            script: {
                rows.replacing = true;
                rows.replaced.restart();
                rows.view.contentItem.opacity = 1;
                rows.queue.replace();
                rows.swapped();
            }
        }
    }

    property Connections player: Connections {
        target: Player
        enabled: rows.slidesOnSkip

        function onTrackChanging() {
            rows.skipping = true;
            rows.skipEnd.restart();
        }
    }
    property Timer skipEnd: Timer {
        interval: 600
        onTriggered: rows.skipping = false
    }
    // The view lays the new rows out after the swap, in the same frame.
    property Timer replaced: Timer {
        interval: 100
        onTriggered: rows.replacing = false
    }

    property Transition add: Transition {
        id: adding

        SequentialAnimation {
            PropertyAction {
                property: "opacity"
                value: 0
            }
            // A newly queued row is tinted for a moment, so the eye finds it.
            ScriptAction {
                script: {
                    if (rows.queue.fresh(adding.ViewTransition.index))
                        rows.tint(adding.ViewTransition.item);
                }
            }
            PauseAnimation {
                duration: rows.replacing ? Math.min(Math.max(0, adding.ViewTransition.index - rows.first) * 15, 75) : rows.queue.returning(adding.ViewTransition.index) ? 180 : Motion.exit
            }
            ParallelAnimation {
                NumberAnimation {
                    property: "opacity"
                    from: 0
                    to: 1
                    duration: !rows.replacing && rows.queue.returning(adding.ViewTransition.index) ? Motion.move : Motion.fade
                    easing.bezierCurve: Motion.standard
                }
                NumberAnimation {
                    property: "y"
                    from: adding.ViewTransition.destination.y - (rows.replacing || rows.queue.returning(adding.ViewTransition.index) ? 4 * Motion.travel : 0)
                    to: adding.ViewTransition.destination.y
                    duration: rows.replacing ? Motion.fade : Motion.move
                    easing.bezierCurve: Motion.standard
                }
            }
        }
    }

    property Transition remove: Transition {
        id: removing

        NumberAnimation {
            property: "opacity"
            to: 0
            duration: rows.replacing ? 0 : Motion.exit
        }
        NumberAnimation {
            property: "y"
            // Bound before any row leaves, when there is none.
            to: removing.ViewTransition.item ? removing.ViewTransition.item.y - (rows.skipping && removing.ViewTransition.index === 0 ? 0.6 * rows.pitch * Motion.travel : 0) : 0
            duration: Motion.move
            easing.bezierCurve: Motion.standard
        }
    }

    // A row coming in may be cut short by another change: these take it to full opacity too.
    property Transition removeDisplaced: Transition {
        SequentialAnimation {
            PauseAnimation {
                duration: rows.skipping ? 0 : Motion.exit
            }
            ParallelAnimation {
                NumberAnimation {
                    property: "y"
                    duration: Motion.reduced ? 0 : Motion.move
                    easing.bezierCurve: Motion.standard
                }
                NumberAnimation {
                    property: "opacity"
                    to: 1
                    duration: Motion.fade
                }
            }
        }
    }

    property Transition addDisplaced: Transition {
        NumberAnimation {
            property: "y"
            duration: Motion.reduced ? 0 : Motion.move
            easing.bezierCurve: Motion.standard
        }
        NumberAnimation {
            property: "opacity"
            to: 1
            duration: Motion.fade
        }
    }

    property Transition moveDisplaced: Transition {
        NumberAnimation {
            property: "y"
            duration: Motion.reduced ? 0 : Motion.move
            easing.bezierCurve: Motion.standard
        }
        NumberAnimation {
            property: "opacity"
            to: 1
            duration: Motion.fade
        }
    }

    property Transition move: Transition {
        NumberAnimation {
            property: "y"
            duration: Motion.reduced ? 0 : Motion.move
            easing.bezierCurve: Motion.standard
        }
        NumberAnimation {
            property: "opacity"
            to: 1
            duration: Motion.fade
        }
    }

    property Transition populate: Transition {
        id: populating

        SequentialAnimation {
            PropertyAction {
                property: "opacity"
                value: 0
            }
            PauseAnimation {
                duration: Math.min(Math.max(0, populating.ViewTransition.index - rows.first) * 15, 75)
            }
            ParallelAnimation {
                NumberAnimation {
                    property: "opacity"
                    from: 0
                    to: 1
                    duration: Motion.fade
                    easing.bezierCurve: Motion.standard
                }
                NumberAnimation {
                    property: "y"
                    from: populating.ViewTransition.destination.y - 4 * Motion.travel
                    to: populating.ViewTransition.destination.y
                    duration: Motion.fade
                    easing.bezierCurve: Motion.standard
                }
            }
        }
    }
}
