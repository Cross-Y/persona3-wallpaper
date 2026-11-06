import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: card

    required property var wallpaper
    signal applyRequested(var wallpaper)
    signal uiHover()
    signal uiSelect()

    width: 210
    height: 220
    radius: 12
    color: mouseArea.containsMouse ? "#E01B4168" : "#D9153150"
    border.color: mouseArea.containsMouse ? "#9BD7FF" : "#628FB8"
    border.width: mouseArea.containsMouse ? 2 : 1

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 8

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 8
            color: "#101F36"
            clip: true

            Image {
                id: preview
                anchors.fill: parent
                source: card.wallpaper.is_video
                    ? (card.wallpaper.thumbnail_path
                        ? "file://" + encodeURI(card.wallpaper.thumbnail_path)
                        : "")
                    : "file://" + encodeURI(card.wallpaper.path)
                visible: !card.wallpaper.is_video || status === Image.Ready
                fillMode: Image.PreserveAspectCrop
                sourceSize: Qt.size(360, 200)
                asynchronous: true
                cache: false
            }

            Column {
                anchors.centerIn: parent
                spacing: 8
                visible: card.wallpaper.is_video && preview.status !== Image.Ready

                Label {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "▶"
                    color: "#83CEFF"
                    font.family: "Inter"; font.pixelSize: 42
                }
                Label {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "VIDEO"
                    color: "#D9EDFF"
                    font.family: "Inter"; font.pixelSize: 13
                    font.bold: true
                }
            }

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 28
                color: "#BB081425"

                Label {
                    anchors.centerIn: parent
                    text: String(card.wallpaper.extension || "").toUpperCase()
                    color: "#E4F3FF"
                    font.family: "Inter"; font.pixelSize: 13
                    font.bold: true
                }
            }
        }

        Label {
            Layout.fillWidth: true
            text: card.wallpaper.name || "Untitled wallpaper"
            color: "#F0F7FF"
            font.family: "Inter"; font.pixelSize: 13
            elide: Text.ElideMiddle
            maximumLineCount: 1
        }

        Label {
            Layout.fillWidth: true
            text: card.wallpaper.is_video ? "Animated wallpaper" : "Static wallpaper"
            color: "#B9D8F2"
            font.family: "Inter"; font.pixelSize: 13
        }
    }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onEntered: card.uiHover()
        onClicked: { card.uiSelect(); card.applyRequested(card.wallpaper) }
    }
}
