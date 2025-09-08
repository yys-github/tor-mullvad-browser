import os
import shutil
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import MagicMock, patch

import mozunit

from mozbuild.tbbutils import (
    get_artifact_index,
    get_artifact_path,
    list_files_http,
    symlink_tree,
)


class TestSymlinkTree(unittest.TestCase):
    def _create_sample_tree(self, base: Path):
        (base / "subdir").mkdir()
        (base / "file1.txt").write_text("content1")
        (base / "subdir" / "file2.txt").write_text("content2")

    def setUp(self):
        self.tmpdir = tempfile.mkdtemp()
        self.src = Path(self.tmpdir) / "src"
        self.dst = Path(self.tmpdir) / "dst"
        self.src.mkdir()
        self.dst.mkdir()

    def tearDown(self):
        shutil.rmtree(self.tmpdir)

    def test_symlinks_created_correctly(self):
        self._create_sample_tree(self.src)

        symlink_tree(self.src, self.dst)

        self.assertTrue((self.dst / "file1.txt").is_symlink())
        self.assertTrue((self.dst / "subdir" / "file2.txt").is_symlink())

        self.assertEqual(
            os.readlink(self.dst / "file1.txt"),
            str(self.src / "file1.txt"),
        )
        self.assertEqual(
            os.readlink(self.dst / "subdir" / "file2.txt"),
            str(self.src / "subdir" / "file2.txt"),
        )

    def test_overwrites_existing_files(self):
        self._create_sample_tree(self.src)

        # Create a conflicting file in destination
        (self.dst / "file1.txt").write_text("old")

        symlink_tree(self.src, self.dst)

        self.assertTrue((self.dst / "file1.txt").is_symlink())
        self.assertEqual(
            os.readlink(self.dst / "file1.txt"),
            str(self.src / "file1.txt"),
        )

    def test_nested_directories_are_mirrored(self):
        (self.src / "a" / "b" / "c").mkdir(parents=True)
        (self.src / "a" / "b" / "c" / "deep.txt").write_text("deep content")

        symlink_tree(self.src, self.dst)

        deep_link = self.dst / "a" / "b" / "c" / "deep.txt"
        self.assertTrue(deep_link.is_symlink())
        self.assertEqual(
            os.readlink(deep_link),
            str(self.src / "a" / "b" / "c" / "deep.txt"),
        )

    def test_idempotence(self):
        self._create_sample_tree(self.src)

        symlink_tree(self.src, self.dst)
        symlink_tree(self.src, self.dst)  # Run again

        self.assertTrue((self.dst / "file1.txt").is_symlink())
        self.assertTrue((self.dst / "subdir" / "file2.txt").is_symlink())

    def test_symlinks_use_absolute_paths(self):
        (self.src / "file.txt").write_text("absolute")

        symlink_tree(self.src, self.dst)

        link_target = os.readlink(self.dst / "file.txt")
        self.assertTrue(Path(link_target).is_absolute())
        self.assertEqual(Path(link_target), self.src / "file.txt")


class TestGetArtifactName(unittest.TestCase):
    def setUp(self):
        self.artifact = "artifact"
        self.host = "linux64"

    @patch("mozbuild.tbbutils.TOR_BROWSER_BUILD_ARTIFACTS", new=["artifact"])
    def test_artifact_in_tbb_artifacts(self):
        from mozbuild.tbbutils import get_artifact_name

        result = get_artifact_name(self.artifact, self.host)
        self.assertEqual(result, self.artifact)

    @patch("mozbuild.tbbutils.ARTIFACT_NAME_MAP", new={"artifact": "tcafitra"})
    def test_host_is_not_linux64(self):
        from mozbuild.tbbutils import get_artifact_name

        result = get_artifact_name(self.artifact, "linux64-aarch64")
        self.assertIsNone(result)

    @patch("mozbuild.tbbutils.ARTIFACT_NAME_MAP", new={"artifact": "tcafitra"})
    def test_mapped_artifact(self):
        from mozbuild.tbbutils import get_artifact_name

        result = get_artifact_name(self.artifact, self.host)
        self.assertEqual(result, self.artifact[::-1])


class TestGetArtifactIndex(unittest.TestCase):
    def test_regular_artifact(self):
        artifact = "tor"
        path = "https://tb-build-06.torproject.org/~tb-builder/tor-browser-build/out/tor/tor-b1f9824464dc-linux-x86_64-b0ffe2.tar.gz"
        expected = "tor-b1f9824464dc-linux-x86_64-b0ffe2.tar.gz"
        self.assertEqual(get_artifact_index(path, artifact), expected)

    def test_expert_bundle_artifact(self):
        artifact = "tor-expert-bundle"
        path = "https://tb-build-06.torproject.org/~tb-builder/tor-browser-build/out/tor-expert-bundle/tor-expert-bundle-linux-x86_64-tbb-nightly.2025.10.14-d9aa09/"
        expected = "tor-expert-bundle-linux-x86_64-tbb-nightly.2025.10.14-d9aa09"
        self.assertEqual(get_artifact_index(path, artifact), expected)


class TestGetArtifactPath(unittest.TestCase):
    def setUp(self):
        self.url = "http://example.com"
        self.artifact = "artifact"
        # This is just an example target which is valid. But it doesn't make
        # any difference and could be anything for these tests.
        self.target = SimpleNamespace(tor_browser_build_alias="linux", cpu="x86_64")

    @patch("mozbuild.tbbutils.list_files_http")
    def test_no_files_returns_none(self, mock_list_files):
        mock_list_files.return_value = []
        result = get_artifact_path(self.url, self.artifact, self.target)
        self.assertIsNone(result)

    @patch("mozbuild.tbbutils.list_files_http")
    def test_no_matching_files_returns_none(self, mock_list_files):
        mock_list_files.return_value = ["somethingelse.zip", "yetanotherthing.zip"]
        result = get_artifact_path(self.url, self.artifact, self.target)
        self.assertIsNone(result)

    @patch("mozbuild.tbbutils.list_files_http")
    def test_single_artifact_match(self, mock_list_files):
        mock_list_files.return_value = ["artifact-1.zip"]
        result = get_artifact_path(self.url, self.artifact, self.target)
        self.assertEqual(result, f"{self.url}/{self.artifact}/artifact-1.zip")

    @patch("mozbuild.tbbutils.list_files_http")
    def test_artifact_without_os_returns_first(self, mock_list_files):
        mock_list_files.return_value = ["artifact-1.zip", "artifact-2.zip"]
        result = get_artifact_path(self.url, self.artifact, self.target)
        self.assertTrue(result.startswith(f"{self.url}/{self.artifact}/"))
        self.assertIn("artifact-", result)

    @patch("mozbuild.tbbutils.list_files_http")
    def test_artifact_with_os_match(self, mock_list_files):
        mock_list_files.return_value = [
            "artifact-windows.zip",
            "artifact-linux.zip",
        ]
        result = get_artifact_path(self.url, self.artifact, self.target)
        self.assertEqual(result, f"{self.url}/{self.artifact}/artifact-linux.zip")

    @patch("mozbuild.tbbutils.list_files_http")
    def test_artifact_with_cpu_match(self, mock_list_files):
        mock_list_files.return_value = [
            "artifact-linux-arm.zip",
            "artifact-linux-x86_64.zip",
        ]
        result = get_artifact_path(self.url, self.artifact, self.target)
        self.assertEqual(
            result, f"{self.url}/{self.artifact}/artifact-linux-x86_64.zip"
        )

    @patch("mozbuild.tbbutils.list_files_http")
    def test_artifact_with_prefix(self, mock_list_files):
        mock_list_files.return_value = ["artifact-1.zip"]

        prefix = "prefix"
        result = get_artifact_path(self.url, self.artifact, self.target, prefix=prefix)
        self.assertEqual(result, f"{self.url}/{prefix}/artifact-1.zip")
        mock_list_files.assert_called_with(f"{self.url}/{prefix}?C=M;O=D")


class TestListFilesHttp(unittest.TestCase):
    def setUp(self):
        self.url = "http://example.com"

    @patch("mozbuild.tbbutils.urlopen")
    def test_non_200_status_returns_empty(self, mock_urlopen):
        mock_resp = MagicMock()
        mock_resp.status = 404
        mock_resp.read.return_value = b""
        mock_urlopen.return_value.__enter__.return_value = mock_resp

        result = list_files_http(self.url)
        self.assertEqual(result, [])

    @patch("mozbuild.tbbutils.urlopen")
    def test_exception_returns_empty(self, mock_urlopen):
        mock_urlopen.side_effect = Exception("network error")
        result = list_files_http(self.url)
        self.assertEqual(result, [])

    @patch("mozbuild.tbbutils.urlopen")
    def test_regular_links(self, mock_urlopen):
        html = b"""
        <html><body>
        <a href="../">Parent</a>
        <a href="file1.zip">file1</a>
        <a href="file2.zip">file2</a>
        </body></html>
        """
        mock_resp = MagicMock()
        mock_resp.status = 200
        mock_resp.read.return_value = html
        mock_urlopen.return_value.__enter__.return_value = mock_resp

        result = list_files_http(self.url)
        self.assertEqual(result, ["file1.zip", "file2.zip"])

    @patch("mozbuild.tbbutils.urlopen")
    def test_tor_expert_bundle_rewrites(self, mock_urlopen):
        html = """
            <a href="tor-expert-bundle">bundle</a>
            <a href="tor-expert-bundle-aar">bundle</a>
        """
        mock_resp = MagicMock()
        mock_resp.status = 200
        mock_resp.read.return_value = html.encode()
        mock_urlopen.return_value.__enter__.return_value = mock_resp

        result = list_files_http(self.url)
        self.assertEqual(
            result,
            [
                "tor-expert-bundle/tor-expert-bundle.tar.gz",
                "tor-expert-bundle-aar/tor-expert-bundle.aar",
            ],
        )


if __name__ == "__main__":
    mozunit.main()
