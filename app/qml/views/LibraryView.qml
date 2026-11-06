import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

ColumnLayout {
    id: view

    property bool active: false
    property var libraryItems: []
    property var monitorItems: []
    property string platformNotice: ""
    property string selectedMonitor: ""
    property string importStatus: ""

    signal statusRequested(string message)
    signal importRequested()
    signal uiHover()
    signal uiSelect()
    signal applyWallpaperRequested(var wallpaper, string monitorName)

    onMonitorItemsChanged: {
        if (selectedMonitor.length === 0 && monitorItems.length > 0) {
            const focused = monitorItems.find(item => item.focused)
            selectedMonitor = focused ? focused.name : monitorItems[0].name
        }
    }

    spacing: 16

    WallpaperHeader {
        Layout.fillWidth: true
        onImportRequested: view.importRequested()
        onUiHover: view.uiHover()
        onUiSelect: view.uiSelect()
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 12

        Label {
            text: view.monitorItems.length > 0 && view.monitorItems[0].name === "all" ? "Apply to:" : "Apply to monitor:"
            color: "#C0D8F3"
            font.family: "Inter"; font.pixelSize: 13
        }

        ComboBox {
            id: monitorSelector
            Layout.preferredWidth: 230
            model: view.monitorItems
            textRole: "description"
            enabled: view.monitorItems.length > 0
            currentIndex: {
                const index = view.monitorItems.findIndex(item => item.name === view.selectedMonitor)
                return index >= 0 ? index : 0
            }
            onHoveredChanged: if (hovered && enabled) view.uiHover()
            onActivated: index => { view.uiSelect(); view.selectedMonitor = view.monitorItems[index].name }
        }

        Label {
            visible: view.monitorItems.length === 0
            text: "No monitors detected"
            color: "#FFB4AB"
            font.family: "Inter"; font.pixelSize: 13
        }

        Item { Layout.fillWidth: true }
    }

    Label {
        Layout.fillWidth: true
        visible: view.platformNotice.length > 0
        text: view.platformNotice
        color: "#FFD29A"
        font.pixelSize: 13
        wrapMode: Text.WordWrap
    }

    Label {
        text: "Your animated and static backgrounds."
        color: "#C0D8F3"
        font.family: "Inter"; font.pixelSize: 14
    }

    Label {
        text: view.importStatus
        color: "#83CEFF"
        font.family: "Inter"; font.pixelSize: 13
        wrapMode: Text.Wrap
        Layout.fillWidth: true
        visible: text.length > 0
    }

    // Video library surface: frosted-glass layers and restrained Persona-blue accents.
    // The effects are confined to this panel; wallpaper thumbnails remain crisp.
    Rectangle {
        id: librarySurface
        Layout.fillWidth: true
        Layout.fillHeight: true
        radius: 20
        color: "#B9081428"
        border.color: "#668FC7"
        border.width: 1
        clip: true

        gradient: Gradient {
            GradientStop { position: 0.0; color: "#D9101D36" }
            GradientStop { position: 0.32; color: "#C9123154" }
            GradientStop { position: 0.72; color: "#C8173F69" }
            GradientStop { position: 1.0; color: "#D90A1B35" }
        }

        // Soft cyan/blue light blooms create a blurred-glass impression without
        // applying a costly blur pass to every thumbnail in the GridView.
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            width: parent.width * 0.62
            height: 110
            radius: 55
            opacity: 0.42
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.0; color: "#5547B9FF" }
                GradientStop { position: 0.55; color: "#222B8DDB" }
                GradientStop { position: 1.0; color: "#002B8DDB" }
            }
        }

        Rectangle {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            width: parent.width * 0.48
            height: 150
            radius: 75
            opacity: 0.28
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.0; color: "#001B75C8" }
                GradientStop { position: 1.0; color: "#554A9EFF" }
            }
        }

        // Fine inner outline and a brighter top edge give the panel depth.
        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            radius: 19
            color: "transparent"
            border.width: 1
            border.color: "#668DCB"
            opacity: 0.62
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.leftMargin: 28
            anchors.rightMargin: 28
            anchors.topMargin: 1
            height: 2
            radius: 1
            opacity: 0.9
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.0; color: "#004BCBFF" }
                GradientStop { position: 0.22; color: "#A64BCBFF" }
                GradientStop { position: 0.72; color: "#D68DEBFF" }
                GradientStop { position: 1.0; color: "#004BCBFF" }
            }
        }

        // Small corner marks: decorative outlines, deliberately subtle.
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.leftMargin: 14
            anchors.topMargin: 14
            width: 22; height: 2
            color: "#A8DFFF"
            opacity: 0.8
        }
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.leftMargin: 14
            anchors.topMargin: 14
            width: 2; height: 22
            color: "#A8DFFF"
            opacity: 0.8
        }
        Rectangle {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.rightMargin: 14
            anchors.bottomMargin: 14
            width: 22; height: 2
            color: "#579DFF"
            opacity: 0.75
        }
        Rectangle {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.rightMargin: 14
            anchors.bottomMargin: 14
            width: 2; height: 22
            color: "#579DFF"
            opacity: 0.75
        }

        GridView {
            anchors.fill: parent
            anchors.margins: 18
            clip: true
            cacheBuffer: 0
            reuseItems: true
            cellWidth: 225
            cellHeight: 235
            model: view.active ? view.libraryItems : []

            delegate: WallpaperCard {
                onUiHover: view.uiHover()
                onUiSelect: view.uiSelect()
                required property var modelData
                wallpaper: modelData
                onApplyRequested: wallpaper => {
                    if (view.selectedMonitor.length === 0) {
                        view.statusRequested("Select a monitor before applying a wallpaper.")
                    } else {
                        view.applyWallpaperRequested(wallpaper, view.selectedMonitor)
                    }
                }
            }
        }

        EmptyLibrary {
            visible: view.libraryItems.length === 0
        }
    }

    Label {
        text: view.libraryItems.length
            + (view.libraryItems.length === 1 ? " wallpaper" : " wallpapers")
        color: "#C2D8EE"
        font.family: "Inter"; font.pixelSize: 13
    }
}
