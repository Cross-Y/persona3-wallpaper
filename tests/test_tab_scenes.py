"""Static checks for replaceable backgrounds and the blue-edged clock wipe."""
from pathlib import Path
import unittest
from test_ui_lifecycle import balanced_qml

ROOT = Path(__file__).resolve().parents[1]
QML = ROOT / 'app' / 'qml'
ASSETS = ROOT / 'app' / 'assets'


class TabSceneTests(unittest.TestCase):
    def test_four_replaceable_image_resources_are_bundled(self):
        background = (QML / 'components/AppBackground.qml').read_text()
        build = (ROOT / 'app/build.rs').read_text()
        stems = ('persona3-background', 'persona3-url', 'persona3-appearance', 'persona3-settings')
        for stem in stems:
            with self.subTest(stem=stem):
                images = [ASSETS / (stem + '.' + ext) for ext in ('webp', 'png', 'jpeg', 'jpg')]
                self.assertTrue(any(file.is_file() for file in images))
                self.assertIn(stem, build)
                self.assertIn(stem, background)
                self.assertGreater(max((file.stat().st_size for file in images if file.is_file())), 10_000)
        self.assertIn('fn artwork(stem: &str)', build)
        self.assertIn('.qrc(&qrc_file)', build)
        self.assertIn('function artForPage(page)', background)
        self.assertIn('case "URL"', background)
        self.assertIn('case "Appearance"', background)
        self.assertIn('case "Settings"', background)
        self.assertIn('default: return', background)

    def test_page_swap_happens_under_black_cover(self):
        main = (QML / 'Main.qml').read_text()
        clock = (QML / 'components/ClockTransition.qml').read_text()
        self.assertIn('onPageSelected: page => window.navigateTo(page)', main)
        self.assertIn('appBackground.prepareScene(page)', main)
        self.assertIn('appBackground.commitScene(window.transitionTarget)', main)
        self.assertIn('window.currentPage = window.transitionTarget', main)
        self.assertIn('appBackground.finishScene()', main)
        self.assertIn('queuedPage', main)
        self.assertIn('property: "coverage"', clock)
        self.assertIn('duration: 420', clock)
        self.assertIn('PauseAnimation { duration: 65 }', clock)
        self.assertIn('root.sendMidpoint()', clock)
        self.assertIn('property: "reveal"', clock)
        self.assertLess(clock.index('property: "coverage"'), clock.index('root.sendMidpoint()'))
        self.assertLess(clock.index('root.sendMidpoint()'), clock.index('property: "reveal"'))

    def test_black_wipe_and_blue_triangle_geometry(self):
        clock = (QML / 'components/ClockTransition.qml').read_text()
        self.assertIn('const start = -Math.PI / 2', clock)
        self.assertIn('Math.sqrt(width * width + height * height) + 4', clock)
        self.assertIn('ctx.globalCompositeOperation = "destination-out"', clock)
        self.assertIn('ctx.fillStyle = "#000000"', clock)
        self.assertIn('ctx.fillRect(0, 0, width, height)', clock)
        self.assertIn('ctx.clearRect(0, 0, width, height)', clock)
        self.assertIn('function drawBlueEdge(', clock)
        self.assertIn('const trailing = direction - 0.056', clock)
        self.assertIn('const leading = direction + 0.026', clock)
        self.assertIn('ctx.moveTo(cx, cy)', clock)
        self.assertIn('ctx.lineTo(cx + radius * Math.cos(trailing)', clock)
        self.assertIn('ctx.lineTo(cx + radius * Math.cos(leading)', clock)
        self.assertIn('ctx.createLinearGradient(', clock)
        self.assertIn('wipe.drawBlueEdge(', clock)
        self.assertIn('angle > 0.008 && angle < turn - 0.008', clock)
        self.assertNotIn('fillText(', clock)
        self.assertNotIn('ctx.fillText(', clock)

    def test_image_preload_waits_before_revealing(self):
        main = (QML / 'Main.qml').read_text()
        clock = (QML / 'components/ClockTransition.qml').read_text()
        background = (QML / 'components/AppBackground.qml').read_text()
        self.assertIn('readyForReveal: appBackground.pendingReady', main)
        self.assertIn('onReadyForRevealChanged: maybeReveal()', clock)
        self.assertIn('if (!playing || !midpointSent || !readyForReveal', clock)
        self.assertIn('revealHold.start()', clock)
        self.assertIn('readonly property bool pendingReady', background)
        self.assertIn('imageStatus === Image.Ready', background)
        self.assertIn('function prepareScene(page)', background)
        self.assertIn('function commitScene(page)', background)

    def test_loaded_scenes_are_discarded_in_tray_and_after_transition(self):
        background = (QML / 'components/AppBackground.qml').read_text()
        self.assertEqual(background.count('    Image {'), 2)
        self.assertIn('source: background.visible ? background.frontSource : ""', background)
        self.assertIn('source: background.visible ? background.backSource : ""', background)
        self.assertEqual(background.count('cache: false'), 2)
        self.assertIn('if (frontActive) backSource = ""', background)
        self.assertIn('else frontSource = ""', background)
        main = (QML / 'Main.qml').read_text()
        self.assertIn('visible: window.visible', main)

    def test_fast_navigation_and_tray_hide_still_work(self):
        main = (QML / 'Main.qml').read_text()
        clock = (QML / 'components/ClockTransition.qml').read_text()
        self.assertIn('if (clockTransition.playing)', main)
        self.assertIn('queuedPage = page', main)
        self.assertIn('window.navigateTo(destination)', main)
        self.assertIn('if (!windowVisible && playing)', clock)
        self.assertIn('coverMotion.stop()', clock)
        self.assertIn('revealMotion.stop()', clock)
        self.assertIn('sendMidpoint()', clock)
        self.assertIn('finished()', clock)

    def test_qml_structure(self):
        for name in ('components/AppBackground.qml', 'components/ClockTransition.qml', 'Main.qml'):
            with self.subTest(file=name):
                self.assertTrue(balanced_qml((QML / name).read_text()))

    def test_wallpaper_backend_untouched_by_visuals(self):
        main = (QML / 'Main.qml').read_text()
        self.assertIn('wallpaperController.apply_wallpaper(', main)
        self.assertIn('onApplyWallpaperRequested:', main)


if __name__ == '__main__':
    unittest.main()
