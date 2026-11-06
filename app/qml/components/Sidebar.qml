import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    property string currentPage: "Library"
    signal pageSelected(string page)
    signal uiHover()
    signal uiSelect()

    gradient: Gradient {
        GradientStop { position: 0.0; color: "#E609172D" }
        GradientStop { position: 0.55; color: "#D0112C4B" }
        GradientStop { position: 1.0; color: "#CC17416A" }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 22
        spacing: 18

        Label {
            text: "PERSONA 3"
            color: "#F0F7FF"
            font.pixelSize: 21
            font.bold: true
        }

        Label {
            text: "WALLPAPER"
            color: "#73C5FF"
            font.pixelSize: 13
            font.letterSpacing: 3
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: "#547CA8"
            opacity: 0.65
        }

        Repeater {
            model: [
                { label: "Library", available: true },
                { label: "URL", available: true },
                { label: "Appearance", available: true },
                { label: "Settings", available: true }
            ]

            delegate: Button {
                required property var modelData
                Layout.fillWidth: true
                text: modelData.label
                enabled: modelData.available
                flat: true
                highlighted: root.currentPage === modelData.label
                onHoveredChanged: if (hovered && enabled) root.uiHover()
                onClicked: {
                    // Switching tabs has its own cue; avoid also playing ui-select.
                    if (modelData.label === root.currentPage) root.uiSelect()
                    else root.pageSelected(modelData.label)
                }

                contentItem: Text {
                    text: parent.text
                    color: parent.enabled ? "#E5F1FF" : "#B3C5D9"
                    verticalAlignment: Text.AlignVCenter
                    font.pixelSize: 15
                }

                background: Rectangle {
                    radius: 10
                    color: root.currentPage === modelData.label ? "#446BA0C8" : "transparent"
                }
            }
        }

        Item { Layout.fillHeight: true }

        Label {
            text: "Linux Edition · Preview"
            color: "#C2D8EE"
            font.pixelSize: 13
        }
    }
}
