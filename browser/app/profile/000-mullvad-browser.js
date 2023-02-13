// Preferences specific to Mullvad Browser

pref("browser.startup.homepage", "about:mullvad-browser");

// Do not show the bookmark panel for now, because it makes the initial browser
// window (about:home) bigger, and regular pages will show letterbox margins as
// a result.
pref("browser.toolbars.bookmarks.visibility", "never");

// privacy-browser#19: Enable Mullvad's DOH
pref("network.trr.uri", "https://doh.mullvad.net/dns-query");
pref("network.trr.default_provider_uri", "https://doh.mullvad.net/dns-query");
pref("network.trr.mode", 3);
pref("doh-rollout.provider-list", "[{\"UIName\":\"Mullvad\",\"autoDefault\":true,\"canonicalName\":\"\",\"id\":\"mullvad\",\"last_modified\":0,\"schema\":0,\"uri\":\"https://doh.mullvad.net/dns-query\"},{\"UIName\":\"Mullvad (Ad-blocking)\",\"autoDefault\":false,\"canonicalName\":\"\",\"id\":\"mullvad\",\"last_modified\":0,\"schema\":0,\"uri\":\"https://adblock.doh.mullvad.net/dns-query\"}]");

// privacy-browser#37: Customization for the about dialog
pref("app.releaseNotesURL.aboutDialog", "about:blank");
