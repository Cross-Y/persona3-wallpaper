import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

Item {
    id: root
    property var controller
    signal savedFont(string family)
    signal savedCursor(string theme)
    signal savedThemeSetting(string key, bool enabled)
    signal uiHover()
    signal uiSelect()
    property string fontFamily: "Inter"
    property string pendingFontFamily: "Inter"
    property string cursorTheme: "Adwaita"
    property var fontFamilies: ["Inter"]
    property var cursorThemes: ["Adwaita"]
    property string previewText: "The quick brown fox jumps over the lazy dog. Hello, world!"
    property string statusMessage: ""
    property string environmentInfo: "Detecting desktop environment…"
    property bool dynamicEnabled: false
    property bool gtkColors: true
    property bool noctaliaColors: true
    property bool desktopColors: true
    property bool qtColors: false
    property bool waybarColors: false
    property bool rofiColors: false
    property bool wofiColors: false
    property string dynamicAccent: "#559DD7"
    property var dynamicPalette: ({primary: "#559DD7", secondary: "#8DAFD9", tertiary: "#8167B9", quaternary: "#C0A1B5"})
    property var automaticPalette: ({})
    property var paletteOverrides: ({})
    property int paletteRevision: 0
    readonly property var paletteRoles: [
        {key:"primary", label:"Primary", description:"Main accent color"},
        {key:"secondary", label:"Secondary", description:"Complementary accents"},
        {key:"tertiary", label:"Tertiary", description:"Contrast details"},
        {key:"quaternary", label:"Quaternary", description:"Borders and supporting details"}
    ]
    property string dynamicMessage: ""
    property bool noctaliaDetected: false
    property bool noctaliaWallpaperDisabled: false
    property bool ffmpegAvailable: false
    readonly property int cardMargin: width < 420 ? 12 : 18

    component ThemeChoice: Button {
        id: choice
        property bool active: false
        property string labelText: ""
        property bool showPowerState: false
        checkable: false
        onHoveredChanged: if (hovered && enabled) root.uiHover()
        onPressed: root.uiSelect()
        focusPolicy: Qt.StrongFocus
        Layout.fillWidth: true
        implicitHeight: showPowerState ? 46 : 38
        Accessible.role: Accessible.CheckBox
        Accessible.checked: choice.active
        Accessible.name: choice.labelText
        contentItem: RowLayout {
            spacing: 10
            Rectangle {
                width: 22; height: 22; radius: 6
                color: choice.active ? "#168FE0" : "#1B344F"
                border.color: choice.active ? "#8DDAFF" : "#7897B4"
                Label { anchors.centerIn: parent; text: "✓"; color: "white"; font.bold: true; visible: choice.active }
            }
            Label { Layout.fillWidth: true; text: choice.labelText; color: "#E4EFFA"; wrapMode: Text.WordWrap; font.pixelSize: 13 }
            Rectangle {
                visible: choice.showPowerState
                implicitWidth: 50; implicitHeight: 26; radius: 13
                color: choice.active ? "#176F59" : "#33465B"
                border.width: 1; border.color: choice.active ? "#7EEAC2" : "#8196AB"
                Label { anchors.centerIn: parent; text: choice.active ? "ON" : "OFF"; color: "#FFFFFF"; font.bold: true; font.pixelSize: 11 }
            }
        }
        background: Rectangle {
            radius: 10; color: choice.hovered ? "#233E60" : "transparent"
            border.width: choice.activeFocus ? 2 : 0
            border.color: "#8DDAFF"
        }
    }
    function isCustom(role) { return Object.prototype.hasOwnProperty.call(root.paletteOverrides, role) }
    function paletteColor(role) { return String(root.dynamicPalette[role] || "#559DD7") }
    function setPaletteColor(role, hex) {
        if (!root.controller) return
        try {
            const result = String(root.controller.set_palette_color(role, hex))
            root.dynamicMessage = result.replace(/^(OK:|ERROR:)\s*/, "")
        } catch (e) { root.dynamicMessage = String(e) }
        root.refreshDynamicStatus()
    }
    function togglePaletteManual(role) {
        root.setPaletteColor(role, root.isCustom(role) ? "" : root.paletteColor(role))
    }
    function openColorEditor(role) {
        if (!root.isCustom(role)) return
        const hex = root.paletteColor(role)
        colorEditor.role = role
        colorEditor.red = parseInt(hex.substring(1,3),16)
        colorEditor.green = parseInt(hex.substring(3,5),16)
        colorEditor.blue = parseInt(hex.substring(5,7),16)
        colorEditor.open()
    }

    function loadSettings(s) {
        dynamicEnabled = Boolean(s.theme_auto_enabled)
        gtkColors = s.theme_gtk_enabled !== false
        noctaliaColors = s.theme_noctalia_enabled !== false
        desktopColors = s.theme_desktop_enabled !== false
        qtColors = Boolean(s.theme_qt_enabled)
        waybarColors = Boolean(s.theme_waybar_enabled)
        rofiColors = Boolean(s.theme_rofi_enabled)
        wofiColors = Boolean(s.theme_wofi_enabled)
        try { refreshDynamicStatus() } catch (e) { dynamicMessage = String(e) }
        fontFamily = String(s.font_family || "Inter")
        pendingFontFamily = fontFamily
        cursorTheme = String(s.cursor_theme || "Adwaita")
        try { fontFamilies = JSON.parse(String(root.controller.list_fonts())) } catch (e) { fontFamilies = [fontFamily] }
        try { environmentInfo = String(root.controller.font_environment()) } catch (e) { environmentInfo = "Could not detect desktop environment." }
        if (fontFamilies.length > 0 && fontFamilies.indexOf(fontFamily) < 0) fontFamily = fontFamilies[0]
        try { cursorThemes = JSON.parse(String(root.controller.list_cursor_themes())) } catch (e) { cursorThemes = [cursorTheme] }
        if (cursorThemes.length > 0 && cursorThemes.indexOf(cursorTheme) < 0) cursorTheme = cursorThemes[0]
    }
    function refreshDynamicStatus() {
        if (!root.controller) return
        try {
            const info = JSON.parse(String(root.controller.dynamic_theme_status()))
            root.dynamicEnabled = Boolean(info.enabled)
            const keys = ["gtk", "noctalia", "desktop", "qt", "waybar", "rofi", "wofi"]
            const properties = ["gtkColors", "noctaliaColors", "desktopColors", "qtColors", "waybarColors", "rofiColors", "wofiColors"]
            for (let i = 0; i < keys.length; i++) {
                root[properties[i]] = Boolean(info[keys[i]])
                root.savedThemeSetting("theme_" + keys[i] + "_enabled", Boolean(info[keys[i]]))
            }
            root.savedThemeSetting("theme_auto_enabled", root.dynamicEnabled)
            root.dynamicAccent = String((info.palette && info.palette.primary) || info.accent || "#559DD7")
            root.dynamicPalette = info.palette || root.dynamicPalette
            root.automaticPalette = info.automatic_palette || root.automaticPalette
            root.paletteOverrides = info.overrides || ({})
            root.paletteRevision++
            root.noctaliaDetected = Boolean(info.noctalia_detected)
            root.noctaliaWallpaperDisabled = Boolean(info.noctalia_wallpaper_disabled)
            root.ffmpegAvailable = Boolean(info.ffmpeg)
        } catch (e) { root.dynamicMessage = "Could not read palette: " + String(e) }
    }
    function toggleDynamicColors(value) {
        try {
            const result = String(root.controller.set_dynamic_colors(value))
            root.dynamicMessage = result.replace(/^(OK:|ERROR:)\s*/, "")
        } catch(e) { root.dynamicMessage = String(e) }
        root.refreshDynamicStatus()
    }
    function toggleTarget(target, value) {
        try {
            const result = String(root.controller.set_dynamic_theme_target(target, value))
            root.dynamicMessage = result.replace(/^(OK:|ERROR:)\s*/, "")
        } catch(e) { root.dynamicMessage = String(e) }
        root.refreshDynamicStatus()
    }
    function regenerateColors() {
        try {
            const result = String(root.controller.refresh_dynamic_colors())
            root.dynamicMessage = result.replace(/^(OK:|ERROR:)\s*/, "")
        } catch(e) { root.dynamicMessage = String(e) }
        root.refreshDynamicStatus()
    }
    function applyFont() {
        if (!root.controller || !root.pendingFontFamily.length) return
        const result = String(root.controller.set_font_family(root.pendingFontFamily))
        if (result.startsWith("OK:")) {
            root.fontFamily = root.pendingFontFamily
            root.savedFont(root.fontFamily)
            root.statusMessage = "Applied to: " + result.substring(3) + ". Some applications may need to be restarted or you may need to log out and back in."
        } else {
            root.statusMessage = result.replace("ERROR:", "")
        }
    }
    function importCursorArchive(url) {
        if (!root.controller) return
        try {
            const result = String(root.controller.import_cursor_archive(url))
            if (!result.startsWith("OK:")) {
                root.statusMessage = result.replace(/^ERROR:\s*/, "")
                return
            }
            const imported = result.substring(3).split("|").filter(name => name.length > 0)
            root.cursorThemes = JSON.parse(String(root.controller.list_cursor_themes()))
            root.statusMessage = "Installed " + imported.join(", ") + ". Select a theme above to apply it."
        } catch (e) { root.statusMessage = "Could not import cursor theme: " + String(e) }
    }
    FileDialog {
        id: cursorImportDialog
        title: "Import cursor theme"
        fileMode: FileDialog.OpenFile
        nameFilters: [
            "Cursor archives (*.zip *.tar *.tar.gz *.tgz *.tar.xz *.txz *.tar.bz2 *.tbz *.tbz2)",
            "All files (*)"
        ]
        onAccepted: root.importCursorArchive(String(selectedFile))
    }
    function saveCursor(theme) {
        if (!root.controller) return
        const result = String(root.controller.set_cursor_theme(theme))
        if (result.startsWith("OK:")) { root.cursorTheme = theme; root.savedCursor(theme); root.statusMessage = "Cursor applied to: " + result.substring(3) }
        else { cursorMode.currentIndex = Math.max(0, root.cursorThemes.indexOf(root.cursorTheme)); root.statusMessage = result.replace("ERROR:", "") }
    }

    Dialog {
        id: colorEditor
        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true
        width: Math.min(440, root.width - 50)
        padding: 18
        title: "Edit color: " + colorEditor.role
        standardButtons: Dialog.Save | Dialog.Cancel
        property string role: "primary"
        property int red: 0
        property int green: 0
        property int blue: 0
        function channelHex(n) { return Math.round(n).toString(16).padStart(2, "0").toUpperCase() }
        function hexColor() { return "#" + channelHex(red) + channelHex(green) + channelHex(blue) }
        onAccepted: root.setPaletteColor(colorEditor.role, colorEditor.hexColor())
        background: Rectangle { radius: 16; color: "#152A46"; border.color: "#77B9E8" }
        contentItem: ColumnLayout {
            spacing: 12
            Rectangle { Layout.fillWidth: true; implicitHeight: 62; radius: 12; color: colorEditor.hexColor(); border.color: "#E7F4FF" }
            Label { text: colorEditor.hexColor(); color: "#F4FAFF"; font.pixelSize: 20; font.bold: true; Layout.alignment: Qt.AlignHCenter }
            Repeater {
                model: [{label:"R",key:"red"},{label:"G",key:"green"},{label:"B",key:"blue"}]
                delegate: RowLayout {
                    required property var modelData
                    Layout.fillWidth: true; spacing: 8
                    Label { text: modelData.label; color: "#CBE3F7"; font.bold: true; Layout.preferredWidth: 18 }
                    Slider { id: channelSlider; Layout.fillWidth: true; from: 0; to: 255; stepSize: 1
                        onMoved: colorEditor[modelData.key] = Math.round(value)
                        Binding { target: channelSlider; property: "value"; value: colorEditor[modelData.key] }
                    }
                    Label { text: colorEditor[modelData.key].toString(); color: "#F4FAFF"; Layout.preferredWidth: 30; horizontalAlignment: Text.AlignRight }
                }
            }
            Label { text: "Adjust the RGB channels, then click Save."; color: "#A9D2EF"; font.pixelSize: 12; wrapMode: Text.WordWrap; Layout.fillWidth: true }
        }
    }

    Flickable {
        anchors.fill: parent
        contentHeight: content.implicitHeight
        clip: true
        ColumnLayout {
            id: content
            width: parent.width
            spacing: 16
            Label { text: "Appearance"; color: "#F0F7FF"; font.pixelSize: 28; font.bold: true }
            Label { text: "Customize the look of your Linux desktop."; color: "#B8D0EA"; font.pixelSize: 14 }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: dynamicColumn.implicitHeight + root.cardMargin * 2
                radius: 18; color: "#A5102545"; border.color: root.dynamicEnabled ? root.dynamicAccent : "#668DBB"
                ColumnLayout {
                    id: dynamicColumn; anchors.fill: parent; anchors.margins: root.cardMargin; spacing: 10
                    RowLayout {
                        Layout.fillWidth: true
                        ColumnLayout { Layout.fillWidth: true; spacing: 3
                            Label { text: "DYNAMIC WALLPAPER COLORS"; color: "#73C5FF"; font.pixelSize: 13; font.bold: true; font.letterSpacing: 1.2 }
                            Label { text: "Match desktop accents to your current wallpaper"; color: "#D8E9F9"; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        }
                        Rectangle { width: 42; height: 42; radius: 12; color: root.dynamicAccent; border.color: "#DCE9FF"; border.width: 1 }
                    }
                    ThemeChoice {
                        labelText: "Desktop recoloring"
                        showPowerState: true
                        active: root.dynamicEnabled
                        onClicked: root.toggleDynamicColors(!root.dynamicEnabled)
                    }
                    Label {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        color: root.dynamicEnabled ? "#8EE3CB" : "#C6D4E3"
                        font.pixelSize: 12
                        text: root.dynamicEnabled
                            ? "ON — Wallpaper colors are applied to your selected desktop integrations."
                            : "OFF — Desktop recoloring is disabled. Your integration choices and custom palette are saved for later."
                    }
                    Label {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        color: "#9DBBD9"; font.pixelSize: 11
                        text: "Turning this off stops future color changes and attempts to restore the previous desktop appearance. Some applications may need to restart."
                    }
                    Label { text: "DESKTOP INTEGRATIONS"; color: "#B8D0EA"; font.pixelSize: 12; font.bold: true }
                    Label { Layout.fillWidth: true; visible: !root.dynamicEnabled; text: "Selections below are saved but inactive while desktop recoloring is OFF."; color: "#8FAFCB"; font.pixelSize: 11; wrapMode: Text.WordWrap }
                    ThemeChoice { labelText: "GTK 3 / GTK 4"; active: root.gtkColors; onClicked: root.toggleTarget("gtk", !root.gtkColors) }
                    ThemeChoice { labelText: "Noctalia (shell and templates)"; active: root.noctaliaColors; onClicked: root.toggleTarget("noctalia", !root.noctaliaColors) }
                    ThemeChoice { labelText: "GNOME / KDE Plasma accent"; active: root.desktopColors; onClicked: root.toggleTarget("desktop", !root.desktopColors) }
                    Label { text: "ADDITIONAL INTEGRATIONS"; color: "#73C5FF"; font.bold: true; font.pixelSize: 12 }
                    ThemeChoice { labelText: "Qt5 / Qt6 and Kvantum-compatible palette"; active: root.qtColors; onClicked: root.toggleTarget("qt", !root.qtColors) }
                    ThemeChoice { labelText: "Waybar"; active: root.waybarColors; onClicked: root.toggleTarget("waybar", !root.waybarColors) }
                    ThemeChoice { labelText: "Rofi"; active: root.rofiColors; onClicked: root.toggleTarget("rofi", !root.rofiColors) }
                    ThemeChoice { labelText: "Wofi"; active: root.wofiColors; onClicked: root.toggleTarget("wofi", !root.wofiColors) }
                    Rectangle { Layout.fillWidth: true; height: 1; color: "#426080" }
                    Label { Layout.fillWidth: true; text: "WALLPAPER COLOR PALETTE"; color: "#73C5FF"; font.bold: true; font.pixelSize: 13 }
                    Label { Layout.fillWidth: true; color: "#B8D0EA"; font.pixelSize: 12; wrapMode: Text.WordWrap
                        text: "Four color roles. Each can follow the wallpaper automatically or stay fixed to your chosen color. Saving updates enabled integrations." }
                    Repeater {
                        model: root.paletteRoles
                        delegate: Rectangle {
                            required property var modelData
                            readonly property string role: modelData.key
                            Layout.fillWidth: true
                            implicitHeight: paletteRow.implicitHeight + 20
                            radius: 12; color: "#122B47"; border.color: root.isCustom(role) ? "#65C5F5" : "#426080"
                            RowLayout {
                                id: paletteRow; anchors.left: parent.left; anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                anchors.leftMargin: 12; anchors.rightMargin: 12; spacing: 10
                                Rectangle { width: 42; height: 42; radius: 10; color: root.paletteColor(role); border.width: 1; border.color: "#DDEBFC" }
                                ColumnLayout { Layout.fillWidth: true; spacing: 1
                                    Label { text: modelData.label + " · " + root.paletteColor(role); color: "#F2F7FF"; font.bold: true; font.pixelSize: 13 }
                                    Label { text: root.isCustom(role) ? "Manual" : "Automatic"; color: root.isCustom(role) ? "#8EE3CB" : "#A8C4E1"; font.pixelSize: 12 }
                                    Label { text: modelData.description; color: "#9DBBD9"; font.pixelSize: 11; Layout.fillWidth: true; wrapMode: Text.WordWrap }
                                }
                                Button {
                                    onHoveredChanged: if (hovered && enabled) root.uiHover()
                                    onPressed: root.uiSelect()
                                    text: root.isCustom(role) ? "Auto" : "Customize"; onClicked: root.togglePaletteManual(role)
                                    ToolTip.visible: hovered; ToolTip.text: root.isCustom(role) ? "Use the wallpaper color again" : "Keep this color when the wallpaper changes" }
                                Button {
                                    onHoveredChanged: if (hovered && enabled) root.uiHover()
                                    onPressed: root.uiSelect()
                                    text: "Edit"; enabled: root.isCustom(role); onClicked: root.openColorEditor(role) }
                            }
                        }
                    }
                    Label { Layout.fillWidth: true; color: "#A9D5EC"; font.pixelSize: 11; wrapMode: Text.WordWrap
                        text: "Note: when recoloring is ON, all four colors are included in the exported palette. Noctalia v5 supports a custom palette; GNOME and Plasma use the native primary accent only. Some GTK/Qt themes may ignore certain color roles." }
                    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; font.pixelSize: 12; color: "#B8D0EA"
                        text: "Qt: select Persona 3 Wallpaper once in qt5ct/qt6ct or a KDE color scheme; Kvantum SVG themes may not follow this palette. Bars and launchers may require reopening after a palette update. Your existing config remains intact except for a marked import."
                    }
                    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; font.pixelSize: 12; color: "#B8D0EA"
                        text: "For video/GIF wallpapers a single frame is used; no permanent background scanner. GTK/libadwaita may require restarting applications or a compatible theme."
                    }
                    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; font.pixelSize: 12; color: "#FFCF97"
                        visible: root.noctaliaDetected && root.noctaliaColors && !root.noctaliaWallpaperDisabled
                        text: "Noctalia: first turn off Wallpaper → General → Enabled, so Noctalia does not cover Persona 3 Wallpaper's video. Then click Update now."
                    }
                    Label { Layout.fillWidth: true; font.pixelSize: 12; color: "#FFCF97"; visible: !root.ffmpegAvailable
                        text: "FFmpeg is required to sample wallpaper colors."
                    }
                    Button {
                        onHoveredChanged: if (hovered && enabled) root.uiHover()
                        onPressed: root.uiSelect()
                        text: "Update colors from current wallpaper"; enabled: root.dynamicEnabled; Layout.fillWidth: true
                        onClicked: root.regenerateColors()
                    }
                    Label { text: "Accent: " + root.dynamicAccent; color: root.dynamicAccent; font.bold: true; font.pixelSize: 13 }
                    Label { Layout.fillWidth: true; text: root.dynamicMessage; visible: root.dynamicMessage.length > 0
                        color: "#BDE5FF"; wrapMode: Text.WordWrap; font.pixelSize: 12 }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: fonts.implicitHeight + root.cardMargin * 2
                radius: 18; color: "#A5102545"; border.color: "#668DBB"
                ColumnLayout {
                    id: fonts; anchors.fill: parent; anchors.margins: root.cardMargin; spacing: 12
                    Label { text: "SYSTEM TYPOGRAPHY"; color: "#73C5FF"; font.pixelSize: 13; font.bold: true; font.letterSpacing: 2 }
                    Label { text: "Desktop font"; color: "#F0F7FF"; font.pixelSize: 15; font.bold: true }
                    ComboBox {
                        onHoveredChanged: if (hovered && enabled) root.uiHover()
                        id: fontMode; Layout.fillWidth: true; implicitHeight: 46
                        model: root.fontFamilies
                        currentIndex: Math.max(0, root.fontFamilies.indexOf(root.pendingFontFamily))
                        onActivated: index => root.pendingFontFamily = root.fontFamilies[index]
                        contentItem: Label { leftPadding: 14; rightPadding: 34; text: fontMode.displayText; color: "#F0F7FF"; font.family: root.pendingFontFamily; font.pixelSize: 14; verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight }
                        indicator: Label { text: "⌄"; color: "#8ED7FF"; font.pixelSize: 22; anchors.right: parent.right; anchors.rightMargin: 14; anchors.verticalCenter: parent.verticalCenter }
                        background: Rectangle { radius: 13; color: "#172F4D"; border.color: fontMode.hovered ? "#62C5FF" : "#52799E" }
                        popup: Popup { y: fontMode.height + 6; width: fontMode.width; padding: 6; implicitHeight: Math.min(contentItem.implicitHeight + padding * 2, 280); background: Rectangle { radius: 14; color: "#10233B"; border.color: "#52799E" }
                            contentItem: ListView { clip: true; implicitHeight: Math.min(contentHeight, 268); model: fontMode.popup.visible ? fontMode.delegateModel : null; currentIndex: fontMode.highlightedIndex; ScrollIndicator.vertical: ScrollIndicator {} }
                        }
                        delegate: ItemDelegate {
                            width: fontMode.width - 12; height: 38; highlighted: fontMode.highlightedIndex === index
                            contentItem: Label { text: modelData; color: "#F0F7FF"; font.family: modelData; font.pixelSize: 13; verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight }
                            background: Rectangle { radius: 8; color: highlighted ? "#24547A" : (parent.hovered ? "#1A3A5A" : "transparent") }
                        }
                    }
                    TextField {
                        id: fontSampleInput; Layout.fillWidth: true; text: root.previewText; placeholderText: "Write something to preview the font…"
                        onTextChanged: root.previewText = text
                        color: "#F0F7FF"; placeholderTextColor: "#819BB5"; font.family: root.pendingFontFamily; font.pixelSize: 14; padding: 12
                        background: Rectangle { radius: 11; color: "#10243B"; border.color: "#52799E" }
                    }
                    Rectangle {
                        Layout.fillWidth: true; implicitHeight: 86; radius: 12; color: "#0D1D31"; border.color: "#426080"
                        ColumnLayout { anchors.fill: parent; anchors.margins: 12; spacing: 5
                            Label { text: "FONT PREVIEW"; color: "#83CFFF"; font.family: root.pendingFontFamily; font.pixelSize: 10; font.letterSpacing: 1.5 }
                            Label { Layout.fillWidth: true; text: root.previewText.length ? root.previewText : "Your font preview will appear here"; color: "#F0F7FF"; font.family: root.pendingFontFamily; font.pixelSize: 19; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight }
                        }
                    }
                    Button {
                        onHoveredChanged: if (hovered && enabled) root.uiHover()
                        onPressed: root.uiSelect()
                        Layout.fillWidth: true
                        text: "Apply font"
                        enabled: root.pendingFontFamily.length > 0
                        onClicked: root.applyFont()
                    }
                    Label { Layout.fillWidth: true; text: root.environmentInfo; color: "#8ED7FF"; wrapMode: Text.WordWrap; font.pixelSize: 12 }
                    Label { Layout.fillWidth: true; text: "The selected font is previewed here and only applied after pressing the button. Persona 3 Wallpaper detects your desktop and updates compatible settings plus Fontconfig. Some applications use their own font settings and may need a restart."; color: "#B8D0EA"; wrapMode: Text.WordWrap; font.pixelSize: 12 }
                    Label { Layout.fillWidth: true; visible: root.statusMessage.length > 0; text: root.statusMessage; color: "#9DD5FF"; wrapMode: Text.WordWrap; font.pixelSize: 12 }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: cursor.implicitHeight + root.cardMargin * 2
                radius: 18; color: "#A5102545"; border.color: "#668DBB"
                ColumnLayout {
                    id: cursor; anchors.fill: parent; anchors.margins: root.cardMargin; spacing: 12
                    Label { text: "CURSOR"; color: "#73C5FF"; font.pixelSize: 13; font.bold: true; font.letterSpacing: 2 }
                    Label { text: "Cursor theme"; color: "#F0F7FF"; font.pixelSize: 15; font.bold: true }
                    RowLayout {
                        Layout.fillWidth: true; spacing: 12
                        ComboBox {
                            onHoveredChanged: if (hovered && enabled) root.uiHover()
                            id: cursorMode; Layout.fillWidth: true; Layout.preferredWidth: 1; implicitHeight: 46
                            model: root.cursorThemes; currentIndex: Math.max(0, root.cursorThemes.indexOf(root.cursorTheme))
                            onActivated: index => root.saveCursor(root.cursorThemes[index])
                            contentItem: Label { leftPadding: 14; rightPadding: 34; text: cursorMode.displayText; color: "#F0F7FF"; font.pixelSize: 14; verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight }
                            indicator: Label { text: "⌄"; color: "#8ED7FF"; font.pixelSize: 22; anchors.right: parent.right; anchors.rightMargin: 14; anchors.verticalCenter: parent.verticalCenter }
                            background: Rectangle { radius: 13; color: "#172F4D"; border.color: cursorMode.hovered ? "#62C5FF" : "#52799E" }
                            popup: Popup { y: cursorMode.height + 6; width: cursorMode.width; padding: 6; implicitHeight: Math.min(contentItem.implicitHeight + padding * 2, 240); background: Rectangle { radius: 14; color: "#10233B"; border.color: "#52799E" }
                                contentItem: ListView { clip: true; implicitHeight: Math.min(contentHeight, 228); model: cursorMode.popup.visible ? cursorMode.delegateModel : null; currentIndex: cursorMode.highlightedIndex; ScrollIndicator.vertical: ScrollIndicator {} }
                            }
                            delegate: ItemDelegate {
                                width: cursorMode.width - 12; height: 38; highlighted: cursorMode.highlightedIndex === index
                                contentItem: Label { text: modelData; color: "#F0F7FF"; font.pixelSize: 13; verticalAlignment: Text.AlignVCenter }
                                background: Rectangle { radius: 8; color: highlighted ? "#24547A" : (parent.hovered ? "#1A3A5A" : "transparent") }
                            }
                        }
                    }
                    Button {
                        onHoveredChanged: if (hovered && enabled) root.uiHover()
                        onPressed: root.uiSelect()
                        Layout.fillWidth: true
                        text: "Import cursor theme…"
                        onClicked: cursorImportDialog.open()
                    }
                    Label {
                        Layout.fillWidth: true
                        text: "Install Xcursor themes from ZIP, TAR, TAR.GZ, TAR.XZ or TAR.BZ2 archives. Themes are installed for your user only, without overwriting existing ones. Python 3 is required."
                        color: "#B8D0EA"; wrapMode: Text.WordWrap; font.pixelSize: 12
                    }
                    Label { Layout.fillWidth: true; text: "Cursor themes are discovered from installed icon themes. Persona 3 Wallpaper detects your desktop and applies the theme through the available compositor and desktop settings. A logout or application restart may be required for some desktops."; color: "#B8D0EA"; wrapMode: Text.WordWrap; font.pixelSize: 12 }
                }
            }
            Label { Layout.fillWidth: true; text: root.statusMessage; color: "#83CFFF"; wrapMode: Text.WordWrap; font.pixelSize: 13 }
        }
    }
}
