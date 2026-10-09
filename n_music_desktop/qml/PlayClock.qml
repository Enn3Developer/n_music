import QtQuick
import NMusic

// Where the current track is, in seconds, moving on smoothly between the player's reports, which
// come four times a second: a linear run to the track's end over the time it has left. A seek, a
// skip, a pause or a report far from it starts it again from where the player is, so it jumps
// rather than glides there. It rests while not `running`.
QtObject {
    id: clock

    /// Someone can see it: a hidden window lets it rest.
    property bool running: true
    property real position: 0
    readonly property bool advancing: running && Player.playing && Player.length > 0

    /// Starts again from where the player is.
    function sync() {
        run.stop();
        position = Math.max(0, Math.min(Player.position, Player.length));
        if (advancing && Player.length > position) {
            run.from = position;
            run.to = Player.length;
            run.duration = (Player.length - position) * 1000;
            run.start();
        }
    }

    onAdvancingChanged: sync()
    Component.onCompleted: sync()

    property NumberAnimation run: NumberAnimation {
        target: clock
        property: "position"
    }

    property Connections reports: Connections {
        target: Player

        function onPositionChanged() {
            // Reports a little off the run leave it be: following each would stutter.
            if (!clock.run.running || Math.abs(Player.position - clock.position) > 0.35)
                clock.sync();
        }
        function onLengthChanged() {
            clock.sync();
        }
    }
}
