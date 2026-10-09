pragma ComponentBehavior: Bound
import QtQuick
import NMusic

// Words about the current track that fade through when it changes, so two titles never share a
// frame: the old words go in 80 ms, then the new ones fade in over 150 ms and rise 8 px over
// 200 ms, coming down from above when going back. A change within 250 ms of the one before
// shows the new words at once. Under reduced motion they fade without rising.
Item {
    id: swap

    /// Lays out the words: its `required property var track` gets what the player tells of a
    /// track, as `{ loaded, title, artist, album, year, codec, sampleRate, bits, albumGain,
    /// trackGain, plays }`. It is made as wide as this.
    property Component delegate

    /// What the player tells of the current track; a new object at each change.
    readonly property var live: ({
            loaded: Player.loaded,
            title: Player.title,
            artist: Player.artist,
            album: Player.album,
            year: Player.year,
            codec: Player.codec,
            sampleRate: Player.sampleRate,
            bits: Player.bits,
            albumGain: Player.albumGain,
            trackGain: Player.trackGain,
            plays: Player.plays
        })
    /// The slot showing the current track; the other holds the words leaving.
    property Item front: first
    readonly property Item back: front === first ? second : first
    /// When the last change came, in ms.
    property real changed: 0

    // The old words stay as they were; the other slot takes the new ones.
    function change(goingBack: bool) {
        const now = Date.now();
        const quick = now - changed < Motion.quick;
        changed = now;
        through.stop();
        const leaving = front, coming = back;
        leaving.track = live;
        coming.track = Qt.binding(() => swap.live);
        coming.y = 0;
        front = coming;
        if (quick) {
            leaving.opacity = 0;
            coming.opacity = 1;
            return;
        }
        coming.opacity = 0;
        out.target = leaving;
        fadeIn.target = coming;
        rise.target = coming;
        rise.from = 8 * Motion.travel * (goingBack ? -1 : 1);
        through.start();
    }

    implicitWidth: front.implicitWidth
    implicitHeight: front.implicitHeight

    Connections {
        target: Player

        function onTrackChanging(back: bool) {
            swap.change(back);
        }
    }

    SequentialAnimation {
        id: through

        OpacityAnimator {
            id: out
            to: 0
            duration: Motion.exit
        }
        ParallelAnimation {
            OpacityAnimator {
                id: fadeIn
                from: 0
                to: 1
                duration: Motion.fade
                easing.bezierCurve: Motion.standard
            }
            YAnimator {
                id: rise
                to: 0
                duration: Motion.move
                easing.bezierCurve: Motion.standard
            }
        }
    }

    Slot {
        id: first
    }
    Slot {
        id: second
        opacity: 0
    }

    // Holds a copy of the words; only the slot in front is read out.
    component Slot: Item {
        id: slot

        /// The live words until it holds old ones.
        property var track: swap.live
        property Item words: null

        width: swap.width
        implicitWidth: words ? words.implicitWidth : 0
        implicitHeight: words ? words.implicitHeight : 0
        visible: opacity > 0 || swap.front === slot
        Accessible.ignored: swap.front !== slot
        Component.onCompleted: {
            words = swap.delegate.createObject(slot, {
                width: Qt.binding(() => slot.width),
                track: Qt.binding(() => slot.track)
            });
        }
    }
}
