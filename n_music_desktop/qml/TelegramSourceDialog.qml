pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import NMusic

// Adds a Telegram chat as a source. Signs in first when needed, with the account's phone
// number, the code Telegram sends and the account's password; then lists the account's chats
// to pick one, or finds a public one by its @name or t.me link.
DialogPopup {
    id: dialog

    /// The locations of the chats added already: they show as added, and are not picked again.
    property var taken: []
    /// Only signs in: it closes once signed in, without picking a chat.
    property bool signInOnly: false

    /// The chat at `location` was picked, to be called `name`.
    signal chosen(string location, string name)

    readonly property string step: Telegram.status
    readonly property bool picking: step === "signedIn" && !signInOnly
    /// A step ended while it was open: an error is that step's, not one from before.
    property bool answered: false

    /// What `chat` is called: Saved Messages has no name of its own.
    function title(chat: var): string {
        return chat.kind === "saved" ? Tr.t.telegram_saved_messages : chat.title;
    }

    /// What kind of chat `chat` is, and its username.
    function detail(chat: var): string {
        const kind = ({
                saved: Tr.t.telegram_kind_saved,
                user: Tr.t.telegram_kind_user,
                bot: Tr.t.telegram_kind_bot,
                group: Tr.t.telegram_kind_group,
                channel: Tr.t.telegram_kind_channel
            })[chat.kind] ?? "";
        return chat.username !== "" ? kind + " · @" + chat.username : kind;
    }

    /// Why a request failed, from `Telegram.error` and its detail.
    function problem(error: string, detail: string): string {
        if (error === "")
            return "";
        return (Tr.t["telegram_error_" + error] ?? Tr.t.telegram_error_failed).arg(detail);
    }

    function pick(chat: var) {
        if (taken.includes(chat.location))
            return;
        close();
        chosen(chat.location, title(chat));
    }

    /// Sends what the step asks for.
    function next() {
        const text = field.text.trim();
        if (Telegram.busy || text === "")
            return;
        if (step === "signedOut")
            Telegram.sendPhone(text);
        else if (step === "codeSent")
            Telegram.sendCode(text);
        else if (step === "passwordNeeded")
            Telegram.sendPassword(text);
    }

    function focusStep() {
        if (picking)
            search.focusInput();
        else
            field.forceActiveFocus();
    }

    width: Math.min(picking ? 560 : 420, parent.width - 32)
    height: picking ? Math.min(600, parent.height - 32) : implicitHeight
    focus: true
    padding: 20
    closePolicy: Popup.CloseOnEscape
    onAboutToShow: {
        answered = false;
        field.text = "";
        search.text = "";
        if (picking)
            Telegram.find("");
    }
    onOpened: focusStep()

    Connections {
        target: Telegram

        function onBusyChanged() {
            if (!Telegram.busy && dialog.opened)
                dialog.answered = true;
        }

        function onStatusChanged() {
            field.text = "";
            if (!dialog.opened)
                return;
            if (Telegram.status === "signedIn") {
                if (dialog.signInOnly) {
                    dialog.close();
                    return;
                }
                Telegram.find(search.text);
            }
            Qt.callLater(dialog.focusStep);
        }
    }

    // Asks for chats once typing paused.
    Timer {
        id: typing
        interval: 300
        onTriggered: Telegram.find(search.text)
    }

    contentItem: ColumnLayout {
        spacing: 14

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Label {
                Layout.fillWidth: true
                text: dialog.picking ? Tr.t.add_telegram_source : Tr.t.telegram_sign_in
                wrapMode: Text.Wrap
                color: Theme.text
                font.pixelSize: 17
                font.weight: Font.Bold
            }
            IconButton {
                Layout.alignment: Qt.AlignTop
                size: 30
                iconSize: 16
                iconName: "close"
                text: Tr.t.cancel
                onClicked: dialog.close()
            }
        }

        // Signing in.
        ColumnLayout {
            Layout.fillWidth: true
            visible: !dialog.picking
            spacing: 14
            Keys.onReturnPressed: dialog.next()
            Keys.onEnterPressed: dialog.next()

            Label {
                Layout.fillWidth: true
                text: {
                    switch (dialog.step) {
                    case "codeSent":
                        return Tr.t.telegram_code_message.arg("+" + Telegram.phone);
                    case "passwordNeeded":
                        return Tr.t.telegram_password_message;
                    case "signedIn":
                        return Tr.t.telegram_signed_in_as.arg(Telegram.account);
                    }
                    return Tr.t.telegram_phone_message;
                }
                wrapMode: Text.Wrap
                color: Theme.text2
                font.pixelSize: 14
            }
            Label {
                Layout.fillWidth: true
                visible: dialog.step === "passwordNeeded" && Telegram.hint !== ""
                text: Tr.t.telegram_password_hint.arg(Telegram.hint)
                wrapMode: Text.Wrap
                color: Theme.text3
                font.pixelSize: 13
            }
            TextBox {
                id: field
                Layout.fillWidth: true
                visible: dialog.step !== "signedIn"
                enabled: !Telegram.busy
                echoMode: dialog.step === "passwordNeeded" ? TextInput.Password : TextInput.Normal
                inputMethodHints: dialog.step === "passwordNeeded" ? Qt.ImhSensitiveData | Qt.ImhNoPredictiveText : Qt.ImhDigitsOnly
                placeholderText: ({
                        signedOut: Tr.t.telegram_phone_placeholder,
                        codeSent: Tr.t.telegram_code_placeholder,
                        passwordNeeded: Tr.t.telegram_password_placeholder
                    })[dialog.step] ?? ""
                Accessible.name: placeholderText
            }
            Label {
                Layout.fillWidth: true
                visible: text !== ""
                text: dialog.answered ? dialog.problem(Telegram.error, Telegram.errorDetail) : ""
                wrapMode: Text.Wrap
                color: Theme.danger
                font.pixelSize: 13
            }
            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: 4
                spacing: 8

                PillButton {
                    implicitHeight: 38
                    visible: dialog.step === "codeSent" || dialog.step === "passwordNeeded"
                    enabled: !Telegram.busy
                    text: Tr.t.telegram_other_number
                    onClicked: Telegram.signOut()
                }
                Item {
                    Layout.fillWidth: true
                }
                PillButton {
                    implicitHeight: 38
                    text: dialog.step === "signedIn" ? Tr.t.done : Tr.t.cancel
                    onClicked: dialog.close()
                }
                PillButton {
                    implicitHeight: 38
                    visible: dialog.step !== "signedIn"
                    primary: true
                    enabled: !Telegram.busy && field.text.trim() !== ""
                    text: Telegram.busy ? Tr.t.telegram_waiting : dialog.step === "signedOut" ? Tr.t.telegram_send_code : Tr.t.telegram_sign_in_action
                    onClicked: dialog.next()
                }
            }
        }

        // Picking a chat.
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: dialog.picking
            spacing: 12

            Label {
                Layout.fillWidth: true
                text: Tr.t.telegram_pick_message.arg(Telegram.account)
                wrapMode: Text.Wrap
                color: Theme.text2
                font.pixelSize: 14
            }
            SearchField {
                id: search
                Layout.fillWidth: true
                placeholder: Tr.t.telegram_search_placeholder
                onTextChanged: typing.restart()
                Keys.onDownPressed: {
                    chats.forceActiveFocus();
                    chats.currentIndex = 0;
                }
                Keys.onReturnPressed: {
                    if (chats.count > 0)
                        dialog.pick(Telegram.chats[0]);
                }
            }
            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: 10
                color: "transparent"
                border.width: 1
                border.color: Theme.line

                ListView {
                    id: chats
                    anchors.fill: parent
                    anchors.margins: 1
                    topMargin: 6
                    bottomMargin: 6
                    clip: true
                    currentIndex: -1
                    boundsBehavior: Flickable.StopAtBounds
                    model: Telegram.chats
                    Accessible.name: Tr.t.telegram_chats
                    Keys.onReturnPressed: {
                        if (currentIndex >= 0)
                            dialog.pick(Telegram.chats[currentIndex]);
                    }
                    Keys.onEnterPressed: {
                        if (currentIndex >= 0)
                            dialog.pick(Telegram.chats[currentIndex]);
                    }
                    Keys.onUpPressed: event => {
                        if (currentIndex <= 0) {
                            currentIndex = -1;
                            search.focusInput();
                        } else {
                            event.accepted = false;
                        }
                    }

                    delegate: AbstractButton {
                        id: chat

                        required property var modelData
                        required property int index
                        readonly property bool added: dialog.taken.includes(modelData.location)
                        readonly property bool marked: ListView.isCurrentItem && chats.activeFocus

                        x: 6
                        width: ListView.view.width - 12
                        height: 52
                        leftPadding: 10
                        rightPadding: 10
                        hoverEnabled: true
                        focusPolicy: Qt.NoFocus
                        enabled: !added
                        text: dialog.title(modelData)
                        Accessible.description: dialog.detail(modelData)
                        onClicked: dialog.pick(modelData)

                        background: Rectangle {
                            radius: 8
                            color: chat.down || chat.marked ? Theme.selected : chat.hovered ? Theme.hover : Qt.alpha(Theme.hover, 0)

                            TintFade on color {}
                        }
                        contentItem: RowLayout {
                            spacing: 12

                            Icon {
                                name: ({
                                        saved: "tag",
                                        user: "artist",
                                        bot: "artist",
                                        group: "queue",
                                        channel: "send"
                                    })[chat.modelData.kind] ?? "send"
                                size: 18
                                color: chat.hovered || chat.marked ? Theme.accentText : Theme.text2

                                ColorFade on color {}
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1

                                Label {
                                    Layout.fillWidth: true
                                    text: chat.text
                                    elide: Text.ElideRight
                                    color: chat.added ? Theme.text2 : Theme.text
                                    font.pixelSize: 14
                                    font.weight: Font.Medium
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: dialog.detail(chat.modelData)
                                    elide: Text.ElideRight
                                    color: Theme.text3
                                    font.pixelSize: 12
                                }
                            }
                            Badge {
                                visible: chat.added
                                text: Tr.t.added
                            }
                        }
                    }

                    ScrollBar.vertical: ThinScrollBar {}
                }

                Label {
                    anchors.centerIn: parent
                    width: parent.width - 48
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    color: Theme.text3
                    font.pixelSize: 14
                    visible: chats.count === 0
                    text: {
                        if (Telegram.searching)
                            return Tr.t.telegram_searching;
                        if (Telegram.searchError !== "")
                            return dialog.problem(Telegram.searchError, Telegram.searchErrorDetail);
                        return Telegram.query !== "" ? Tr.t.telegram_no_match : Tr.t.telegram_no_chats;
                    }
                }
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Label {
                    Layout.fillWidth: true
                    text: Tr.t.telegram_pick_hint
                    wrapMode: Text.Wrap
                    color: Theme.text3
                    font.pixelSize: 13
                }
                PillButton {
                    implicitHeight: 38
                    text: Tr.t.cancel
                    onClicked: dialog.close()
                }
            }
        }
    }
}
