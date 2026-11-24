import os
import subprocess
import sys
import tempfile
import unittest

import buildconfig
import mozunit

# Files we add on top of Firefox that must be credited in about:license.
# Paths are relative to the top source directory.
EXPECTED_PATHS = [
    "browser/components/cryptosafety/Bech32Decode.sys.mjs",
    "toolkit/modules/QRCode.sys.mjs",
]


class TestAboutLicense(unittest.TestCase):
    """Check the about:license page rendered by ./mach licenses.

    Rebases onto upstream changes to how the page is generated can silently
    drop our changes to it, since they only show up in the rendered page.
    Requires a configured tree.
    """

    @classmethod
    def setUpClass(cls):
        with tempfile.TemporaryDirectory() as tmp:
            output = os.path.join(tmp, "license.html")
            result = subprocess.run(
                [
                    sys.executable,
                    os.path.join(buildconfig.topsrcdir, "mach"),
                    "licenses",
                    "-o",
                    output,
                ],
                cwd=buildconfig.topsrcdir,
                capture_output=True,
                text=True,
                check=False,
            )
            if result.returncode != 0:
                raise AssertionError(
                    f"./mach licenses failed:\n{result.stdout}\n{result.stderr}"
                )
            with open(output, encoding="utf-8") as f:
                cls.document = f.read()

    def test_app_license_block_is_not_included(self):
        # tor-browser#43901.
        with open(
            os.path.join(
                buildconfig.topsrcdir,
                "browser/base/content/overrides/app-license.html",
            ),
            encoding="utf-8",
        ) as f:
            block = f.read().split("-->", 1)[-1].strip()
        self.assertTrue(block)
        self.assertFalse(
            block in self.document,
            "about:license includes browser/base/content/overrides/app-license.html",
        )

    def test_expected_paths_are_credited(self):
        for path in EXPECTED_PATHS:
            with self.subTest(path=path):
                self.assertTrue(
                    f"<code>{path}</code>" in self.document,
                    f"about:license does not credit {path}",
                )


if __name__ == "__main__":
    mozunit.main()
