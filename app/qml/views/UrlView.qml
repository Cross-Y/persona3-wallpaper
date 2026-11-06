import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: view
    property var controller
    property var session: null
    signal sessionSaved(var snapshot)
    property bool active: false
    property var results: []
    property int selectedIndex: -1
    property string statusText: "Loading featured wallpapers…"
    property string searchText: ""
    property bool loading: false
    property int currentPage: 1
    property int lastPage: 1
    property int totalResults: 0
    signal libraryRefreshRequested()
    signal uiHover()
    signal uiSelect()

    spacing: 12

    Label {
        Layout.fillWidth: true
        text: "URL"
        color: "#F0F7FF"
        font.family: "Inter"
        font.pixelSize: 27
        font.bold: true
    }

    Label {
        Layout.fillWidth: true
        text: "EXPLORE WALLPAPER SOURCES"
        color: "#80C9FF"
        font.family: "Inter"
        font.pixelSize: 13
        font.bold: true
        font.letterSpacing: 1.5
    }

    Flow {
        Layout.fillWidth: true
        spacing: 8

        Repeater {
            model: [
                { "name": "MoeWalls", "url": "https://moewalls.com/", "description": "Animated wallpapers" },
                { "name": "Wallsflow", "url": "https://wallsflow.com/", "description": "Live wallpaper catalog" },
                { "name": "NASA SVS", "url": "https://svs.gsfc.nasa.gov/", "description": "Public-domain space videos" }
            ]
            delegate: Button {
                onHoveredChanged: if (hovered && enabled) view.uiHover()
                onPressed: view.uiSelect()
                required property var modelData
                text: modelData.name + " ↗"
                onClicked: Qt.openUrlExternally(modelData.url)
                ToolTip.visible: hovered
                ToolTip.text: modelData.description + " · Opens in your browser"
            }
        }
    }

    Label {
        Layout.fillWidth: true
        text: "External sources open in your browser. Download a wallpaper there, then import the file into your library. The catalog below searches Wallhaven."
        color: "#B7CBE2"
        wrapMode: Text.Wrap
        font.family: "Inter"
        font.pixelSize: 13
    }

    Rectangle {
        Layout.fillWidth: true
        radius: 14
        color: "#C20B1B32"
        border.color: "#668DBB"
        border.width: 1
        implicitHeight: searchRow.implicitHeight + 22

        RowLayout {
            id: searchRow
            anchors.fill: parent
            anchors.margins: 11
            spacing: 9

            TextField {
                id: searchField
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                placeholderText: "Search wallpapers… (leave empty for latest)"
                selectByMouse: true
                onAccepted: { view.uiSelect(); searchButton.clicked() }
            }
            Button {
                onHoveredChanged: if (hovered && enabled) view.uiHover()
                onPressed: view.uiSelect()
                id: searchButton
                text: view.loading ? "Loading…" : "Search"
                enabled: !view.loading
                onClicked: view.loadCatalog(searchField.text)
            }
        }
    }

    Label {
        Layout.fillWidth: true
        text: view.statusText
        color: "#9BCFFF"
        wrapMode: Text.Wrap
        font.family: "Inter"
        font.pixelSize: 13
    }

    // The catalog's small JSON metadata survives navigation; thumbnail Image
    // objects and GPU textures do not. A one-shot Timer avoids accumulating
    // Qt.callLater closures and the previous duplicate startup requests.
    Timer {
        id: initialCatalogTimer
        interval: 1
        repeat: false
        onTriggered: view.loadCatalog(view.searchText, view.currentPage)
    }

    function restoreSession() {
        if (session && session.loaded) {
            view.results = session.results || []
            view.currentPage = session.currentPage || 1
            view.lastPage = session.lastPage || 1
            view.totalResults = session.totalResults || 0
            view.searchText = session.searchText || ""
            view.statusText = session.statusText || ""
            view.selectedIndex = typeof session.selectedIndex === "number" ? session.selectedIndex : -1
            searchField.text = view.searchText
        } else if (active) {
            initialCatalogTimer.start()
        }
    }

    Component.onCompleted: restoreSession()
    Component.onDestruction: {
        initialCatalogTimer.stop()
        view.sessionSaved({
            loaded: view.results.length > 0,
            results: view.results,
            currentPage: view.currentPage,
            lastPage: view.lastPage,
            totalResults: view.totalResults,
            searchText: view.searchText,
            statusText: view.statusText,
            selectedIndex: view.selectedIndex
        })
    }

    function loadCatalog(query, requestedPage) {
        if (!view.active || !view.controller || view.loading) return
        const targetPage = requestedPage === undefined ? 1 : Math.max(1, requestedPage)
        view.loading = true
        view.searchText = query.trim()
        view.statusText = view.searchText.length
            ? "Searching Wallhaven…"
            : "Loading featured wallpapers from Wallhaven…"
        view.selectedIndex = -1
        try {
            const raw = String(view.controller.search_catalog("wallhaven", view.searchText, targetPage))
            if (raw.startsWith("ERROR:")) {
                view.results = []
                view.lastPage = 1
                view.totalResults = 0
                view.statusText = raw.substring(6)
            } else {
                const payload = JSON.parse(raw)
                // Keep one page modest; this is a preview browser, not an image cache.
                view.results = (payload.items || []).slice(0, 18)
                view.currentPage = Number(payload.page || targetPage)
                view.lastPage = Number(payload.last_page || 1)
                view.totalResults = Number(payload.total || view.results.length)
                view.statusText = view.totalResults + " wallpapers · Page " + view.currentPage + " of " + view.lastPage + ". Select a wallpaper to see details."
            }
        } catch (error) {
            view.results = []
            view.lastPage = 1
            view.totalResults = 0
            view.statusText = "Could not load Wallhaven: " + error
        }
        view.loading = false
    }

    RowLayout {
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 220
        spacing: 12

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: 0
            radius: 16
            color: "#9908172B"
            border.color: "#385D8C"
            clip: true

            GridView {
                id: grid
                anchors.fill: parent
                anchors.margins: 12
                clip: true
                cellWidth: Math.max(150, Math.floor(width / Math.max(1, Math.floor(width / 205))))
                cellHeight: 151
                cacheBuffer: 0
                reuseItems: true
                model: view.active ? view.results : []
                delegate: Rectangle {
                    required property int index
                    required property var modelData
                    width: grid.cellWidth - 9
                    height: 140
                    radius: 11
                    clip: true
                    color: view.selectedIndex === index ? "#345B91" : "#B5122945"
                    border.width: view.selectedIndex === index ? 2 : 1
                    border.color: view.selectedIndex === index ? "#76C8FF" : "#3F6D9E"

                    Image {
                        anchors.top: parent.top
                        anchors.left: parent.left
                        anchors.right: parent.right
                        height: 101
                        source: view.active ? modelData.preview : ""
                        fillMode: Image.PreserveAspectCrop
                        sourceSize: Qt.size(320, 180)
                        asynchronous: true
                        cache: false
                    }
                    Label {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.margins: 8
                        text: modelData.title
                        color: "#EDF6FF"
                        elide: Text.ElideRight
                        font.family: "Inter"
                        font.pixelSize: 13
                    }
                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        onEntered: view.uiHover()
                        onClicked: { view.uiSelect(); view.selectedIndex = index }
                    }
                }
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }
            }

            Label {
                anchors.centerIn: parent
                width: parent.width - 32
                horizontalAlignment: Text.AlignHCenter
                visible: view.results.length === 0 && !view.loading
                text: "No wallpapers found. Try another search."
                color: "#B7CBE2"
                wrapMode: Text.Wrap
                font.family: "Inter"
                font.pixelSize: 13
            }
        }

        Rectangle {
            Layout.preferredWidth: view.width < 760 ? 205 : 238
            Layout.minimumWidth: 190
            Layout.fillHeight: true
            radius: 16
            color: "#CC0B1B32"
            border.color: "#557DB4"
            clip: true

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 9

                Label {
                    Layout.fillWidth: true
                    text: "WALLPAPER DETAILS"
                    color: "#9BCFFF"
                    font.family: "Inter"
                    font.pixelSize: 13
                    font.bold: true
                }
                Image {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 122
                    source: view.active && view.selectedIndex >= 0 && view.selectedIndex < view.results.length
                        ? view.results[view.selectedIndex].preview : ""
                    fillMode: Image.PreserveAspectCrop
                    sourceSize: Qt.size(480, 270)
                    asynchronous: true
                    cache: false
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: "#F0F7FF"
                    font.family: "Inter"
                    font.bold: true
                    text: view.selectedIndex >= 0 && view.selectedIndex < view.results.length
                        ? view.results[view.selectedIndex].title : "Select a wallpaper"
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    color: "#C8D9EB"
                    font.family: "Inter"
                    font.pixelSize: 13
                    text: view.selectedIndex >= 0 && view.selectedIndex < view.results.length
                        ? "Wallhaven · " + view.results[view.selectedIndex].resolution
                        : "Preview and resolution will appear here."
                }
                Label {
                    Layout.fillWidth: true
                    text: "DOWNLOAD RESOLUTION"
                    color: "#80C9FF"
                    font.family: "Inter"
                    font.pixelSize: 13
                    font.bold: true
                    visible: view.selectedIndex >= 0 && view.selectedIndex < view.results.length
                }
                ComboBox {
                    onHoveredChanged: if (hovered && enabled) view.uiHover()
                    onActivated: view.uiSelect()
                    id: resolutionSelector
                    Layout.fillWidth: true
                    visible: view.selectedIndex >= 0 && view.selectedIndex < view.results.length
                    model: view.selectedIndex >= 0 && view.selectedIndex < view.results.length
                        ? (view.results[view.selectedIndex].download_options || [{label: "Original", url: view.results[view.selectedIndex].download, filename: view.results[view.selectedIndex].filename}])
                        : []
                    textRole: "label"
                    enabled: count > 0 && !view.loading
                }
                Label {
                    Layout.fillWidth: true
                    visible: view.selectedIndex >= 0 && view.selectedIndex < view.results.length
                    text: "Wallhaven provides the original uploaded resolution for each wallpaper; alternate downloadable sizes are not available for every item."
                    color: "#B7CBE2"
                    wrapMode: Text.Wrap
                    font.family: "Inter"
                    font.pixelSize: 13
                }
                Item { Layout.fillHeight: true }
                Button {
                    onHoveredChanged: if (hovered && enabled) view.uiHover()
                    onPressed: view.uiSelect()
                    Layout.fillWidth: true
                    enabled: view.selectedIndex >= 0 && view.selectedIndex < view.results.length && !view.loading
                    text: "Install"
                    onClicked: {
                        const item = view.results[view.selectedIndex]
                        const options = item.download_options || [{url: item.download, filename: item.filename, label: "Original"}]
                        const option = options[Math.max(0, resolutionSelector.currentIndex)]
                        view.statusText = "Downloading " + item.title + " · " + option.label + "…"
                        const result = String(view.controller.install_remote(option.url, option.filename))
                        if (result.startsWith("OK:")) {
                            view.statusText = "Installed successfully in your library."
                            view.libraryRefreshRequested()
                        } else {
                            view.statusText = result.replace("ERROR:", "")
                        }
                    }
                }
                Label {
                    Layout.fillWidth: true
                    text: "Images are provided by Wallhaven. Check each wallpaper's usage rights."
                    color: "#B7CBE2"
                    wrapMode: Text.Wrap
                    font.family: "Inter"
                    font.pixelSize: 13
                }
            }
        }
    }

    RowLayout {
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignHCenter
        spacing: 12
        Button {
            onHoveredChanged: if (hovered && enabled) view.uiHover()
            onPressed: view.uiSelect()
            text: "‹ Previous"
            enabled: !view.loading && view.currentPage > 1
            onClicked: view.loadCatalog(view.searchText, view.currentPage - 1)
        }
        Label {
            text: "Page " + view.currentPage + " / " + view.lastPage
            color: "#C8D9EB"
            font.family: "Inter"
            font.pixelSize: 13
        }
        Button {
            onHoveredChanged: if (hovered && enabled) view.uiHover()
            onPressed: view.uiSelect()
            text: "Next ›"
            enabled: !view.loading && view.currentPage < view.lastPage
            onClicked: view.loadCatalog(view.searchText, view.currentPage + 1)
        }
    }
}
