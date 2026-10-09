pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

ApplicationWindow {
    id: window

    /// The page the content area shows; one of many, like a playlist's, is `<section>:<which>`.
    readonly property string page: History.page
    /// The page without what follows its `:`.
    readonly property string section: page.indexOf(":") < 0 ? page : page.slice(0, page.indexOf(":"))
    /// What followed the `:` the last time each section showed, by section.
    property var subpages: ({})
    /// Pages shown so far; they stay loaded, keeping their search and scroll position.
    property var visited: ({
            tracks: true
        })
    /// The sections, in the order their pages are made.
    readonly property list<string> sections: ["tracks", "albums", "album", "artists", "artist", "genres", "genre", "sources", "source", "playlist", "queue", "settings"]
    /// The page shown before this one; set at start rather than bound, which could see a
    /// change before `onPageChanged` does.
    property string shown
    /// A cover flight waiting for the next frame, when the pages are laid out; null for none.
    property var pendingFlight: null
    /// The page fading out under a flight: it shows until it has.
    property Item leaving: null
    /// The collection page whose header a flying cover hides.
    property CollectionPage flown: null

    /// The window was maximized when it last showed, to reopen that way.
    property bool maximized: AppState.windowMaximized
    /// The window's size when it is not maximized, to reopen at.
    property size normalSize: Qt.size(AppState.windowWidth, AppState.windowHeight)

    // The saved size, shrunk to fit smaller screens.
    width: Math.min(AppState.windowWidth, Screen.desktopAvailableWidth)
    height: Math.min(AppState.windowHeight, Screen.desktopAvailableHeight)
    minimumWidth: 360
    minimumHeight: 480
    visibility: AppState.windowMaximized ? Window.Maximized : Window.Windowed
    title: "N Music"
    color: Theme.bg
    font.family: Theme.font
    font.pixelSize: 14

    onPageChanged: {
        // Not `section`: its binding may not have seen this change yet.
        const colon = page.indexOf(":");
        const name = colon < 0 ? page : page.slice(0, colon);
        if (colon >= 0)
            subpages = Object.assign({}, subpages, {
                [name]: page.slice(colon + 1)
            });
        visited = Object.assign({}, visited, {
            [name]: true
        });
        const before = shown;
        shown = page;
        const beforeName = before.indexOf(":") < 0 ? before : before.slice(0, before.indexOf(":"));
        land();
        // The queue shows where a queued track went.
        if (name === "queue")
            queuedToast.dismiss();
        const plan = Motion.reduced ? null : flightFor(beforeName, before, name, page);
        if (plan !== null) {
            // The pages are laid out by the next frame: the flight starts then.
            leaving = plan.outgoing;
            plan.incoming.opacity = 0;
            pendingFlight = plan;
        } else if (name !== beforeName || (name !== "settings" && before !== page)) {
            // Another album or playlist is another page; another section of the settings is not.
            arrive(pages.itemAt(sections.indexOf(name)));
        }
    }

    /// The flight of a cover between a grid card and the header of the page it opens, when the
    /// card shows whole; null for none.
    function flightFor(fromName: string, from: string, toName: string, to: string): var {
        const opening = (fromName === "albums" && toName === "album") || (fromName === "artists" && toName === "artist");
        const closing = (fromName === "album" && toName === "albums") || (fromName === "artist" && toName === "artists");
        if (!opening && !closing)
            return null;
        const outgoing = pages.itemAt(sections.indexOf(fromName)) as Loader;
        const incoming = pages.itemAt(sections.indexOf(toName)) as Loader;
        const grid = (opening ? outgoing : incoming).item as GroupsPage;
        const album = (opening ? incoming : outgoing).item as CollectionPage;
        if (grid === null || album === null)
            return null;
        const card = grid.coverFor(opening ? to : from);
        if (card === null)
            return null;
        return {
            opening: opening,
            card: card,
            album: album,
            outgoing: outgoing,
            incoming: incoming
        };
    }

    // Opening, the grid fades out in 80 ms while a copy of the card's cover flies to the header
    // over 400 ms, and the page fades in once the copy is clear of its words and list. Going
    // back, the page fades out in 80 ms, the copy flies home, and the grid fades in around it
    // once it lands. The copy takes the corners of where it lands.
    function fly() {
        const plan = pendingFlight;
        pendingFlight = null;
        const source = plan.opening ? plan.card : plan.album.cover;
        const target = plan.opening ? plan.album.cover : plan.card;
        const first = source.mapToItem(pageArea, 0, 0, source.width, source.height);
        const last = target.mapToItem(pageArea, 0, 0, target.width, target.height);
        flyer.path = plan.card.path;
        flyer.iconName = plan.card.iconName;
        flyer.size = last.width;
        flyer.radius = target.radius;
        flyer.x = last.x;
        flyer.y = last.y;
        flyX.from = first.x;
        flyX.to = last.x;
        flyY.from = first.y;
        flyY.to = last.y;
        flyScale.from = first.width / Math.max(1, last.width);
        flyer.visible = true;
        flown = plan.album;
        flown.coverHidden = true;
        leaveFade.target = plan.outgoing;
        pageOut.start();
        arriveFade.target = plan.incoming;
        arriveWait.duration = plan.opening ? clearAt(first, last, plan.album.zones(pageArea)) : Motion.flight;
        pageIn.start();
        flightHold.duration = plan.opening ? 0 : Motion.fade;
        flight.start();
    }

    /// When the flying copy, going from `first` to `last`, stays clear of `zones` for good, in
    /// ms: 120 for a card right below the header, up to 320 for the far corner.
    function clearAt(first: rect, last: rect, zones: var): int {
        let clear = 0;
        for (let step = 0; step <= 80; ++step) {
            const along = Motion.eased(Motion.standard, step / 80);
            const x = first.x + (last.x - first.x) * along;
            const y = first.y + (last.y - first.y) * along;
            const size = first.width + (last.width - first.width) * along;
            if (zones.some(zone => x < zone.x + zone.width - 0.5 && x + size > zone.x + 0.5 && y < zone.y + zone.height - 0.5 && y + size > zone.y + 0.5))
                clear = step / 80;
        }
        return Math.max(120, Math.min(320, Math.round(clear * Motion.flight)));
    }

    // The copy has landed: the cover it stood for shows again.
    function landed() {
        flyer.visible = false;
        if (flown !== null) {
            flown.coverHidden = false;
            flown = null;
        }
    }

    // Ends a flight at once: the copy goes and every page shows as it should.
    function land() {
        pendingFlight = null;
        flight.stop();
        pageOut.stop();
        pageIn.stop();
        flyer.visible = false;
        if (leaving !== null) {
            leaving.opacity = 1;
            leaving = null;
        }
        if (arriveFade.target !== null) {
            arriveFade.target.opacity = 1;
            arriveFade.target = null;
        }
        if (flown !== null) {
            flown.coverHidden = false;
            flown = null;
        }
    }

    // One page at a time: the old one hides at once, the new one fades in over 150 ms and rises
    // 8 px over 200 ms. A page changed to meanwhile starts its own from the start.
    function arrive(loader: Item) {
        if (arrival.running) {
            arrival.stop();
            const left = arrivalFade.target;
            left.opacity = 1;
            left.y = 0;
        }
        if (loader === null)
            return;
        arrivalFade.target = loader;
        arrivalRise.target = loader;
        loader.opacity = 0;
        arrival.start();
    }

    /// Shows the page `to`, after the one shown in the history.
    function go(to: string) {
        // The settings remember their section: going back shows the one shown then.
        History.go(to === "settings" ? "settings:" + (subpages.settings ?? "playback") : to);
    }

    /// Whether the place in Up next of a track queued now shows, in the queue page or the panel:
    /// first when it plays `next`, else after the queued ones.
    function upNextShows(next: bool): bool {
        if (section === "queue") {
            const loader = pages.itemAt(sections.indexOf("queue")) as Loader;
            const queue = loader ? loader.item as QueuePage : null;
            return queue !== null && queue.shows(next);
        }
        return panel.visible && panel.shows(next);
    }

    function toggleQueue() {
        if (page !== "queue")
            go("queue");
        else if (History.canGoBack)
            History.back();
        else
            go("tracks");
    }

    // Space plays and pauses, unless a text field takes it.
    Shortcut {
        sequence: "Space"
        onActivated: Player.toggle()
    }
    Shortcut {
        sequences: [StandardKey.Back]
        enabled: !Sources.firstRun
        onActivated: History.back()
    }
    Shortcut {
        sequences: [StandardKey.Forward]
        enabled: !Sources.firstRun
        onActivated: History.forward()
    }
    onClosing: {
        if (visibility === Window.Windowed)
            normalSize = Qt.size(width, height);
        AppState.windowClosing(normalSize.width, normalSize.height, maximized);
    }
    onWidthChanged: {
        Shell.width = width;
        normalSizeTimer.restart();
    }
    onHeightChanged: normalSizeTimer.restart()
    Component.onCompleted: {
        Shell.width = width;
        shown = page;
    }
    onVisibilityChanged: {
        AppState.setVisible(visibility !== Window.Minimized && visibility !== Window.Hidden);
        // Minimized or hidden, it stays maximized or not as it was.
        if (visibility === Window.Windowed || visibility === Window.Maximized)
            maximized = visibility === Window.Maximized;
    }

    // Maximizing may resize the window just before it says it is maximized: a new size is its
    // size when not maximized only once it held a moment.
    Timer {
        id: normalSizeTimer
        interval: 500
        onTriggered: {
            if (window.visibility === Window.Windowed)
                window.normalSize = Qt.size(window.width, window.height);
        }
    }

    FontLoader {
        source: "../assets/fonts/Figtree-Regular.ttf"
    }
    FontLoader {
        source: "../assets/fonts/Figtree-Medium.ttf"
    }
    FontLoader {
        source: "../assets/fonts/Figtree-SemiBold.ttf"
    }
    FontLoader {
        source: "../assets/fonts/Figtree-Bold.ttf"
    }

    Component {
        id: tracksPage
        TracksPage {
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: queuePage
        QueuePage {
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: playlistPage
        PlaylistPage {
            playlistId: Number(window.subpages.playlist ?? 0)
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: albumsPage
        GroupsPage {
            kind: "album"
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: artistsPage
        GroupsPage {
            kind: "artist"
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: genresPage
        GroupsPage {
            kind: "genre"
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: sourcesPage
        SourcesPage {
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: albumPage
        CollectionPage {
            kind: "album"
            argument: window.subpages.album ?? ""
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: artistPage
        CollectionPage {
            kind: "artist"
            argument: window.subpages.artist ?? ""
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: genrePage
        CollectionPage {
            kind: "genre"
            argument: window.subpages.genre ?? ""
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: sourcePage
        CollectionPage {
            kind: "source"
            argument: window.subpages.source ?? ""
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: settingsPage
        SettingsPage {
            section: window.subpages.settings ?? "playback"
            onNavigate: to => window.go(to)
        }
    }
    Component {
        id: placeholderPage
        PlaceholderPage {}
    }

    Connections {
        target: Playlists

        function onCreated(id: real) {
            window.go("playlist:" + id);
        }
        function onRejected(message: string) {
            rejected.show(message);
        }
    }

    ColumnLayout {
        anchors.fill: parent
        visible: !Sources.firstRun
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Sidebar {
                Layout.fillHeight: true
                visible: Shell.regular || Shell.wide
                page: window.page
                onNavigate: to => window.go(to)
            }
            Rail {
                Layout.fillHeight: true
                visible: Shell.compact
                page: window.page
                onNavigate: to => window.go(to)
            }

            Item {
                id: pageArea
                Layout.fillWidth: true
                Layout.fillHeight: true

                ParallelAnimation {
                    id: arrival

                    OpacityAnimator {
                        id: arrivalFade
                        from: 0
                        to: 1
                        duration: Motion.fade
                        easing.bezierCurve: Motion.standard
                    }
                    YAnimator {
                        id: arrivalRise
                        from: 8 * Motion.travel
                        to: 0
                        duration: Motion.move
                        easing.bezierCurve: Motion.standard
                    }
                }

                // A flight's pages: the one leaving fades out, the one coming waits for the copy.
                SequentialAnimation {
                    id: pageOut

                    OpacityAnimator {
                        id: leaveFade
                        to: 0
                        duration: Motion.exit
                    }
                    ScriptAction {
                        script: {
                            if (window.leaving !== null)
                                window.leaving.opacity = 1;
                            window.leaving = null;
                        }
                    }
                }
                SequentialAnimation {
                    id: pageIn

                    PauseAnimation {
                        id: arriveWait
                    }
                    OpacityAnimator {
                        id: arriveFade
                        from: 0
                        to: 1
                        duration: Motion.fade
                        easing.bezierCurve: Motion.standard
                    }
                }
                SequentialAnimation {
                    id: flight

                    ParallelAnimation {
                        XAnimator {
                            id: flyX
                            target: flyer
                            duration: Motion.flight
                            easing.bezierCurve: Motion.standard
                        }
                        YAnimator {
                            id: flyY
                            target: flyer
                            duration: Motion.flight
                            easing.bezierCurve: Motion.standard
                        }
                        ScaleAnimator {
                            id: flyScale
                            target: flyer
                            to: 1
                            duration: Motion.flight
                            easing.bezierCurve: Motion.standard
                        }
                    }
                    // Back home, the copy stays while the grid fades in around it.
                    PauseAnimation {
                        id: flightHold
                    }
                    ScriptAction {
                        script: window.landed()
                    }
                }

                Repeater {
                    id: pages
                    model: window.sections

                    Loader {
                        id: holder

                        required property string modelData

                        // Not anchored, so it can rise into place.
                        width: parent.width
                        height: parent.height
                        active: window.visited[modelData] === true
                        visible: window.section === modelData || window.leaving === holder
                        sourceComponent: ({
                                tracks: tracksPage,
                                albums: albumsPage,
                                album: albumPage,
                                artists: artistsPage,
                                artist: artistPage,
                                genres: genresPage,
                                genre: genrePage,
                                sources: sourcesPage,
                                source: sourcePage,
                                playlist: playlistPage,
                                queue: queuePage,
                                settings: settingsPage
                            })[modelData] ?? placeholderPage
                        onLoaded: {
                            if (item instanceof PlaceholderPage)
                                item.title = Qt.binding(() => Tr.t[modelData]);
                        }
                    }
                }

                // A cover flying between a grid card and the header of the page it opens.
                Cover {
                    id: flyer
                    visible: false
                    transformOrigin: Item.TopLeft
                }

                RejectedToast {
                    id: rejected
                    anchors.right: parent.right
                    anchors.rightMargin: 24
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 20
                }

                // Tells where a queued track went, when its place in Up next is out of sight.
                Toast {
                    id: queuedToast

                    property string text

                    anchors.right: parent.right
                    anchors.rightMargin: 24
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 20
                    width: Math.min(16 + said.implicitWidth + 12 + show.width + 5, 480, parent.width - 48)
                    height: 44
                    radius: 10
                    color: Theme.surface
                    border.width: 1
                    border.color: Theme.line2
                    Accessible.role: Accessible.AlertMessage
                    Accessible.name: text

                    Label {
                        id: said
                        x: 16
                        width: show.x - 12 - x
                        anchors.verticalCenter: parent.verticalCenter
                        text: queuedToast.text
                        elide: Text.ElideRight
                        color: Theme.text
                        font.pixelSize: 13
                    }
                    FlatButton {
                        id: show
                        anchors.right: parent.right
                        anchors.rightMargin: 5
                        anchors.verticalCenter: parent.verticalCenter
                        implicitHeight: 34
                        color: Theme.text
                        text: Tr.t.open_queue
                        onClicked: {
                            queuedToast.dismiss();
                            window.go("queue");
                        }
                    }
                }
            }

            // The queue page shows all of it already.
            NowPlayingPanel {
                id: panel
                Layout.fillHeight: true
                visible: Shell.wide && window.section !== "queue"
                onNavigate: to => window.go(to)
            }
        }

        PlayerBar {
            id: playerBar
            Layout.fillWidth: true
            queueOpen: window.page === "queue"
            onToggleQueue: window.toggleQueue()
            onMiniRequested: {
                window.hide();
                mini.show();
                mini.raise();
                mini.requestActivate();
            }
        }
    }

    // The mouse's back and forward buttons, wherever they are pressed; other buttons reach what
    // is under it.
    MouseArea {
        anchors.fill: parent
        enabled: !Sources.firstRun
        acceptedButtons: Qt.BackButton | Qt.ForwardButton
        onPressed: mouse => {
            if (mouse.button === Qt.BackButton)
                History.back();
            else
                History.forward();
        }
    }

    // Stands in for this window while it hides.
    MiniPlayer {
        id: mini
        onExpandRequested: {
            mini.hide();
            window.show();
            window.raise();
            window.requestActivate();
        }
    }

    // Narrow windows keep the navigation here, sliding in from the left over 250 ms and out over
    // 150 ms, the scrim following it. Under reduced motion it fades in and out where it stands.
    Drawer {
        id: navigation
        width: Math.min(280, window.width - 56)
        height: window.height
        edge: Qt.LeftEdge
        interactive: Shell.narrow
        padding: 0

        enter: Transition {
            NumberAnimation {
                property: "position"
                to: 1
                duration: Motion.reduced ? 0 : Motion.enter
                easing.bezierCurve: Motion.standard
            }
            NumberAnimation {
                property: "opacity"
                from: Motion.reduced ? 0 : 1
                to: 1
                duration: Motion.fade
            }
        }
        exit: Transition {
            SequentialAnimation {
                NumberAnimation {
                    property: "opacity"
                    to: Motion.reduced ? 0 : 1
                    duration: Motion.reduced ? Motion.exit : 0
                }
                NumberAnimation {
                    property: "position"
                    to: 0
                    duration: Motion.reduced ? 0 : Motion.fade
                    easing.bezierCurve: Motion.leaving
                }
            }
        }

        Overlay.modal: Rectangle {
            color: Theme.shadow
        }
        background: null

        Sidebar {
            anchors.fill: parent
            page: window.page
            onNavigate: to => {
                window.go(to);
                navigation.close();
            }
        }
    }

    Connections {
        target: History

        // Going back or forward from it closes it too.
        function onPageChanged() {
            navigation.close();
        }
    }

    // After the pages are laid out and before the frame shows them.
    Connections {
        target: window
        enabled: window.pendingFlight !== null

        function onAfterAnimating() {
            window.fly();
        }
    }

    Connections {
        target: Shell

        function onNavigationRequested() {
            navigation.open();
        }
        // A track queued where Up next shows comes in there; out of sight, a toast tells.
        function onQueued(title: string, next: bool) {
            if (window.upNextShows(next))
                return;
            queuedToast.text = (next ? Tr.t.plays_next : Tr.t.added_to_queue).arg(title);
            queuedToast.pop();
            playerBar.ping();
        }
        function onNarrowChanged() {
            if (!Shell.narrow)
                navigation.close();
        }
    }

    // The first run asks where the music is before the library shows.
    Loader {
        anchors.fill: parent
        active: Sources.firstRun
        sourceComponent: ImportPage {}
    }
}
