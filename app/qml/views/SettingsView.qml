import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property var controller
    // Shared state outlives this temporary page (tray controls must keep working).
    signal uiHover()
    signal uiSelect()
    property var appState
    property bool playbackControlsAvailable: true
    property bool focusControlsAvailable: true
    property string optimizerStatus: ""
    readonly property bool autoVideoOptimize: appState ? appState.autoVideoOptimize : true
    readonly property bool engineActive: appState ? appState.engineActive : true
    readonly property bool restoreOnStart: appState ? appState.restoreOnStart : true
    readonly property bool focusPause: appState ? appState.focusPause : true
    readonly property bool startOnLogin: appState ? appState.startOnLogin : false
    readonly property bool backgroundMode: appState ? appState.backgroundMode : true
    readonly property bool soundEnabled: appState ? appState.soundEnabled : false
    readonly property bool audioDesktopOnly: appState ? appState.audioDesktopOnly : false
    readonly property bool uiSoundsEnabled: appState ? appState.uiSoundsEnabled : true
    readonly property real uiSoundVolume: appState ? appState.uiSoundVolume : 0.55
    readonly property string statusMessage: appState ? appState.statusMessage : ""
    readonly property int cardMargin: width < 420 ? 12 : 18

    // Clicks request changes, but only persisted appState determines the indicator.
    // A non-checkable button cannot get visually out of sync with backend failures
    // or with pause/resume actions from the tray.
    component AccentSwitch: Button {
        id: control
        property bool active: false
        property string accessibleLabel: "Option"
        checkable: false
        onHoveredChanged: if (hovered && enabled) root.uiHover()
        onPressed: root.uiSelect()
        implicitWidth: 54
        implicitHeight: 32
        padding: 0
        focusPolicy: Qt.StrongFocus
        Accessible.role: Accessible.CheckBox
        Accessible.name: control.accessibleLabel
        Accessible.checked: control.active
        contentItem: Rectangle {
            implicitWidth: 52
            implicitHeight: 30
            x: 0
            y: (control.height - height) / 2
            radius: height / 2
            color: control.active ? "#168FE0" : "#263B56"
            border.width: 1
            border.color: control.active ? "#76D2FF" : "#66809D"
            Rectangle {
                anchors.fill: parent
                radius: parent.radius
                color: "transparent"
                border.width: control.activeFocus ? 2 : 0
                border.color: "#FFFFFF"
            }
            Rectangle {
                width: 22
                height: 22
                radius: 11
                y: 4
                x: control.active ? 26 : 4
                color: "#F4FAFF"
                border.color: control.active ? "#D8F3FF" : "#AFC4D8"
                Behavior on x { NumberAnimation { duration: 150; easing.type: Easing.OutCubic } }
            }
            Behavior on color { ColorAnimation { duration: 150 } }
        }
        background: Rectangle {
            color: "transparent"
            radius: 16
        }
        opacity: enabled ? 1.0 : 0.45
    }

    function loadSettings(s) {
        if (!root.appState) return
        root.appState.autoVideoOptimize = s.auto_video_optimize !== false
        root.appState.engineActive = !Boolean(s.paused)
        root.appState.restoreOnStart = s.restore_on_start !== false
        root.appState.focusPause = Boolean(s.focus_pause)
        root.appState.startOnLogin = Boolean(s.start_on_login)
        root.appState.backgroundMode = s.background_mode !== false
        root.appState.soundEnabled = Boolean(s.sound_enabled)
        root.appState.audioDesktopOnly = Boolean(s.audio_desktop_only)
        root.appState.uiSoundsEnabled = s.ui_sounds_enabled !== false
        root.appState.uiSoundVolume = s.ui_sound_volume === undefined ? 0.55 : Number(s.ui_sound_volume)
    }
    function saveToggle(method, value, propertyName) {
        if (!root.controller || !root.appState) return
        try {
            const result = String(root.controller[method](value))
            if (result === "OK") {
                root.appState[propertyName] = value
                root.appState.statusMessage = "Settings saved."
            } else {
                root.appState.statusMessage = result.replace(/^ERROR:\s*/, "")
            }
        } catch (e) { root.appState.statusMessage = String(e) }
    }
    function saveAudio(enabled, desktopOnly) {
        if (!root.controller || !root.appState) return
        try {
            const result = String(root.controller.set_audio(enabled, desktopOnly))
            if (result === "OK") {
                root.appState.soundEnabled = enabled
                root.appState.audioDesktopOnly = desktopOnly
                root.appState.statusMessage = "Audio settings saved."
            } else { root.appState.statusMessage = result.replace(/^ERROR:\s*/, "") }
        } catch (e) { root.appState.statusMessage = String(e) }
        // ComboBox can change its own currentIndex before onActivated is called;
        // always reconcile it to the persisted application state.
        audioMode.currentIndex = root.audioDesktopOnly ? 1 : 0
    }

    function saveUiSounds(enabled) {
        if (!root.controller || !root.appState) return
        try {
            const result = String(root.controller.set_ui_sounds(enabled))
            if (result === "OK") {
                root.appState.uiSoundsEnabled = enabled
                root.appState.statusMessage = "Menu sound settings saved."
            } else root.appState.statusMessage = result.replace(/^ERROR:\s*/, "")
        } catch (e) { root.appState.statusMessage = String(e) }
    }
    function saveUiSoundVolume(volume) {
        if (!root.controller || !root.appState) return
        try {
            const result = String(root.controller.set_ui_sound_volume(volume))
            if (result === "OK") root.appState.uiSoundVolume = volume
            else root.appState.statusMessage = result.replace(/^ERROR:\s*/, "")
        } catch (e) { root.appState.statusMessage = String(e) }
    }

    Timer {
        interval: 1800
        running: root.visible
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            if (root.controller) root.optimizerStatus = String(root.controller.video_optimization_status())
        }
    }

    Flickable {
        anchors.fill: parent
        contentHeight: content.implicitHeight
        clip: true
        ColumnLayout {
            id: content
            width: parent.width
            spacing: 16
            Label { text: "Settings"; color: "#F0F7FF"; font.pixelSize: 28; font.bold: true }
            Label { text: "Automatic playback, focus behavior and resource saving."; color: "#B8D0EA"; font.pixelSize: 14 }
            Label {
                Layout.fillWidth: true
                visible: !root.playbackControlsAvailable
                text: "This desktop applies static wallpapers through its native settings. Video playback, pause and audio controls are not available."
                color: "#FFD29A"; wrapMode: Text.WordWrap; font.pixelSize: 13
            }

            Rectangle {
                Layout.fillWidth: true; implicitHeight: general.implicitHeight+root.cardMargin*2; radius: 18
                color: "#A5102545"; border.color: "#668DBB"
                ColumnLayout {
                    id: general; anchors.fill: parent; anchors.margins: root.cardMargin; spacing: 12
                    Label { text: "PERFORMANCE"; color: "#73C5FF"; font.pixelSize: 13; font.bold: true; font.letterSpacing: 2 }
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: 10
                        ColumnLayout { Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 1; spacing: 3
                            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Wallpaper engine"; color: "#F0F7FF"; font.pixelSize: 16; font.bold: true }
                            Label { Layout.fillWidth: true; text: "Pause all managed wallpapers manually."; color: "#B8D0EA"; wrapMode: Text.WordWrap; font.pixelSize: 13 }
                        }
                        AccentSwitch { Layout.alignment: Qt.AlignRight | Qt.AlignVCenter; Layout.minimumWidth: implicitWidth; id: engineSwitch; enabled: root.playbackControlsAvailable; active: root.engineActive; accessibleLabel: "Wallpaper engine"; onClicked: root.saveToggle("set_paused",!root.engineActive,"engineActive") }
                    }
                    Rectangle { Layout.fillWidth: true; height: 1; color: "#426080" }
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: 10
                        ColumnLayout { Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 1; spacing: 3
                            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Optimize video for monitor resolution"; color: "#F0F7FF"; font.pixelSize: 15; font.bold: true }
                            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Automatically convert oversized videos in the background (e.g. 4K to 1080p). Keep originals untouched; reuse optimized versions to reduce playback load. Requires FFmpeg and FFprobe."; color: "#B8D0EA"; font.pixelSize: 13 }
                        }
                        AccentSwitch { Layout.alignment: Qt.AlignRight | Qt.AlignVCenter; Layout.minimumWidth: implicitWidth; active: root.autoVideoOptimize; accessibleLabel: "Optimize video for monitor"; onClicked: root.saveToggle("set_auto_video_optimize", !root.autoVideoOptimize, "autoVideoOptimize") }
                    }
                    Label { Layout.fillWidth: true; text: root.optimizerStatus; color: "#8CD5FF"; wrapMode: Text.WordWrap; font.pixelSize: 12 }
                    Rectangle { Layout.fillWidth: true; height: 1; color: "#426080" }
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: 10
                        ColumnLayout { Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 1; spacing: 3
                            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Pause when a window is visible"; color: "#F0F7FF"; font.pixelSize: 15; font.bold: true }
                            Label { Layout.fillWidth: true; text: "Automatically freeze the wallpaper on each monitor while one or more windows are visible. Empty monitors keep animating."; color: "#B8D0EA"; wrapMode: Text.WordWrap; font.pixelSize: 13 }
                        }
                        AccentSwitch { Layout.alignment: Qt.AlignRight | Qt.AlignVCenter; Layout.minimumWidth: implicitWidth; id: focusSwitch; enabled: root.focusControlsAvailable; active: root.focusPause; accessibleLabel: "Pause when windows are visible"; onClicked: root.saveToggle("set_focus_pause",!root.focusPause,"focusPause") }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true; implicitHeight: startup.implicitHeight+root.cardMargin*2; radius: 18
                color: "#A5102545"; border.color: "#668DBB"
                ColumnLayout {
                    id: startup; anchors.fill: parent; anchors.margins: root.cardMargin; spacing: 12
                    Label { text: "STARTUP"; color: "#73C5FF"; font.pixelSize: 13; font.bold: true; font.letterSpacing: 2 }
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: 10
                        ColumnLayout { Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 1; spacing: 3
                            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Start app on login"; color: "#F0F7FF"; font.pixelSize: 15; font.bold: true }
                            Label { Layout.fillWidth: true; text: "Register Persona 3 Wallpaper in your desktop autostart."; color: "#B8D0EA"; wrapMode: Text.WordWrap; font.pixelSize: 13 }
                        }
                        AccentSwitch { Layout.alignment: Qt.AlignRight | Qt.AlignVCenter; Layout.minimumWidth: implicitWidth; id: loginSwitch; active: root.startOnLogin; accessibleLabel: "Start app on login"; onClicked: root.saveToggle("set_start_on_login",!root.startOnLogin,"startOnLogin") }
                    }
                    Rectangle { Layout.fillWidth: true; height: 1; color: "#426080" }
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: 10
                        ColumnLayout { Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 1; spacing: 3
                            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Run in background"; color: "#F0F7FF"; font.pixelSize: 15; font.bold: true }
                            Label { Layout.fillWidth: true; text: "Closing the window hides it in the system tray. Left click opens the app; right click shows playback controls."; color: "#B8D0EA"; wrapMode: Text.WordWrap; font.pixelSize: 13 }
                        }
                        AccentSwitch { Layout.alignment: Qt.AlignRight | Qt.AlignVCenter; Layout.minimumWidth: implicitWidth; id: backgroundSwitch; active: root.backgroundMode; accessibleLabel: "Run in background"; onClicked: root.saveToggle("set_background_mode",!root.backgroundMode,"backgroundMode") }
                    }
                    Rectangle { Layout.fillWidth: true; height: 1; color: "#426080" }
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: 10
                        ColumnLayout { Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 1; spacing: 3
                            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Restore wallpapers on launch"; color: "#F0F7FF"; font.pixelSize: 15; font.bold: true }
                            Label { Layout.fillWidth: true; text: "Restore the last wallpaper saved for each connected monitor."; color: "#B8D0EA"; wrapMode: Text.WordWrap; font.pixelSize: 13 }
                        }
                        AccentSwitch { Layout.alignment: Qt.AlignRight | Qt.AlignVCenter; Layout.minimumWidth: implicitWidth; id: restoreSwitch; active: root.restoreOnStart; accessibleLabel: "Restore on launch"; onClicked: root.saveToggle("set_restore_on_start",!root.restoreOnStart,"restoreOnStart") }
                    }
                }
            }



            Rectangle {
                Layout.fillWidth: true; implicitHeight: audio.implicitHeight+root.cardMargin*2; radius: 18
                color: "#A5102545"; border.color: "#668DBB"
                ColumnLayout {
                    id: audio; anchors.fill: parent; anchors.margins: root.cardMargin; spacing: 12
                    Label { text: "AUDIO"; color: "#73C5FF"; font.pixelSize: 13; font.bold: true; font.letterSpacing: 2 }
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: 10
                        ColumnLayout { Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 1; spacing: 3
                            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Enable video sound"; color: "#F0F7FF"; font.pixelSize: 15; font.bold: true }
                            Label { Layout.fillWidth: true; text: "Allow videos with an audio track to play sound."; color: "#B8D0EA"; wrapMode: Text.WordWrap; font.pixelSize: 13 }
                        }
                        AccentSwitch { Layout.alignment: Qt.AlignRight | Qt.AlignVCenter; Layout.minimumWidth: implicitWidth; id: soundSwitch; enabled: root.playbackControlsAvailable; active: root.soundEnabled; accessibleLabel: "Enable video sound"; onClicked: root.saveAudio(!root.soundEnabled,root.audioDesktopOnly) }
                    }
                    Label { text: "Sound playback mode"; color: "#F0F7FF"; font.pixelSize: 14; font.bold: true; enabled: root.soundEnabled }
                    ComboBox {
                        id: audioMode
                        onHoveredChanged: if (hovered && enabled) root.uiHover()
                        Layout.fillWidth: true
                        implicitHeight: 46
                        enabled: root.soundEnabled && root.focusControlsAvailable
                        model: ["Always", "Only when desktop is focused"]
                        currentIndex: root.audioDesktopOnly ? 1 : 0
                        spacing: 8
                        onActivated: index => { root.uiSelect(); root.saveAudio(root.soundEnabled,index===1) }
                        Connections {
                            target: root.appState
                            function onAudioDesktopOnlyChanged() { audioMode.currentIndex = root.audioDesktopOnly ? 1 : 0 }
                        }
                        contentItem: Label {
                            leftPadding: 14
                            rightPadding: 34
                            text: audioMode.displayText
                            color: audioMode.enabled ? "#F0F7FF" : "#91A9C2"
                            font.pixelSize: 14
                            font.weight: Font.DemiBold
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                        indicator: Label {
                            text: "⌄"
                            color: audioMode.enabled ? "#8ED7FF" : "#66809D"
                            font.pixelSize: 22
                            font.bold: true
                            anchors.right: parent.right
                            anchors.rightMargin: 14
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        background: Rectangle {
                            radius: 13
                            color: audioMode.enabled ? "#172F4D" : "#14243A"
                            border.width: 1
                            border.color: audioMode.enabled ? (audioMode.hovered ? "#62C5FF" : "#52799E") : "#344A62"
                        }
                        popup: Popup {
                            y: audioMode.height + 6
                            width: audioMode.width
                            padding: 6
                            implicitHeight: contentItem.implicitHeight + padding * 2
                            background: Rectangle {
                                radius: 14
                                color: "#10233B"
                                border.color: "#52799E"
                                border.width: 1
                            }
                            contentItem: ListView {
                                clip: true
                                implicitHeight: contentHeight
                                model: audioMode.popup.visible ? audioMode.delegateModel : null
                                currentIndex: audioMode.highlightedIndex
                                spacing: 3
                                ScrollIndicator.vertical: ScrollIndicator {}
                            }
                        }
                        delegate: ItemDelegate {
                            onHoveredChanged: if (hovered && enabled) root.uiHover()
                            width: audioMode.width - 12
                            height: 40
                            highlighted: audioMode.highlightedIndex === index
                            contentItem: Label {
                                text: modelData
                                color: "#F0F7FF"
                                    font.pixelSize: 13
                                font.weight: highlighted ? Font.DemiBold : Font.Medium
                                verticalAlignment: Text.AlignVCenter
                                leftPadding: 10
                            }
                            background: Rectangle {
                                radius: 9
                                color: highlighted ? "#24547A" : (parent.hovered ? "#1A3A5A" : "transparent")
                            }
                        }
                    }
                    Label { Layout.fillWidth: true; text: "When desktop-only mode is selected, audio is muted while an application window is focused. The video itself keeps playing unless monitor occupancy pause is enabled."; color: "#B8D0EA"; wrapMode: Text.WordWrap; font.pixelSize: 13 }
                    Rectangle { Layout.fillWidth: true; height: 1; color: "#426080" }
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 0
                        spacing: 10
                        ColumnLayout {
                            Layout.fillWidth: true; Layout.minimumWidth: 0; Layout.preferredWidth: 1; spacing: 3
                            Label { text: "Menu sound effects"; color: "#F0F7FF"; font.pixelSize: 15; font.bold: true }
                            Label { Layout.fillWidth: true; text: "Play interface sounds for hovering, selecting, switching tabs, opening and closing the window."; color: "#B8D0EA"; wrapMode: Text.WordWrap; font.pixelSize: 13 }
                        }
                        AccentSwitch {
                            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                            Layout.minimumWidth: implicitWidth
                            active: root.uiSoundsEnabled
                            accessibleLabel: "Menu sound effects"
                            onClicked: root.saveUiSounds(!root.uiSoundsEnabled)
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        enabled: root.uiSoundsEnabled
                        Label { text: "Menu sound volume"; color: "#B8D0EA"; font.pixelSize: 13 }
                        Slider {
                            id: menuFxVolumeSlider
                            Layout.fillWidth: true
                            from: 0; to: 1; stepSize: 0.05
                            value: root.uiSoundVolume
                            onMoved: if (root.appState) root.appState.uiSoundVolume = value
                            onPressedChanged: if (!pressed) root.saveUiSoundVolume(value)
                        }
                        Label { text: Math.round(menuFxVolumeSlider.value * 100) + "%"; color: "#F0F7FF"; font.pixelSize: 12; Layout.preferredWidth: 35 }
                    }
                    Label { Layout.fillWidth: true; text: "Interface sounds are separate from video audio. Requires the Qt Multimedia QML module; the application works normally without it."; color: "#9CB7D3"; wrapMode: Text.WordWrap; font.pixelSize: 12 }
                }
            }
            Label { Layout.fillWidth: true; visible: root.statusMessage.length>0; text: root.statusMessage; color: "#9DD5FF"; wrapMode: Text.WordWrap; font.pixelSize: 13 }
        }
    }
}
