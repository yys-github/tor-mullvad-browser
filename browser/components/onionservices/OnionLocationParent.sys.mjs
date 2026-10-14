// Copyright (c) 2020, The Tor Project, Inc.

import { TorStrings } from "moz-src:///toolkit/modules/TorStrings.sys.mjs";

import { BrowserUtils } from "resource://gre/modules/BrowserUtils.sys.mjs";

// Prefs

// We keep the "prioritizeonions" name, even if obsolete, in order to
// prevent the notification from being shown again to upgrading users.
const NOTIFICATION_PREF = "privacy.prioritizeonions.showNotification";

// Element IDs
const ONIONLOCATION_BOX_ID = "onion-location-box";
const ONIONLOCATION_LABEL_ID = "onion-label";

// Notification IDs
const NOTIFICATION_ID = "onion-location";
const NOTIFICATION_ANCHOR_ID = "onion-location-box";

// Strings
const STRING_ONION_AVAILABLE = TorStrings.onionLocation.onionAvailable;
const NOTIFICATION_CANCEL_LABEL = TorStrings.onionLocation.notNow;
const NOTIFICATION_CANCEL_ACCESSKEY = TorStrings.onionLocation.notNowAccessKey;
const NOTIFICATION_OK_LABEL = TorStrings.onionLocation.loadOnion;
const NOTIFICATION_OK_ACCESSKEY = TorStrings.onionLocation.loadOnionAccessKey;
const NOTIFICATION_TITLE = TorStrings.onionLocation.tryThis;
const NOTIFICATION_DESCRIPTION = TorStrings.onionLocation.description;
const NOTIFICATION_LEARN_MORE_URL =
  TorStrings.onionLocation.learnMoreURLNotification;

/**
 * This class contains the parent part of Onion Location.
 */
export class OnionLocationParent extends JSWindowActorParent {
  // Listeners are added in BrowserGlue.jsm
  receiveMessage(aMsg) {
    switch (aMsg.name) {
      case "OnionLocation:Set": {
        let browser = this.browsingContext.embedderElement;
        OnionLocationParent.setOnionLocationFallback(browser, aMsg);

        // tor-browser #40100
        // Wait for the page to fully load before showing the notification.
        // This prevents redirects that occur before onPageShow from causing it to disappear.
        // This could be handled more elegantly by changing the notification's "appear once and never again behaviour".
        // Sites that redirect after the onPageShow event are not affected and will therefore cause the notification to disappear.
        // https://gitlab.torproject.org/tpo/applications/tor-browser/-/merge_requests/2176#note_3459271
        OnionLocationParent.showNotification(browser);
        break;
      }
    }
  }

  // Maintains compatibility with <meta http-equiv="onion-location">
  static setOnionLocationFallback(browser, aMsg) {
    let onionLocation;
    try {
      onionLocation = new URL(aMsg.data.uri);
    } catch (e) {
      return;
    }

    if (
      browser &&
      !browser._onionLocation &&
      OnionLocationParent.validateOnionV3(onionLocation)
    ) {
      OnionLocationParent.setOnionLocation(browser, onionLocation);
    }
  }

  static init(document) {
    document
      .getElementById(ONIONLOCATION_BOX_ID)
      .addEventListener("click", event => this.openOnionLink(event));
  }

  static openOnionLink(event) {
    const win = event.target.documentGlobal;
    const browser = win.gBrowser?.selectedBrowser;

    if (!browser?._onionLocation || event.button > 1) {
      return;
    }

    const where = BrowserUtils.whereToOpenLink(event);
    const triggeringPrincipal =
      Services.scriptSecurityManager.getSystemPrincipal();

    if (where == "current") {
      browser.webNavigation.stop(Ci.nsIWebNavigation.STOP_ALL);
      OnionLocationParent.setDisabled(browser);
    }

    const onionUrl = browser._onionLocation;
    const docUrl = new URL(browser.browsingContext.currentURI.asciiSpec);

    if (docUrl.hash && docUrl.hash != onionUrl.hash) {
      onionUrl.hash = docUrl.hash;
    }

    win.openLinkIn(onionUrl.href, where, {
      triggeringPrincipal,
    });
  }

  static onStateChange(browser) {
    delete browser._onionLocation;
    OnionLocationParent.hideNotification(browser);
  }

  static setOnionLocation(browser, onionLocation) {
    browser._onionLocation = onionLocation;
    let tabBrowser = browser.getTabBrowser();
    if (tabBrowser && browser === tabBrowser.selectedBrowser) {
      OnionLocationParent.updateOnionLocationBadge(browser);
    }
  }

  static hideNotification(browser) {
    const win = browser.documentGlobal;
    if (browser._onionLocationPrompt) {
      win.PopupNotifications.remove(browser._onionLocationPrompt);
    }
  }

  static showNotification(browser) {
    const mustShow = Services.prefs.getBoolPref(NOTIFICATION_PREF, true);
    if (!mustShow || !browser._onionLocation) {
      return;
    }

    const win = browser.documentGlobal;
    Services.prefs.setBoolPref(NOTIFICATION_PREF, false);

    const mainAction = {
      label: NOTIFICATION_OK_LABEL,
      accessKey: NOTIFICATION_OK_ACCESSKEY,
      callback(obj) {
        OnionLocationParent.openOnionLink(obj.event);
      },
    };

    const cancelAction = {
      label: NOTIFICATION_CANCEL_LABEL,
      accessKey: NOTIFICATION_CANCEL_ACCESSKEY,
      callback: () => {},
    };

    win.document.getElementById("onion-location-body-text").textContent =
      NOTIFICATION_DESCRIPTION;

    const options = {
      autofocus: true,
      persistent: true,
      removeOnDismissal: false,
      eventCallback(aTopic) {
        if (aTopic === "removed") {
          delete browser._onionLocationPrompt;
        }
      },
      learnMoreURL: NOTIFICATION_LEARN_MORE_URL,
      hideClose: true,
      popupOptions: {
        position: "bottomright topright",
      },
    };

    // A hacky way of setting the popup anchor outside the usual url bar icon
    // box, similar to CRF and addons.
    // See: https://searchfox.org/mozilla-esr115/rev/7962d6b7b17ee105ad64ab7906af2b9179f6e3d2/toolkit/modules/PopupNotifications.sys.mjs#46
    browser[NOTIFICATION_ANCHOR_ID + "popupnotificationanchor"] =
      win.document.getElementById(NOTIFICATION_ANCHOR_ID);

    browser._onionLocationPrompt = win.PopupNotifications.show(
      browser,
      NOTIFICATION_ID,
      NOTIFICATION_TITLE,
      NOTIFICATION_ANCHOR_ID,
      mainAction,
      [cancelAction],
      options
    );
  }

  static setEnabled(browser) {
    const win = browser.documentGlobal;
    const label = win.document.getElementById(ONIONLOCATION_LABEL_ID);
    label.textContent = STRING_ONION_AVAILABLE;
    const elem = win.document.getElementById(ONIONLOCATION_BOX_ID);
    elem.removeAttribute("hidden");
  }

  static setDisabled(browser) {
    const win = browser.documentGlobal;
    const elem = win.document.getElementById(ONIONLOCATION_BOX_ID);
    elem.setAttribute("hidden", true);
  }

  static updateOnionLocationBadge(browser) {
    if (browser._onionLocation) {
      OnionLocationParent.setEnabled(browser);
    } else {
      OnionLocationParent.setDisabled(browser);
    }
  }

  static validateOnionV3(url) {
    if (!["http:", "https:"].includes(url.protocol)) {
      return false;
    }
    return /(^|\.)[a-z2-7]{56}\.onion$/i.test(url.hostname);
  }
}
