#include 001-base-profile.js

// Proxy and proxy security
pref("network.proxy.type", 1);
pref("network.proxy.socks", "127.0.0.1");
pref("network.proxy.socks_port", 9150);
pref("network.proxy.socks_remote_dns", true);
pref("network.http.max-persistent-connections-per-proxy", 256);
// https://gitlab.torproject.org/legacy/trac/-/work_items/10419: prevent
// fingerprinting and exploiting of local services vulnerabilities.
pref("network.proxy.no_proxies_on", "");
// tor-browser#31065: Force proxies also for localhost
pref("network.proxy.allow_hijacking_localhost", true);
// tor-browser#41317: banned port can be fingerprinted and is not necessary,
// since there are multiple protections that prevent localhost access.
// Lock ratoinale: prevent fingerprinting of old configurations.
pref("network.security.ports.banned", "", locked);
// tor-browser#44155: Block Local Network Access (LNA)
pref("network.lna.enabled", true);
pref("network.lna.blocking", true);
pref("network.lna.block_trackers", true);
// https://gitlab.torproject.org/legacy/trac/-/work_items/5741 and
// tor-browser#33962: disable DNS resolution to avoid potential proxy bypasses.
// In our setup, the proxy is going to do DNS resolution.
pref("network.dns.disabled", true);
// tor-browser#41906: disable DNS over HTTPS to prevent linkability thorugh use
// of a fixed DNS server rather than the exit relay's. 5 is MODE_TRROFF.
// Also, there are concerns about the interaction with network.dns.disabled
// (tor-browser#40034).
pref("network.trr.mode", 5);

// Treat .onions as secure
pref("dom.securecontext.allowlist_onions", true);

// tor-browser#19850: disable HTTPS-Only mode for .onion domains.
// This is already false in Firefox, but we set it again in case upstream
// changes default value.
pref("dom.security.https_only_mode.upgrade_onion", false);

// tor-browser#40423, tor-browser#41137: Disable HTTP/3.
// We should re-enable it if Tor gets UDP support.
pref("network.http.http3.enable", false);

// https://gitlab.torproject.org/legacy/trac/-/work_items/7656: rely on tor to
// rebuild streams rather than on browser's retry mechanisms.
// 0 means "do not use a second HTTP connection" (see also all.js).
pref("network.http.connection-retry-timeout", 0);

// Tor Browser used to be compatible with non-Tor proxies. This feature is not
// available anymore, but this legacy preference can be still used to disable
// first-party domain circuit isolation.
// In general, it should not be used. This use-case is still supported only for
// sites that break with this isolation (and even in that case, its use should
// be reduced to the strictly required time).
pref("extensions.torbutton.use_nontor_proxy", false);

// Browser home page
pref("browser.startup.homepage", "about:tor");

// tor-browser#43864, tor-browser#40899: general browser support url.
pref("browser.base-browser-support-url", "https://support.torproject.org/tor-browser");
// Point to our feedback url rather than Mozilla's
pref("app.feedback.baseURL", "https://support.torproject.org/%LOCALE%/get-in-touch/bug-or-feedback");

// tor-browser#40701: add our custom download warning.
pref("browser.download.showTorWarning", true);

// tor-browser#45262: hide "reset PBM" burn/fire button.
pref("browser.privatebrowsing.resetPBM.enabled", false);

pref("browser.shell.checkDefaultBrowser", false);

// Tor connection setting preferences.
// See TorSettings.sys.mjs for more information.

pref("torbrowser.settings.quickstart.enabled", false);
pref("torbrowser.settings.bridges.enabled", false);
// TorBridgeSource. Initially TorBridgeSource.Invalid = -1.
pref("torbrowser.settings.bridges.source", -1);
pref("torbrowser.settings.bridges.lox_id", "");
// obfs4|meek|snowflake|etc.
pref("torbrowser.settings.bridges.builtin_type", "");
// torbrowser.settings.bridges.bridge_strings.0
// torbrowser.settings.bridges.bridge_strings.1
// etc hold the bridge lines.
pref("torbrowser.settings.proxy.enabled", false);
// TorProxyType. Initially TorProxyType.Invalid = -1.
pref("torbrowser.settings.proxy.type", -1);
pref("torbrowser.settings.proxy.address", "");
pref("torbrowser.settings.proxy.port", 0);
pref("torbrowser.settings.proxy.username", "");
pref("torbrowser.settings.proxy.password", "");
pref("torbrowser.settings.firewall.enabled", false);
// comma-delimited list of port numbers.
pref("torbrowser.settings.firewall.allowed_ports", "");


// Formerly tor-launcher defaults

pref("extensions.torlauncher.start_tor", true);
pref("extensions.torlauncher.prompt_at_startup", true);

pref("extensions.torlauncher.max_tor_log_entries", 1000);

// By default, Tor Launcher configures a TCP listener for the Tor
// control port, as defined by control_host and control_port.
// Set control_port_use_ipc to true to use an IPC object (e.g., a Unix
// domain socket) instead. You may also modify control_ipc_path to
// override the default IPC object location. If a relative path is used,
// it is handled like torrc_path (see below).
pref("extensions.torlauncher.control_host", "127.0.0.1");
pref("extensions.torlauncher.control_port", 9151);
pref("extensions.torlauncher.control_port_use_ipc", false);
pref("extensions.torlauncher.control_ipc_path", "");

// By default, Tor Launcher configures a TCP listener for the Tor
// SOCKS port. The host is taken from the network.proxy.socks pref and
// the port is taken from the network.proxy.socks_port pref.
// Set socks_port_use_ipc to true to use an IPC object (e.g., a Unix
// domain socket) instead. You may also modify socks_ipc_path to
// override the default IPC object location. If a relative path is used,
// it is handled like torrc_path (see below).
// Modify socks_port_flags to use a different set of SocksPort flags (but be
// careful).
pref("extensions.torlauncher.socks_port_use_ipc", false);
pref("extensions.torlauncher.socks_ipc_path", "");
pref("extensions.torlauncher.socks_port_flags", "ExtendedErrors IPv6Traffic PreferIPv6 KeepAliveIsolateSOCKSAuth");

// The tor_path is relative to the application directory. On Linux and
// Windows this is the Browser/ directory that contains the firefox
// executables, and on Mac OS it is the TorBrowser.app directory.
pref("extensions.torlauncher.tor_path", "");

// The torrc_path and tordatadir_path are relative to the data directory,
// which is TorBrowser-Data/ if it exists as a sibling of the application
// directory. If TorBrowser-Data/ does not exist, these paths are relative
// to the TorBrowser/ directory within the application directory.
pref("extensions.torlauncher.torrc_path", "");
pref("extensions.torlauncher.tordatadir_path", "");

// BridgeDB-related preferences (used for Moat).
pref("extensions.torlauncher.bridgedb_targets", "https://1723079976.rsc.cdn77.org|cdn.zk.mk+www.cdn77.com");
pref("extensions.torlauncher.moat_service", "https://bridges.torproject.org/moat");

// tor-browser#45237: Disable multiple profiles feature
pref("browser.profiles.enable", false);

// Log levels
pref("browser.new_identity.log_level", "Info");
pref("browser.tor_provider.log_level", "Warn");
pref("browser.tor_provider.cp_log_level", "Warn");
pref("lox.log_level", "Warn");
pref("torbrowser.bootstrap.log_level", "Info");
pref("browser.torsettings.log_level", "Warn");
pref("browser.torMoat.loglevel", "Warn");
pref("browser.tordomainisolator.loglevel", "Warn");
pref("browser.torcircuitpanel.loglevel", "Log");
pref("browser.tor_android.log_level", "Info");
pref("browser.dragdropfilter.log_level", "Warn");
pref("browser.onionAuthPrompt.loglevel", "Warn");
pref("browser.onionalias.log_level", "Warn");
pref("browser.torRequestWatch.log_level", "Warn");
