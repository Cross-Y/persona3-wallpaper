import QtQuick

// Four replaceable Qt resources, one per page. Build-time assets may be
// PNG, JPG, JPEG, or WEBP; Qt detects their actual contents automatically.
// Preload the next page while the clock wipe covers the current page; the
// image swap happens only while the screen is completely black.
Item {
    id: background

    property string activePage: "Library"
    property string pendingPage: ""
    property bool frontActive: true
    property bool pendingOnFront: false
    property string frontSource: artForPage("Library")
    property string backSource: ""
    readonly property string fallbackSource: "qrc:/qt/qml/com/persona3/wallpaper/assets/persona3-background"
    readonly property bool pendingReady: {
        if (pendingPage === "") return true
        const imageStatus = pendingOnFront ? frontImage.status : backImage.status
        return imageStatus === Image.Ready || (imageStatus === Image.Error &&
            (pendingOnFront ? frontSource : backSource) === fallbackSource)
    }

    function artForPage(page) {
        const base = "qrc:/qt/qml/com/persona3/wallpaper/assets/"
        switch (page) {
        case "URL": return base + "persona3-url"
        case "Appearance": return base + "persona3-appearance"
        case "Settings": return base + "persona3-settings"
        default: return base + "persona3-background"
        }
    }

    function prepareScene(page) {
        if (page === activePage) return
        pendingPage = page
        pendingOnFront = !frontActive
        if (pendingOnFront) frontSource = artForPage(page)
        else backSource = artForPage(page)
    }

    function commitScene(page) {
        if (page === activePage) return
        if (pendingPage !== page) prepareScene(page)
        frontActive = pendingOnFront
        activePage = page
    }

    function finishScene() {
        pendingPage = ""
        // Keep only the current page decoded when the transition finishes.
        if (frontActive) backSource = ""
        else frontSource = ""
    }

    // The source binding clears decoded textures automatically when the user
    // hides the application in the tray. We keep only the file names in memory.
    Image {
        id: frontImage
        anchors.fill: parent
        visible: background.frontActive
        source: background.visible ? background.frontSource : ""
        sourceSize: Qt.size(1280, 800)
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: false
        onStatusChanged: {
            if (status === Image.Error && background.frontSource !== background.fallbackSource)
                background.frontSource = background.fallbackSource
        }
    }

    Image {
        id: backImage
        anchors.fill: parent
        visible: !background.frontActive
        source: background.visible ? background.backSource : ""
        sourceSize: Qt.size(1280, 800)
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: false
        onStatusChanged: {
            if (status === Image.Error && background.backSource !== background.fallbackSource)
                background.backSource = background.fallbackSource
        }
    }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#B5081429" }
            GradientStop { position: 0.45; color: "#85102649" }
            GradientStop { position: 1.0; color: "#A0203B68" }
        }
    }
}
