import QtQuick
import QtMultimedia

// Optional player, created only while the app is visible and interface audio is enabled.
// All clips are user-replaceable PCM WAV resources (mono or stereo).
Item {
    id: soundBank
    property real soundVolume: 0.55
    property bool queuedOpen: false
    property bool closeInProgress: false
    signal closeFinished()

    SoundEffect {
        id: hoverEffect
        source: "qrc:/qt/qml/com/persona3/wallpaper/audio/ui-hover.wav"
        volume: soundBank.soundVolume * 0.45
    }
    SoundEffect {
        id: selectEffect
        source: "qrc:/qt/qml/com/persona3/wallpaper/audio/ui-select.wav"
        volume: soundBank.soundVolume * 0.65
    }
    SoundEffect {
        id: openEffect
        source: "qrc:/qt/qml/com/persona3/wallpaper/audio/ui-open.wav"
        volume: soundBank.soundVolume * 0.65
        onStatusChanged: {
            if (status === SoundEffect.Ready && soundBank.queuedOpen) {
                soundBank.queuedOpen = false
                play()
            }
            if (status === SoundEffect.Error) soundBank.queuedOpen = false
        }
    }
    SoundEffect {
        id: closeEffect
        source: "qrc:/qt/qml/com/persona3/wallpaper/audio/ui-close.wav"
        volume: soundBank.soundVolume * 0.65
        onPlayingChanged: {
            if (!playing && soundBank.closeInProgress) {
                soundBank.closeInProgress = false
                soundBank.closeFinished()
            }
        }
    }
    SoundEffect {
        id: tabEffect
        source: "qrc:/qt/qml/com/persona3/wallpaper/audio/ui-tab.wav"
        volume: soundBank.soundVolume * 0.6
    }

    function playHover() {
        if (hoverEffect.status === SoundEffect.Ready) hoverEffect.play()
    }
    function playSelect() {
        if (selectEffect.status === SoundEffect.Ready) selectEffect.play()
    }
    function playOpen() {
        if (openEffect.status === SoundEffect.Ready) openEffect.play()
        else if (openEffect.status !== SoundEffect.Error) queuedOpen = true
    }
    function playClose() {
        if (closeEffect.status !== SoundEffect.Ready) return false
        closeInProgress = true
        closeEffect.play()
        return true
    }
    function playTab() {
        if (tabEffect.status === SoundEffect.Ready) tabEffect.play()
    }
}
