import { setTimeout, clearTimeout } from "resource://gre/modules/Timer.sys.mjs";
import { TorProviderBase } from "moz-src:///toolkit/components/tor-launcher/TorProviderBase.sys.mjs";
import { TorProviderTopics } from "moz-src:///toolkit/components/tor-launcher/TorProviderBuilder.sys.mjs";

const kBootstrapSteps = [
  { progress: 5, tag: "starting", summary: "Starting" },
  { progress: 14, tag: "handshake", summary: "Handshaking with a relay" },
  {
    progress: 45,
    tag: "requesting_descriptors",
    summary: "Asking for relay descriptors",
  },
  {
    progress: 75,
    tag: "loading_descriptors",
    summary: "Loading relay descriptors",
  },
  { progress: 100, tag: "done", summary: "Done" },
];

const kBootstrapStepDelayMs = 500;

/**
 * A mock tor provider for testing purposes. Fakes all provider operations
 * without starting a real Tor daemon. This implementation is intentionally
 * minimal and will be extended as test requirements become clearer.
 */
export class TorProviderMock extends TorProviderBase {
  #bootstrapTimeoutIds = [];

  async _initInternal() {}

  async _uninitInternal() {
    this.#cancelBootstrap();
  }

  async writeBridgeSettings(_bridges) {}

  async writeProxySettings(_proxy) {}

  async writeFirewallSettings(_firewall) {}

  async flushSettings() {}

  async connect() {
    this.#cancelBootstrap();
    for (const [i, step] of kBootstrapSteps.entries()) {
      const id = setTimeout(
        () => {
          Services.obs.notifyObservers(
            {
              ...step,
              severity: "NOTICE",
              reason: "",
              shouldWarn: false,
              QueryInterface: ChromeUtils.generateQI([
                Ci.torITorBootstrapStatus,
              ]),
            },
            TorProviderTopics.BootstrapStatus
          );
        },
        (i + 1) * kBootstrapStepDelayMs
      );
      this.#bootstrapTimeoutIds.push(id);
    }
  }

  async stopBootstrap() {
    this.#cancelBootstrap();
  }

  #cancelBootstrap() {
    for (const id of this.#bootstrapTimeoutIds) {
      clearTimeout(id);
    }
    this.#bootstrapTimeoutIds = [];
  }

  async newnym() {}

  async getBridges() {
    return [];
  }

  async getPluggableTransports() {
    return [];
  }

  async onionAuthAdd(_address, _b64PrivateKey, _isPermanent) {}

  async onionAuthRemove(_address) {}

  async onionAuthViewKeys() {
    return [];
  }

  get currentBridge() {
    return null;
  }
}
