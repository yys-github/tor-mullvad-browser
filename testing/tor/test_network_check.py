from marionette_harness import MarionetteTestCase, TorBrowserMixin

NETWORK_CHECK_URL = "https://check.torproject.org/"


class TestNetworkCheck(MarionetteTestCase, TorBrowserMixin):
    def test_network_check(self):
        self.bootstrap()
        self.marionette.navigate(NETWORK_CHECK_URL)
        self.assertRegex(
            self.marionette.title,
            r"^Congratulations\.",
            f"{NETWORK_CHECK_URL} should have the expected title.",
        )
