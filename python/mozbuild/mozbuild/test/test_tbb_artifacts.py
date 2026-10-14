# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import tempfile
from unittest import TestCase, mock

import mozunit

from mozbuild import tbbutils
from mozbuild.artifacts import Artifacts
from mozbuild.tbbutils import TOR_PROJECT_BROWSER_PREFIX, TOR_PROJECT_TOOLCHAINS_URL

BASE_URL = (
    f"{TOR_PROJECT_TOOLCHAINS_URL}/{TOR_PROJECT_BROWSER_PREFIX}-140.0/linux-x86_64"
)


class TestTorProjectArtifacts(TestCase):
    def setUp(self):
        self._tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmpdir.cleanup)
        self.log = mock.Mock()
        self.artifacts = self._make_artifacts("linux64-opt")

    def _make_artifacts(self, job):
        return Artifacts(
            tree=None,
            substs={"MOZ_APP_VERSION": "140.0"},
            defines={},
            job=job,
            log=self.log,
            cache_dir=self._tmpdir.name,
            git="git",
        )

    def test_install_from_tor_project_artifacts(self):
        with mock.patch.object(
            tbbutils,
            "find_artifact_urls",
            return_value=[f"{BASE_URL}/target.tar.xz", f"{BASE_URL}/target.zip"],
        ), mock.patch.object(
            self.artifacts, "install_from_url", return_value=0
        ) as install_from_url:
            self.assertEqual(
                self.artifacts.install_from_tor_project_artifacts("dist"), 0
            )

        self.assertEqual(
            install_from_url.call_args_list,
            [
                mock.call(f"{BASE_URL}/target.tar.xz", "dist"),
                mock.call(f"{BASE_URL}/target.zip", "dist"),
            ],
        )

    def test_install_from_tor_project_artifacts_stops_on_failure(self):
        with mock.patch.object(
            tbbutils,
            "find_artifact_urls",
            return_value=[f"{BASE_URL}/a.tar.xz", f"{BASE_URL}/b.tar.xz"],
        ), mock.patch.object(
            self.artifacts, "install_from_url", return_value=1
        ) as install_from_url:
            self.assertEqual(
                self.artifacts.install_from_tor_project_artifacts("dist"), 1
            )
        self.assertEqual(install_from_url.call_count, 1)

    def test_install_from_tor_project_artifacts_no_artifacts(self):
        with mock.patch.object(
            tbbutils, "find_artifact_urls", return_value=[]
        ), mock.patch.object(self.artifacts, "install_from_url") as install_from_url:
            with self.assertRaises(ValueError):
                self.artifacts.install_from_tor_project_artifacts("dist")
        install_from_url.assert_not_called()


if __name__ == "__main__":
    mozunit.main()
