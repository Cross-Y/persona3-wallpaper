import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: panel
    modal: true
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(550, parent ? parent.width - 44 : 550)
    padding: 22
    closePolicy: Popup.CloseOnEscape
    title: "Optimize video before playback?"
    standardButtons: Dialog.NoButton

    property var controller: null
    property var wallpaper: null
    property string monitorName: ""
    property var mediaInfo: ({})
    property bool working: false
    property string progressMessage: ""
    signal useOptimized(var wallpaper, string monitorName)
    signal useOriginal(var wallpaper, string monitorName)

    function openFor(item, monitor, info) {
        if (working && controller && wallpaper)
            controller.cancel_video_preparation(String(wallpaper.path), monitorName)
        wallpaper = item
        monitorName = monitor
        mediaInfo = info
        working = false
        progressMessage = info.cached
            ? "A completed optimized copy is already available."
            : "The original wallpaper will remain active while the new copy is prepared."
        open()
    }

    function startOptimization() {
        if (!controller || !wallpaper || working) return
        const state = String(controller.prepare_video_before_apply(String(wallpaper.path), monitorName))
        if (state === "ready") {
            useOptimized(wallpaper, monitorName)
            close()
        } else if (state === "working") {
            working = true
            progressMessage = "Optimizing video… The current wallpaper stays visible."
            pollTimer.start()
        } else {
            progressMessage = state.replace(/^ERROR:/, "Optimization unavailable: ")
        }
    }

    onClosed: {
        pollTimer.stop()
        if (working && controller && wallpaper)
            controller.cancel_video_preparation(String(wallpaper.path), monitorName)
        working = false
    }

    Timer {
        id: pollTimer
        interval: 450
        repeat: true
        running: false
        onTriggered: {
            if (!panel.controller || !panel.wallpaper) { stop(); return }
            const state = String(panel.controller.video_preparation_status(String(panel.wallpaper.path), panel.monitorName))
            if (state === "ready") {
                stop()
                panel.working = false
                panel.controller.cancel_video_preparation(String(panel.wallpaper.path), panel.monitorName)
                panel.useOptimized(panel.wallpaper, panel.monitorName)
                panel.close()
            } else if (state !== "working") {
                stop()
                panel.working = false
                panel.progressMessage = state.replace(/^ERROR:/, "Conversion failed: ")
            }
        }
    }

    background: Rectangle {
        radius: 17
        color: "#0D213D"
        border.width: 1
        border.color: "#64B6FC"
    }

    contentItem: ColumnLayout {
        spacing: 15
        Label {
            Layout.fillWidth: true
            color: "#D9EFFF"
            wrapMode: Text.Wrap
            text: panel.wallpaper ? String(panel.wallpaper.name || "Selected video") : "Selected video"
            font.pixelSize: 17
            font.bold: true
        }
        Label {
            Layout.fillWidth: true
            color: "#B8D9F4"
            wrapMode: Text.Wrap
            text: panel.mediaInfo.source_width + " × " + panel.mediaInfo.source_height
                + " source  →  " + panel.mediaInfo.screen_width + " × " + panel.mediaInfo.screen_height
                + " monitor\nThe video exceeds your screen resolution. Converting first can reduce playback load."
        }
        BusyIndicator { Layout.alignment: Qt.AlignHCenter; running: panel.working; visible: panel.working }
        Label { Layout.fillWidth: true; color: "#8BD5FF"; wrapMode: Text.Wrap; text: panel.progressMessage }
        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            Button {
                text: panel.mediaInfo.cached ? "Use optimized copy" : "Optimize, then apply"
                Layout.fillWidth: true
                enabled: !panel.working
                onClicked: panel.startOptimization()
            }
            Button {
                text: "Play original"
                enabled: !panel.working
                onClicked: { panel.useOriginal(panel.wallpaper, panel.monitorName); panel.close() }
            }
            Button { text: panel.working ? "Cancel conversion" : "Cancel"; onClicked: panel.close() }
        }
    }
}
