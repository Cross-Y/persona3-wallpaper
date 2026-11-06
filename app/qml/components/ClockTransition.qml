import QtQuick

// Clock-hand wipe: a clockwise black curtain carries the old screen away.
// The moving edge has a wide blue base at the rim that tapers to a point at
// the center. No clock face, numerals or literal clock hands are drawn.
Item {
    id: root

    property bool playing: false
    property bool windowVisible: true
    property bool animationsEnabled: true
    property bool readyForReveal: true
    property bool midpointSent: false
    property bool uncovering: false
    property bool revealScheduled: false
    property real coverage: 0
    property real reveal: 0

    signal midpoint()
    signal finished()

    visible: playing && windowVisible
    enabled: false // Decorative overlay never intercepts user input.

    function sendMidpoint() {
        if (midpointSent) return
        midpointSent = true
        midpoint()
    }

    function maybeReveal() {
        if (!playing || !midpointSent || !readyForReveal || revealScheduled || !windowVisible) return
        revealScheduled = true
        revealHold.start()
    }

    function play() {
        if (playing) return
        midpointSent = false
        uncovering = false
        revealScheduled = false
        coverage = 0
        reveal = 0
        playing = true
        wipe.requestPaint()
        if (animationsEnabled && windowVisible) coverMotion.start()
        else {
            sendMidpoint()
            conclude()
        }
    }

    function conclude() {
        coverMotion.stop()
        revealMotion.stop()
        revealHold.stop()
        playing = false
        coverage = 0
        reveal = 0
        uncovering = false
        revealScheduled = false
        finished()
    }

    // Never keep a decorative animation running while in the system tray.
    onWindowVisibleChanged: {
        if (!windowVisible && playing) {
            coverMotion.stop()
            revealMotion.stop()
            revealHold.stop()
            sendMidpoint()
            conclude()
        }
    }
    onReadyForRevealChanged: maybeReveal()
    onCoverageChanged: wipe.requestPaint()
    onRevealChanged: wipe.requestPaint()
    onUncoveringChanged: wipe.requestPaint()

    SequentialAnimation {
        id: coverMotion
        NumberAnimation {
            target: root
            property: "coverage"
            from: 0
            to: 1
            duration: 420
            easing.type: Easing.InOutCubic
        }
        // Keep the screen entirely black at the exact moment the view swaps.
        PauseAnimation { duration: 65 }
        ScriptAction {
            script: {
                root.uncovering = true
                root.sendMidpoint()
                root.maybeReveal()
            }
        }
    }

    // The next photo is preloaded during the cover. With a very large image,
    // stay fully black until Qt has decoded it rather than revealing a blank.
    Timer {
        id: revealHold
        interval: 70
        repeat: false
        onTriggered: { if (root.playing && root.windowVisible) revealMotion.start() }
    }

    SequentialAnimation {
        id: revealMotion
        NumberAnimation {
            target: root
            property: "reveal"
            from: 0
            to: 1
            duration: 420
            easing.type: Easing.InOutCubic
        }
        ScriptAction { script: root.conclude() }
    }

    Canvas {
        id: wipe
        anchors.fill: parent
        antialiasing: true

        function fillSector(ctx, cx, cy, radius, first, last) {
            if (last <= first) return
            ctx.beginPath()
            ctx.moveTo(cx, cy)
            ctx.arc(cx, cy, radius, first, last, false)
            ctx.closePath()
            ctx.fill()
        }

        function drawBlueEdge(ctx, cx, cy, radius, direction) {
            // Triangular blade: the tip is at the center, while its outer
            // edge widens into a cyan-blue streak along the moving curtain.
            const trailing = direction - 0.056
            const leading = direction + 0.026
            ctx.save()
            ctx.globalCompositeOperation = "source-over"
            ctx.beginPath()
            ctx.moveTo(cx, cy)
            ctx.lineTo(cx + radius * Math.cos(trailing), cy + radius * Math.sin(trailing))
            ctx.lineTo(cx + radius * Math.cos(leading), cy + radius * Math.sin(leading))
            ctx.closePath()
            const glow = ctx.createLinearGradient(cx, cy,
                cx + radius * Math.cos(direction), cy + radius * Math.sin(direction))
            glow.addColorStop(0.0, "rgba(18,110,230,0.35)")
            glow.addColorStop(0.45, "rgba(0,128,249,0.82)")
            glow.addColorStop(1.0, "rgba(15,168,255,0.96)")
            ctx.fillStyle = glow
            ctx.fill()

            // Narrow brighter ridge next to the black edge, tapering with
            // the same geometry. No persistent visual element remains idle.
            ctx.beginPath()
            ctx.moveTo(cx, cy)
            ctx.lineTo(cx + radius * Math.cos(direction + 0.005), cy + radius * Math.sin(direction + 0.005))
            ctx.lineTo(cx + radius * Math.cos(direction + 0.014), cy + radius * Math.sin(direction + 0.014))
            ctx.closePath()
            ctx.fillStyle = "rgba(115,220,255,0.78)"
            ctx.fill()
            ctx.restore()
        }

        onPaint: {
            const ctx = getContext("2d")
            ctx.save()
            ctx.globalCompositeOperation = "source-over"
            ctx.clearRect(0, 0, width, height)
            if (!root.playing) {
                ctx.restore()
                return
            }

            const turn = 2 * Math.PI
            const start = -Math.PI / 2 // Starts at 12 o'clock, clockwise.
            const cx = width / 2
            const cy = height / 2
            const radius = Math.sqrt(width * width + height * height) + 4
            const portion = root.uncovering ? root.reveal : root.coverage
            const angle = Math.max(0, Math.min(1, portion)) * turn

            if (root.uncovering) {
                ctx.fillStyle = "#000000"
                ctx.fillRect(0, 0, width, height)
                if (angle >= turn - 0.004) {
                    ctx.clearRect(0, 0, width, height)
                } else if (angle > 0) {
                    const feather = Math.min(0.12, angle, turn - angle)
                    ctx.globalCompositeOperation = "destination-out"
                    ctx.fillStyle = "#000000"
                    wipe.fillSector(ctx, cx, cy, radius, start, start + angle - feather)
                    for (let i = 0; i < 8; ++i) {
                        const edgeStart = start + angle - feather + feather * i / 8
                        const edgeEnd = start + angle - feather + feather * (i + 1) / 8
                        ctx.fillStyle = "rgba(0,0,0," + (0.9 - i * 0.1) + ")"
                        wipe.fillSector(ctx, cx, cy, radius, edgeStart, edgeEnd)
                    }
                }
            } else if (angle >= turn - 0.004) {
                ctx.fillStyle = "#000000"
                ctx.fillRect(0, 0, width, height)
            } else if (angle > 0) {
                const feather = Math.min(0.12, angle, turn - angle)
                ctx.fillStyle = "#000000"
                wipe.fillSector(ctx, cx, cy, radius, start, start + angle - feather)
                for (let i = 0; i < 8; ++i) {
                    const edgeStart = start + angle - feather + feather * i / 8
                    const edgeEnd = start + angle - feather + feather * (i + 1) / 8
                    ctx.fillStyle = "rgba(0,0,0," + (0.9 - i * 0.1) + ")"
                    wipe.fillSector(ctx, cx, cy, radius, edgeStart, edgeEnd)
                }
            }
            if (angle > 0.008 && angle < turn - 0.008)
                wipe.drawBlueEdge(ctx, cx, cy, radius, start + angle)
            ctx.restore()
        }
    }
}
