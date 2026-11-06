import QtQuick
import QtQuick.Controls

Column {
    anchors.centerIn: parent
    spacing: 10

    Label {
        anchors.horizontalCenter: parent.horizontalCenter
        text: "✦"
        color: "#83CEFF"
        font.family: "Inter"; font.pixelSize: 42
    }

    Label {
        anchors.horizontalCenter: parent.horizontalCenter
        text: "Your library is empty"
        color: "#F0F7FF"
        font.family: "Inter"; font.pixelSize: 20
    }

    Label {
        anchors.horizontalCenter: parent.horizontalCenter
        text: "Import a video or image to get started."
        color: "#C0D8F3"
        font.family: "Inter"; font.pixelSize: 13
    }
}
