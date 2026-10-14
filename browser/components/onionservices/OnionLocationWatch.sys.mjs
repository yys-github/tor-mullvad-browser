/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

import { OnionLocationParent } from "moz-src:///browser/components/onionservices/OnionLocationParent.sys.mjs";

const log = console.createInstance({
  prefix: "OnionLocationWatch",
});

/**
 * This request observer listens to http-on-examine-responses to know early if a clearnet site has an onion alternative.
 */
class _OnionLocationWatch {
  init() {
    Services.obs.addObserver(this, "http-on-examine-response");
    Services.obs.addObserver(this, "http-on-examine-cached-response");
    Services.obs.addObserver(this, "http-on-examine-merged-response");
    log.debug("Started");
  }

  uninit() {
    Services.obs.removeObserver(this, "http-on-examine-response");
    Services.obs.removeObserver(this, "http-on-examine-cached-response");
    Services.obs.removeObserver(this, "http-on-examine-merged-response");
    log.debug("Stopped");
  }

  // nsIObserver implementation
  observe(subject, topic) {
    try {
      let channel = ChannelWrapper.get(
        subject.QueryInterface(Ci.nsIHttpChannel)
      );
      switch (topic) {
        case "http-on-examine-cached-response":
        case "http-on-examine-merged-response":
        case "http-on-examine-response":
          this.onResponse(channel);
          break;
      }
    } catch (e) {
      log.error(e);
    }
  }

  onResponse(channel) {
    this.checkResponseForOnionLocation(channel);
  }

  checkResponseForOnionLocation(channel) {
    const browser = channel.browserElement;
    const uri = channel.finalURI;

    if (!browser || uri?.scheme != "https") {
      return;
    }

    const isTopLevelDoc =
      channel.channel.loadInfo.isTopLevelLoad && channel.channel.isDocument;
    const isOnion = uri.host?.toLowerCase().endsWith(".onion");

    if (isTopLevelDoc && !isOnion && !browser._onionLocation) {
      const onionLocation = this.getOnionLocation(channel);
      if (onionLocation) {
        OnionLocationParent.setOnionLocation(browser, onionLocation);
      }
    }
  }

  getOnionLocation(channel) {
    let onionLocation = channel
      .getResponseHeaders()
      .find(el => el.name.toLowerCase() == "onion-location")?.value;

    if (onionLocation) {
      try {
        onionLocation = new URL(onionLocation);
      } catch (e) {
        return null;
      }

      if (OnionLocationParent.validateOnionV3(onionLocation)) {
        return onionLocation;
      }
    }
    return null;
  }
}

export const OnionLocationWatch = new _OnionLocationWatch();
