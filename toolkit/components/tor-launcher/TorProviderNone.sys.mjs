import { TorProviderBase } from "resource://gre/modules/TorProviderBase.sys.mjs";

const lazy = {};
ChromeUtils.defineESModuleGetters(lazy, {
  TorLauncherUtil: "resource://gre/modules/TorLauncherUtil.sys.mjs",
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
