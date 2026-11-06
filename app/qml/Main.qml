import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Qt.labs.platform as Platform
import com.persona3.wallpaper

ApplicationWindow {
    id: window

    visible: true
    width: 1180
    height: 760
    minimumWidth: 900
    minimumHeight: 600

    title: "Persona 3 Wallpaper"
    color: "#081425"

    property var libraryItems: []
    property var monitorItems: []
    property string currentPage: "Library"
    property string transitionTarget: ""
    property string queuedPage: ""
    property bool quitRequested: false
    property string libraryStatus: ""
    property string platformNotice: ""
    property var wallpaperCapabilities: ({})
    // Tiny, persistent state only. The QML pages and their image delegates are disposable.
    property var appearanceSettings: ({})
    property var catalogSession: null
    property bool settingsInitialized: false
    property double lastUiHoverMs: 0
    property bool pendingOpenSound: false
    property bool pendingCloseSound: false
    property bool quitAfterCloseSound: false

    QtObject {
        id: runtimeSettings
        property bool autoVideoOptimize: true
        property bool engineActive: true
        property bool restoreOnStart: true
        property bool focusPause: true
        property bool startOnLogin: false
        property bool backgroundMode: true
        property bool soundEnabled: false
        property bool audioDesktopOnly: false
        property bool uiSoundsEnabled: true
        property real uiSoundVolume: 0.55
        property string statusMessage: ""
    }

    function loadRuntimeSettings(s) {
        runtimeSettings.autoVideoOptimize = s.auto_video_optimize !== false
        runtimeSettings.engineActive = !Boolean(s.paused)
        runtimeSettings.restoreOnStart = s.restore_on_start !== false
        runtimeSettings.focusPause = Boolean(s.focus_pause)
        runtimeSettings.startOnLogin = Boolean(s.start_on_login)
        runtimeSettings.backgroundMode = s.background_mode !== false
        runtimeSettings.soundEnabled = Boolean(s.sound_enabled)
        runtimeSettings.audioDesktopOnly = Boolean(s.audio_desktop_only)
        runtimeSettings.uiSoundsEnabled = s.ui_sounds_enabled !== false
        const volume = Number(s.ui_sound_volume)
        runtimeSettings.uiSoundVolume = isFinite(volume) ? Math.max(0,Math.min(1,volume)) : 0.55
        appearanceSettings = s
        settingsInitialized = true
    }

    function saveAppearanceSetting(key, value) {
        const updated = Object.assign({}, appearanceSettings)
        updated[key] = value
        appearanceSettings = updated
    }

    function playUiHover() {
        if (!window.visible || !runtimeSettings.uiSoundsEnabled || clockTransition.playing) return
        const now = Date.now()
        if (now - lastUiHoverMs < 85) return
        lastUiHoverMs = now
        if (menuSoundLoader.item) menuSoundLoader.item.playHover()
    }

    function playUiSelect() {
        if (!window.visible || !runtimeSettings.uiSoundsEnabled) return
        if (menuSoundLoader.item) menuSoundLoader.item.playSelect()
    }

    function requestOpenSound() {
        if (!window.visible || !window.settingsInitialized || !runtimeSettings.uiSoundsEnabled) return
        pendingOpenSound = true
        flushOpenSound()
    }

    function flushOpenSound() {
        if (!pendingOpenSound || !menuSoundLoader.item || !window.visible) return
        pendingOpenSound = false
        menuSoundLoader.item.playOpen()
    }

    function playTabSound() {
        if (window.visible && runtimeSettings.uiSoundsEnabled && menuSoundLoader.item)
            menuSoundLoader.item.playTab()
    }

    function finishCloseWindow() {
        if (!pendingCloseSound) return
        pendingCloseSound = false
        closeSoundFallback.stop()
        if (quitAfterCloseSound || !runtimeSettings.backgroundMode || !trayIcon.available) {
            quitRequested = true
            Qt.quit()
        } else {
            window.hide()
        }
    }

    function beginCloseWithSound(forceQuit) {
        if (pendingCloseSound) return true
        if (!window.visible || !runtimeSettings.uiSoundsEnabled || !menuSoundLoader.item ||
                !menuSoundLoader.item.playClose()) return false
        quitAfterCloseSound = forceQuit
        pendingCloseSound = true
        closeSoundFallback.restart()
        return true
    }

    function quitApp() {
        if (quitRequested || pendingCloseSound) return
        if (!beginCloseWithSound(true)) {
            quitRequested = true
            Qt.quit()
        }
    }

    function navigateTo(page) {
        if (clockTransition.playing) {
            queuedPage = page
            return
        }
        if (page === currentPage) return
        transitionTarget = page
        appBackground.prepareScene(page)
        playTabSound()
        clockTransition.play()
    }

    function showWindow() {
        window.show()
        window.raise()
        window.requestActivate()
    }

    function setManualPause(paused) {
        const result = String(settingsController.set_paused(paused))
        if (result === "OK") {
            runtimeSettings.engineActive = !paused
            runtimeSettings.statusMessage = paused ? "All wallpapers paused." : "Wallpapers resumed."
        } else {
            runtimeSettings.statusMessage = result.replace("ERROR:", "")
        }
    }

    function refreshLibrary() {
        try {
            const data = wallpaperController.list_wallpapers()
            libraryItems = JSON.parse(data)
            console.log("Library updated:", libraryItems.length, "items")
        } catch (error) {
            console.error("Library refresh failed:", error)
            window.libraryStatus = "Could not refresh library."
        }
    }

    function refreshMonitors() {
        try {
            const data = monitorController.list_monitors()
            monitorItems = JSON.parse(data)

            console.log("Monitors detected:", monitorItems.length)
            for (const monitor of monitorItems) {
                console.log(
                    monitor.name,
                    monitor.width + "x" + monitor.height,
                    monitor.focused ? "(Focused)" : ""
                )
            }
        } catch (error) {
            console.error("Monitor detection failed:", error)
        }
    }

    function showApplyResult(result) {
        window.libraryStatus = String(result)
        console.log(window.libraryStatus)
    }

    function applyWallpaperDirect(wallpaper, monitorName) {
        try {
            showApplyResult(wallpaperController.apply_wallpaper(
                String(wallpaper.path), String(monitorName), Boolean(wallpaper.is_video)))
        } catch (error) {
            window.libraryStatus = "Could not apply wallpaper: " + error
        }
    }

    function applyVideoChoice(wallpaper, monitorName, optimized) {
        try {
            showApplyResult(wallpaperController.apply_video_choice(String(wallpaper.path), String(monitorName), optimized))
        } catch (error) {
            window.libraryStatus = "Could not apply video: " + error
        }
    }

    function applyImageLayout(wallpaper, monitorName, mode, fx, fy, zoom) {
        try {
            showApplyResult(wallpaperController.apply_image_layout(
                String(wallpaper.path), String(monitorName), mode, fx, fy, zoom))
        } catch (error) {
            window.libraryStatus = "Could not apply image layout: " + error
        }
    }

    function applyWallpaper(wallpaper, monitorName) {
        // The current wallpaper remains visible while an oversized video is
        // prepared; images use their own framing popup and non-blocking worker.
        try {
            const info = JSON.parse(String(wallpaperController.wallpaper_media_info(
                String(wallpaper.path), String(monitorName), Boolean(wallpaper.is_video))))
            if (info.ok && Boolean(wallpaper.is_video) && info.oversized) {
                videoPreparationDialog.openFor(wallpaper, monitorName, info)
                return
            }
            if (info.ok && !Boolean(wallpaper.is_video) && info.monitor_known) {
                imageFramingDialog.openFor(wallpaper, monitorName, info)
                return
            }
            if (!info.ok)
                console.warn("Media inspection unavailable:", info.error)
        } catch (error) {
            console.warn("Media preflight failed:", error)
        }
        applyWallpaperDirect(wallpaper, monitorName)
    }

    VideoPreparationDialog {
        id: videoPreparationDialog
        controller: wallpaperController
        onUseOptimized: (wallpaper, monitorName) => window.applyVideoChoice(wallpaper, monitorName, true)
        onUseOriginal: (wallpaper, monitorName) => window.applyVideoChoice(wallpaper, monitorName, false)
    }

    ImageFramingDialog {
        id: imageFramingDialog
        controller: wallpaperController
        onImageReady: (wallpaper, monitorName, mode, x, y, zoom) =>
            window.applyImageLayout(wallpaper, monitorName, mode, x, y, zoom)
        onUseDesktopDefault: (wallpaper, monitorName) => window.applyWallpaperDirect(wallpaper, monitorName)
    }

    WallpaperController {
        id: wallpaperController
    }

    MonitorController {
        id: monitorController
    }

    SettingsController {
        id: settingsController
    }

    Component.onCompleted: {
        try {
            wallpaperCapabilities = JSON.parse(String(settingsController.wallpaper_capabilities()))
            platformNotice = String(settingsController.wallpaper_support())
        }
        catch (error) { platformNotice = "Could not check desktop compatibility." }
        refreshLibrary()
        refreshMonitors()

        try {
            const settings = JSON.parse(settingsController.initialize())
            loadRuntimeSettings(settings)
        } catch (error) {
            console.error("Settings initialization failed:", error)
        }
        // Wait for persisted preferences before triggering the first window cue.
        requestOpenSound()
    }

    // QML multimedia is optional. If QtMultimedia is not installed, Loader fails
    // without preventing the UI or the wallpaper engine from running.
    Loader {
        id: menuSoundLoader
        active: window.visible && runtimeSettings.uiSoundsEnabled
        source: "qrc:/qt/qml/com/persona3/wallpaper/audio/MenuSoundEffects.qml"
        onLoaded: {
            item.soundVolume = runtimeSettings.uiSoundVolume
            window.flushOpenSound()
        }
    }
    Connections {
        target: runtimeSettings
        function onUiSoundVolumeChanged() {
            if (menuSoundLoader.item) menuSoundLoader.item.soundVolume = runtimeSettings.uiSoundVolume
        }
        function onUiSoundsEnabledChanged() {
            if (!runtimeSettings.uiSoundsEnabled) window.pendingOpenSound = false
        }
    }
    Connections {
        target: menuSoundLoader.item
        ignoreUnknownSignals: true
        function onCloseFinished() { window.finishCloseWindow() }
    }
    Timer {
        id: closeSoundFallback
        interval: 1800
        repeat: false
        onTriggered: window.finishCloseWindow()
    }

    // Unload the large decorative image too when running only in the system tray.
    AppBackground {
        id: appBackground
        anchors.fill: parent
        visible: window.visible
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Sidebar {
            Layout.preferredWidth: 220
            Layout.fillHeight: true
            currentPage: window.currentPage
            onPageSelected: page => window.navigateTo(page)
            onUiHover: window.playUiHover()
            onUiSelect: window.playUiSelect()
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 30
            currentIndex: window.currentPage === "Settings" ? 3 : (window.currentPage === "Appearance" ? 2 : (window.currentPage === "URL" ? 1 : 0))

            // Loader.active=false DESTROYS the old view, including GridView delegates,
            // decoded images, popups, transient timers and signal connections.
            Loader {
                id: libraryLoader
                Layout.fillWidth: true
                Layout.fillHeight: true
                active: window.visible && window.currentPage === "Library"
                sourceComponent: LibraryView {
                    onUiHover: window.playUiHover()
                    onUiSelect: window.playUiSelect()
                    active: true
                    libraryItems: window.libraryItems
                    monitorItems: window.monitorItems
                    platformNotice: window.platformNotice
                    importStatus: window.libraryStatus
                    onImportRequested: importDialog.open()
                    onStatusRequested: message => window.libraryStatus = message
                    onApplyWallpaperRequested: (wallpaper, monitorName) => window.applyWallpaper(wallpaper, monitorName)
                }
            }

            Loader {
                id: urlLoader
                Layout.fillWidth: true
                Layout.fillHeight: true
                active: window.visible && window.currentPage === "URL"
                sourceComponent: UrlView {
                    onUiHover: window.playUiHover()
                    onUiSelect: window.playUiSelect()
                    active: true
                    controller: wallpaperController
                    session: window.catalogSession
                    onSessionSaved: snapshot => window.catalogSession = snapshot
                    onLibraryRefreshRequested: window.refreshLibrary()
                }
            }

            Loader {
                id: appearanceLoader
                Layout.fillWidth: true
                Layout.fillHeight: true
                active: window.visible && window.currentPage === "Appearance"
                sourceComponent: AppearanceView {
                    onUiHover: window.playUiHover()
                    onUiSelect: window.playUiSelect()
                    controller: settingsController
                    onSavedFont: family => window.saveAppearanceSetting("font_family", family)
                    onSavedCursor: theme => window.saveAppearanceSetting("cursor_theme", theme)
                    onSavedThemeSetting: (key, enabled) => window.saveAppearanceSetting(key, enabled)
                }
                onLoaded: {
                    if (window.settingsInitialized)
                        item.loadSettings(window.appearanceSettings)
                }
            }

            Loader {
                id: settingsLoader
                Layout.fillWidth: true
                Layout.fillHeight: true
                active: window.visible && window.currentPage === "Settings"
                sourceComponent: SettingsView {
                    onUiHover: window.playUiHover()
                    onUiSelect: window.playUiSelect()
                    controller: settingsController
                    appState: runtimeSettings
                    playbackControlsAvailable: Boolean(window.wallpaperCapabilities.playback_controls)
                    focusControlsAvailable: window.wallpaperCapabilities.backend === "hyprland"
                }
            }
        }
    }

    // Clockwise black wipe with a tapered blue edge, covering the entire UI.
    ClockTransition {
        id: clockTransition
        anchors.fill: parent
        windowVisible: window.visible
        readyForReveal: appBackground.pendingReady
        z: 100
        onMidpoint: {
            appBackground.commitScene(window.transitionTarget)
            window.currentPage = window.transitionTarget
        }
        onFinished: {
            appBackground.finishScene()
            if (window.queuedPage !== "" && window.queuedPage !== window.currentPage) {
                const destination = window.queuedPage
                window.queuedPage = ""
                window.navigateTo(destination)
            } else {
                window.queuedPage = ""
            }
        }
    }

    Platform.SystemTrayIcon {
        id: trayIcon
        visible: runtimeSettings.backgroundMode
        tooltip: "Persona 3 Wallpaper\nRAM: calculando…\nCPU: calculando…"
        icon.source: "qrc:/qt/qml/com/persona3/wallpaper/assets/persona3-evoker.png"

        onActivated: function(reason) {
            if (reason === Platform.SystemTrayIcon.Trigger ||
                reason === Platform.SystemTrayIcon.DoubleClick) {
                window.showWindow()
            }
        }

        menu: Platform.Menu {
            Platform.MenuItem {
                text: "Open Persona 3 Wallpaper"
                onTriggered: window.showWindow()
            }

            Platform.MenuSeparator {}

            Platform.MenuItem {
                text: runtimeSettings.engineActive ? "Pause animations" : "Resume animations"
                enabled: Boolean(window.wallpaperCapabilities.playback_controls)
                onTriggered: window.setManualPause(!runtimeSettings.engineActive)
            }

            Platform.MenuItem {
                text: "Stop wallpapers"
                enabled: Boolean(window.wallpaperCapabilities.playback_controls)
                onTriggered: {
                    const result = String(settingsController.stop_wallpapers())
                    runtimeSettings.statusMessage = result === "OK"
                        ? "Wallpaper processes stopped."
                        : result.replace("ERROR:", "")
                }
            }

            Platform.MenuItem {
                text: "Restore wallpapers"
                onTriggered: {
                    const result = String(settingsController.restore_wallpapers())
                    runtimeSettings.statusMessage = result === "OK"
                        ? "Saved wallpapers restored."
                        : result.replace("ERROR:", "")
                }
            }

            Platform.MenuSeparator {}

            Platform.MenuItem {
                text: "Quit"
                onTriggered: window.quitApp()
            }
        }
    }

    // Qt.labs.platform.SystemTrayIcon has no consistent hover event across
    // desktops. Keep the tooltip updated only while the window is hidden
    // and the tray is active, so hovering shows recent measurements.
    Timer {
        id: trayResourceTimer
        interval: 2500
        repeat: true
        running: trayIcon.visible && trayIcon.available && !window.visible
        onTriggered: {
            try {
                trayIcon.tooltip = String(settingsController.resource_usage())
            } catch (error) {
                trayIcon.tooltip = "Persona 3 Wallpaper\nCould not read resource usage"
                console.warn("Resource monitor failed:", error)
            }
        }
    }

    onVisibleChanged: {
        if (visible) requestOpenSound()
        else {
            pendingOpenSound = false
            if (trayIcon.visible && trayIcon.available)
                trayIcon.tooltip = String(settingsController.resource_usage())
        }
    }

    onClosing: function(close) {
        if (quitRequested) return
        // Let the closing cue finish while the QtMultimedia Loader is still alive.
        if (pendingCloseSound) {
            close.accepted = false
            return
        }
        if (beginCloseWithSound(false)) {
            close.accepted = false
            return
        }
        // Original behavior when menu sounds are muted or QtMultimedia is absent.
        if (runtimeSettings.backgroundMode && trayIcon.available) {
            close.accepted = false
            window.hide()
        }
    }

    ImportDialog {
        id: importDialog
        controller: wallpaperController

        onImportStatusChanged: message => window.libraryStatus = message
        onWallpaperImported: refreshLibrary()
    }
}
