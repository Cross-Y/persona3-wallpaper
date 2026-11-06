"""Prevent invalid resource identifier names in cxx-qt generated C++."""
import pathlib
import re
import unittest

PROJECT = pathlib.Path(__file__).resolve().parents[1]


class ResourceNamingTests(unittest.TestCase):
    def test_cpp_resource_identifier(self):
        source = (PROJECT / "app" / "build.rs").read_text()
        match = re.search(r'\.join\("([^"\n]+\.qrc)"\)', source)
        self.assertIsNotNone(match, "Expected a generated .qrc filename")
        filename = pathlib.Path(match.group(1)).stem + "_qrc"
        self.assertRegex(filename, r"^[A-Za-z_][A-Za-z_0-9]*$")

    def test_resource_aliases_still_extensionless(self):
        source = (PROJECT / "app" / "build.rs").read_text()
        self.assertIn('format!("assets/{stem}")', source)
        self.assertIn('"persona3-background"', source)
        self.assertIn('"persona3-settings"', source)


if __name__ == "__main__":
    unittest.main()
