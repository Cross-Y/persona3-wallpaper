import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

RowLayout {
    signal importRequested()
    signal uiHover()
    signal uiSelect()

    Label {
        text: "Wallpaper Library"
        color: "#F0F7FF"
        font.family: "Inter"; font.pixelSize: 27
        font.bold: true
    }

    Item { Layout.fillWidth: true }

    Button {
        text: "＋  Import wallpaper"
        onHoveredChanged: if (hovered && enabled) uiHover()
        onClicked: { uiSelect(); importRequested() }

        contentItem: Text {
            text: parent.text
            color: "#F4FAFF"
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            font.family: "Inter"; font.pixelSize: 14
        }

        background: Rectangle {
            radius: 6
            border.color: "#75C8FF"

            gradient: Gradient {
                GradientStop { position: 0.0; color: "#347DB8" }
                GradientStop { position: 0.5; color: "#245B96" }
                GradientStop { position: 1.0; color: "#183D70" }
            }
        }
    }
}
