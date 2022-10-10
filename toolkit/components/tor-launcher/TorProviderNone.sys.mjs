import { TorProviderBase } from "moz-src:///toolkit/components/tor-launcher/TorProviderBase.sys.mjs";

const lazy = {};
ChromeUtils.defineESModuleGetters(lazy, {
  TorLauncherUtil:
    "moz-src:///toolkit/components/tor-launcher/TorLauncherUtil.sys.mjs",
});

/**
 * A provider that only sets the proxy settings.
 */
export class TorProviderNone extends TorProviderBase {
  async _initInternal() {
    lazy.TorLauncherUtil.setProxyConfiguration(
      lazy.TorLauncherUtil.getPreferredSocksConfiguration()
    );
  }

  async _uninitInternal() {}
}
