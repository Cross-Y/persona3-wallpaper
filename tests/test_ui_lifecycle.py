"""Static regression checks for QML lifetime changes (not a Qt runtime test)."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1] / "app" / "qml"


def balanced_qml(source):
    """Check delimiter pairing outside quoted strings and // or /* */ comments."""
    stack, index, mode = [], 0, "code"
    while index < len(source):
        c = source[index]
        following = source[index:index + 2]
        if mode in ("'", '"'):
            if c == "\\":
                index += 2
                continue
            if c == mode:
                mode = "code"
        elif mode == "line_comment":
            if c == "\n":
                mode = "code"
        elif mode == "block_comment":
            if following == "*/":
                mode = "code"
                index += 2
                continue
        elif following == "//":
            mode = "line_comment"
            index += 2
            continue
        elif following == "/*":
            mode = "block_comment"
            index += 2
            continue
        elif c in ("'", '"'):
            mode = c
        elif c in "{[(":
            stack.append(c)
        elif c in "}])":
            if not stack or (stack.pop(), c) not in (("{", "}"), ("[", "]"), ("(", ")")):
                return False
        index += 1
    return not stack and mode in ("code", "line_comment")


class UiLifecycleTest(unittest.TestCase):
    def text(self, name):
        return (ROOT / name).read_text()

    def test_qml_delimiters(self):
        for filename in ["Main.qml", "views/LibraryView.qml", "views/UrlView.qml", "views/SettingsView.qml", "views/AppearanceView.qml", "components/AppBackground.qml"]:
            with self.subTest(filename=filename):
                self.assertTrue(balanced_qml(self.text(filename)))

    def test_only_one_page_active(self):
        main = self.text("Main.qml")
        for name in ("Library", "URL", "Appearance", "Settings"):
            self.assertIn(f'active: window.visible && window.currentPage === "{name}"', main)
        self.assertEqual(main.count("            Loader {"), 4)
        self.assertNotIn("settingsView.", main)
        self.assertNotIn("libraryView.", main)
        self.assertIn("runtimeSettings.engineActive", main)
        self.assertIn("runtimeSettings.backgroundMode", main)

    def test_catalog_has_one_shot_lifecycle(self):
        catalog = self.text("views/UrlView.qml")
        self.assertEqual(catalog.count("Component.onCompleted:"), 1)
        self.assertIn("Component.onDestruction:", catalog)
        self.assertIn("initialCatalogTimer.stop()", catalog)
        self.assertNotIn("Qt.callLater(", catalog)
        self.assertIn("results: view.results", catalog)

    def test_background_unloads_image(self):
        background = self.text("components/AppBackground.qml")
        self.assertIn('source: background.visible ?', background)
        self.assertIn('cache: false', background)

    def test_settings_are_shared_with_tray(self):
        settings = self.text("views/SettingsView.qml")
        self.assertIn('property var appState', settings)
        self.assertIn('root.appState[propertyName] = value', settings)
        self.assertIn('root.appState.statusMessage', settings)


if __name__ == "__main__":
    unittest.main()
