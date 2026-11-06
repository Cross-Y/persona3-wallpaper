import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: panel
    modal: true
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(740, parent ? parent.width - 56 : 740)
    padding: 20
    closePolicy: Popup.CloseOnEscape
    title: "Static wallpaper layout"
    standardButtons: Dialog.NoButton

    property var controller: null
    property var wallpaper: null
    property string monitorName: ""
    property var mediaInfo: ({})
    property string mode: "crop"
    property real focusX: 0.5
    property real focusY: 0.5
    property real zoom: 1.0
    property bool working: false
    property string progressMessage: ""
    property string requestedMode: ""
    property real requestedX: 0.5
    property real requestedY: 0.5
    property real requestedZoom: 1.0
    signal imageReady(var wallpaper, string monitorName, string mode, real x, real y, real zoom)
    signal useDesktopDefault(var wallpaper, string monitorName)

    readonly property real sourceW: Math.max(1, Number(mediaInfo.source_width) || 1)
    readonly property real sourceH: Math.max(1, Number(mediaInfo.source_height) || 1)
    readonly property real displayW: Math.max(1, Number(mediaInfo.screen_width) || 1)
    readonly property real displayH: Math.max(1, Number(mediaInfo.screen_height) || 1)
    readonly property real baseScale: Math.max(displayW / sourceW, displayH / sourceH)
    readonly property real cropFracW: Math.min(1, displayW / (sourceW * baseScale * zoom))
    readonly property real cropFracH: Math.min(1, displayH / (sourceH * baseScale * zoom))

    function openFor(item, monitor, info) {
        wallpaper = item
        monitorName = monitor
        mediaInfo = info
        const previous = info.image_layout || {}
        mode = ["crop", "fit", "stretch"].indexOf(previous.mode) >= 0 ? previous.mode : "crop"
        focusX = Math.max(0, Math.min(1, Number(previous.focus_x === undefined ? 0.5 : previous.focus_x)))
        focusY = Math.max(0, Math.min(1, Number(previous.focus_y === undefined ? 0.5 : previous.focus_y)))
        zoom = Math.max(1, Math.min(3, Number(previous.zoom === undefined ? 1 : previous.zoom)))
        working = false
        progressMessage = "Drag the outlined area to choose what stays visible. The original image will be preserved."
        open()
    }

    function cancelCurrent() {
        if (working && controller && wallpaper)
            controller.cancel_image_preparation(String(wallpaper.path), monitorName, requestedMode, requestedX, requestedY, requestedZoom)
        working = false
    }

    function applyLayout() {
        if (!controller || !wallpaper || working) return
        requestedMode = mode
        requestedX = focusX
        requestedY = focusY
        requestedZoom = zoom
        const state = String(controller.prepare_image_layout(String(wallpaper.path), monitorName,
            requestedMode, requestedX, requestedY, requestedZoom))
        if (state === "ready") {
            imageReady(wallpaper, monitorName, requestedMode, requestedX, requestedY, requestedZoom)
            close()
        } else if (state === "working") {
            working = true
            progressMessage = "Preparing the monitor-sized image…"
            pollTimer.start()
        } else {
            progressMessage = state.replace(/^ERROR:/, "Image conversion failed: ")
        }
    }

    onClosed: { pollTimer.stop(); cancelCurrent() }
    Timer {
        id: pollTimer
        interval: 300
        repeat: true
        onTriggered: {
            const state = String(panel.controller.prepare_image_layout(String(panel.wallpaper.path), panel.monitorName,
                panel.requestedMode, panel.requestedX, panel.requestedY, panel.requestedZoom))
            if (state === "ready") {
                stop()
                panel.working = false
                panel.imageReady(panel.wallpaper, panel.monitorName, panel.requestedMode,
                    panel.requestedX, panel.requestedY, panel.requestedZoom)
                panel.close()
            } else if (state !== "working") {
                stop()
                panel.working = false
                panel.progressMessage = state.replace(/^ERROR:/, "Image conversion failed: ")
            }
        }
    }

    background: Rectangle { color: "#0D213D"; radius: 17; border.color: "#64B6FC"; border.width: 1 }
    contentItem: ColumnLayout {
        spacing: 11
        Label {
            Layout.fillWidth: true
            text: panel.mediaInfo.source_width + " × " + panel.mediaInfo.source_height + " image on "
                + panel.mediaInfo.screen_width + " × " + panel.mediaInfo.screen_height + " display"
            color: "#C5E7FF"
            wrapMode: Text.WordWrap
        }
        ComboBox {
            id: layoutMode
            Layout.fillWidth: true
            enabled: !panel.working
            model: ["Crop to fill (movable area)", "Fit entire image (black bars)", "Stretch to screen (may distort)"]
            currentIndex: panel.mode === "crop" ? 0 : (panel.mode === "fit" ? 1 : 2)
            onActivated: index => panel.mode = ["crop", "fit", "stretch"][index]
        }
        Rectangle {
            id: previewSurface
            Layout.fillWidth: true
            Layout.preferredHeight: 276
            color: "#030C19"
            radius: 6
            clip: true
            Image {
                id: previewImage
                anchors.fill: parent
                source: panel.wallpaper ? "file://" + encodeURI(String(panel.wallpaper.path)) : ""
                sourceSize.width: 1000
                asynchronous: true
                cache: false
                fillMode: panel.mode === "stretch" ? Image.Stretch : Image.PreserveAspectFit
            }
            Rectangle {
                id: selection
                visible: panel.mode === "crop" && previewImage.status === Image.Ready
                color: "#164BB6FC"
                border.color: "#C2E8FF"
                border.width: 2
                x: (previewImage.width - previewImage.paintedWidth) / 2
                    + previewImage.paintedWidth * (1 - panel.cropFracW) * panel.focusX
                y: (previewImage.height - previewImage.paintedHeight) / 2
                    + previewImage.paintedHeight * (1 - panel.cropFracH) * panel.focusY
                width: previewImage.paintedWidth * panel.cropFracW
                height: previewImage.paintedHeight * panel.cropFracH
            }
            MouseArea {
                anchors.fill: parent
                enabled: panel.mode === "crop" && !panel.working
                cursorShape: Qt.SizeAllCursor
                function place(x, y) {
                    const left = (previewImage.width - previewImage.paintedWidth) / 2
                    const top = (previewImage.height - previewImage.paintedHeight) / 2
                    const nx = (x - left) / Math.max(1, previewImage.paintedWidth)
                    const ny = (y - top) / Math.max(1, previewImage.paintedHeight)
                    const sx = Math.max(0.00001, 1 - panel.cropFracW)
                    const sy = Math.max(0.00001, 1 - panel.cropFracH)
                    panel.focusX = Math.max(0, Math.min(1, (nx - panel.cropFracW / 2) / sx))
                    panel.focusY = Math.max(0, Math.min(1, (ny - panel.cropFracH / 2) / sy))
                }
                onPressed: mouse => place(mouse.x, mouse.y)
                onPositionChanged: mouse => { if (pressed) place(mouse.x, mouse.y) }
            }
            Label {
                visible: previewImage.status === Image.Error
                anchors.centerIn: parent
                text: "Could not load preview. You can still use the layout controls."
                color: "#FFC0C0"
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            visible: panel.mode === "crop"
            spacing: 4
            RowLayout {
                Label { text: "Zoom"; color: "#C5E7FF"; Layout.preferredWidth: 85 }
                Slider { Layout.fillWidth: true; enabled: !panel.working; from: 1; to: 3; stepSize: 0.05; value: panel.zoom; onMoved: panel.zoom = value }
                Label { text: panel.zoom.toFixed(2) + "×"; color: "#C5E7FF"; Layout.preferredWidth: 48 }
            }
            RowLayout {
                Label { text: "Horizontal"; color: "#C5E7FF"; Layout.preferredWidth: 85 }
                Slider { Layout.fillWidth: true; enabled: !panel.working; from: 0; to: 1; value: panel.focusX; onMoved: panel.focusX = value }
                Label { text: Math.round(panel.focusX * 100) + "%"; color: "#C5E7FF"; Layout.preferredWidth: 48 }
            }
            RowLayout {
                Label { text: "Vertical"; color: "#C5E7FF"; Layout.preferredWidth: 85 }
                Slider { Layout.fillWidth: true; enabled: !panel.working; from: 0; to: 1; value: panel.focusY; onMoved: panel.focusY = value }
                Label { text: Math.round(panel.focusY * 100) + "%"; color: "#C5E7FF"; Layout.preferredWidth: 48 }
            }
        }
        Label { Layout.fillWidth: true; text: panel.progressMessage; color: "#8BD5FF"; wrapMode: Text.WordWrap }
        RowLayout {
            Layout.fillWidth: true
            BusyIndicator { visible: panel.working; running: panel.working; Layout.preferredHeight: 28; Layout.preferredWidth: 28 }
            Button { text: "Apply layout"; Layout.fillWidth: true; enabled: !panel.working; onClicked: panel.applyLayout() }
            Button {
                text: "Desktop default"
                enabled: !panel.working
                onClicked: { panel.useDesktopDefault(panel.wallpaper, panel.monitorName); panel.close() }
            }
            Button { text: panel.working ? "Cancel conversion" : "Cancel"; onClicked: panel.close() }
        }
    }
}
