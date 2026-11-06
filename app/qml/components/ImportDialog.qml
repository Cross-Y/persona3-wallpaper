import QtQuick
import QtQuick.Dialogs

FileDialog {
    id: dialog

    property var controller

    signal importStatusChanged(string message)
    signal wallpaperImported()

    title: "Import wallpaper"
    fileMode: FileDialog.OpenFile

    nameFilters: [
        "Images (*.png *.jpg *.jpeg *.webp *.bmp *.gif)",
        "Videos (*.mp4 *.mkv *.webm *.mov *.avi)",
        "All files (*)"
    ]

    onAccepted: {
        try {
            const fileUrl = String(selectedFile)
            const filePath = decodeURIComponent(fileUrl.replace(/^file:\/\//, ""))
            const result = controller.import_file(filePath)

            importStatusChanged(String(result))

            if (String(result).startsWith("Imported successfully:")) {
                wallpaperImported()
            }
        } catch (error) {
            console.error("Import error:", error)
            importStatusChanged("Import error: " + error)
        }
    }

    onRejected: importStatusChanged("Import cancelled.")
}
