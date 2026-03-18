from ipaddress import ip_address

from marionette_driver import By
from marionette_driver.errors import NoSuchElementException
from marionette_harness import MarionetteTestCase, TorBrowserMixin


class TestCircuitIsolation(MarionetteTestCase, TorBrowserMixin):
    def tearDown(self):
        super().tearDown()

    def extract_from_check_tpo(self):
        # Fetch the IP from check.torproject.org.
        # In addition to that, since we are loading this page, we
        # perform some additional sanity checks.
        self.marionette.navigate("https://check.torproject.org/")
        # When check.tpo's check succeed (i.e., it thinks we're
        # connecting through tor), we should be able to find a h1.on,
        # with some message...
        on = self.marionette.find_element(By.CLASS_NAME, "on")
        self.assertIsNotNone(
            on,
            "h1.on not found, you might not be connected through tor",
        )
        # ... but if it fails, the message is inside a h1.off. We want
        # to make sure we do not find that either (even though there is
        # no reason for both of the h1 to be outputted at the moment).
        self.assertRaises(
            NoSuchElementException,
            self.marionette.find_element,
            By.CLASS_NAME,
            "off",
        )
        ip = self.marionette.find_element(By.TAG_NAME, "strong")
        return ip_address(ip.text.strip())

    def extract_generic(self, url):
        # Fetch the IP address from any generic page that only contains
        # the address.
        self.marionette.navigate(url)
        return ip_address(
            self.marionette.execute_script(
                "return document.documentElement.textContent"
            ).strip()
        )

    def test_circuit_isolation(self):
        self.bootstrap()
        ips = [
            self.extract_from_check_tpo(),
            self.extract_generic("https://am.i.mullvad.net/ip"),
            self.extract_generic("https://v4.ident.me"),
        ]
        self.logger.info(f"Found the following IP addresses: {ips}")
        unique_ips = set(ips)
        self.logger.info(f"Found the following unique IP addresses: {unique_ips}")
        self.assertEqual(
            len(ips),
            len(unique_ips),
            "Some of the IP addresses we got are not unique.",
        )

        # TODO: Renable the duplicate check once
        # https://gitlab.torproject.org/tpo/tpa/team/-/issues/42547 is resolved.
        # duplicate = self.extract_generic("https://test2.ifconfig.me/ip")
        # self.assertEqual(
        #     ips[-1],
        #     duplicate,
        #     "Two IPs that were expected to be equal are different, we might be over isolating!",
        # )
