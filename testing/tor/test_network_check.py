from marionette_driver import By, Wait, errors
from marionette_driver.localization import L10n
from marionette_harness import MarionetteTestCase

NETWORK_CHECK_URL = "https://check.torproject.org/"
TOR_BOOTSTRAP_TIMEOUT = 30  # 30s

STRINGS_LOCATION = "chrome://torbutton/locale/torConnect.properties"


class TestNetworkCheck(MarionetteTestCase):
    def setUp(self):
        MarionetteTestCase.setUp(self)

        self.l10n = L10n(self.marionette)

    def attemptConnection(self, tries=1):
        if tries > 3:
            self.assertTrue(False, "Failed to connect to Tor after 3 attempts")

        connectBtn = self.marionette.find_element(By.ID, "connectButton")
        Wait(self.marionette, timeout=10).until(
            lambda _: connectBtn.is_displayed(),
            message="Timed out waiting for tor connect button to show up.",
        )
        connectBtn.click()

        try:

            def check(m):
                if not m.get_url().startswith("about:torconnect"):
                    # We have finished connecting and have been redirected.
                    return True

                try:
                    heading = self.marionette.find_element(By.ID, "tor-connect-heading")
                except errors.NoSuchElementException:
                    # Page is probably redirecting.
                    return False

                if heading.text not in [
                    self.l10n.localize_property(
                        [STRINGS_LOCATION], "torConnect.torConnecting"
                    ),
                    self.l10n.localize_property(
                        [STRINGS_LOCATION], "torConnect.torConnected"
                    ),
                ]:
                    raise ValueError("Tor connect page is not connecting or connected")

                return False

            Wait(self.marionette, timeout=TOR_BOOTSTRAP_TIMEOUT).until(check)
        except (errors.TimeoutException, ValueError):
            cancelBtn = self.marionette.find_element(By.ID, "cancelButton")
            if cancelBtn.is_displayed():
                cancelBtn.click()

            self.attemptConnection(tries + 1)

    def test_network_check(self):
        self.attemptConnection()
        self.marionette.navigate(NETWORK_CHECK_URL)
        self.assertRegex(
            self.marionette.title,
            r"^Congratulations\.",
            f"{NETWORK_CHECK_URL} should have the expected title.",
        )
