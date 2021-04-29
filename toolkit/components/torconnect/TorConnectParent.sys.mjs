// Copyright (c) 2021, The Tor Project, Inc.

import { TorStrings } from "resource://gre/modules/TorStrings.sys.mjs";
import {
  TorConnect,
  TorConnectTopics,
} from "resource://gre/modules/TorConnect.sys.mjs";

const lazy = {};

ChromeUtils.defineESModuleGetters(lazy, {
  HomePage: "resource:///modules/HomePage.sys.jsm",
});

/*
This object is basically a marshalling interface between the TorConnect module
and a particular about:torconnect page
*/

/**
 * Actor parent class for the about:torconnect page.
 * It adapts and relays the messages from and to the TorConnect module.
 */
export class TorConnectParent extends JSWindowActorParent {
  constructor(...args) {
    super(...args);

    const self = this;

    // JSWindowActiveParent derived objects cannot observe directly, so create a
    // member object to do our observing for us.
    //
    // This object converts the various lifecycle events from the TorConnect
    // module, and maintains a state object which we pass down to our
    // about:torconnect page, which uses the state object to update its UI.
    this.torConnectObserver = {
      observe(subject, topic) {
        const obj = subject?.wrappedJSObject;
        switch (topic) {
          case TorConnectTopics.StageChange:
            self.sendAsyncMessage("torconnect:stage-change", obj);
            break;
          case TorConnectTopics.BootstrapProgress:
            self.sendAsyncMessage("torconnect:bootstrap-progress", obj);
            break;
          case TorConnectTopics.QuickstartChange:
            self.sendAsyncMessage(
              "torconnect:quickstart-change",
              TorConnect.quickstart
            );
            break;
        }
      },
    };

    Services.obs.addObserver(
      this.torConnectObserver,
      TorConnectTopics.StageChange
    );
    Services.obs.addObserver(
      this.torConnectObserver,
      TorConnectTopics.BootstrapProgress
    );
    Services.obs.addObserver(
      this.torConnectObserver,
      TorConnectTopics.QuickstartChange
    );
  }

  didDestroy() {
    Services.obs.removeObserver(
      this.torConnectObserver,
      TorConnectTopics.StageChange
    );
    Services.obs.removeObserver(
      this.torConnectObserver,
      TorConnectTopics.BootstrapProgress
    );
    Services.obs.removeObserver(
      this.torConnectObserver,
      TorConnectTopics.QuickstartChange
    );
  }

  async receiveMessage(message) {
    switch (message.name) {
      case "torconnect:should-show":
        return Promise.resolve(TorConnect.shouldShowTorConnect);
      case "torconnect:home-page":
        // If there are multiple home pages, just load the first one.
        return Promise.resolve(TorConnect.fixupURIs(lazy.HomePage.get())[0]);
      case "torconnect:set-quickstart":
        TorConnect.quickstart = message.data;
        break;
      case "torconnect:open-tor-preferences":
        this.browsingContext.top.embedderElement.ownerGlobal.openPreferences(
          "connection"
        );
        break;
      case "torconnect:view-tor-logs":
        this.browsingContext.top.embedderElement.ownerGlobal.openPreferences(
          "connection-viewlogs"
        );
        break;
      case "torconnect:restart":
        Services.startup.quit(
          Ci.nsIAppStartup.eRestart | Ci.nsIAppStartup.eAttemptQuit
        );
        break;
      case "torconnect:start-again":
        TorConnect.startAgain();
        break;
      case "torconnect:choose-region":
        TorConnect.chooseRegion();
        break;
      case "torconnect:begin-bootstrapping":
        TorConnect.beginBootstrapping(message.data.regionCode);
        break;
      case "torconnect:cancel-bootstrapping":
        TorConnect.cancelBootstrapping();
        break;
      case "torconnect:get-init-args":
        // Called on AboutTorConnect.init(), pass down all state data it needs
        // to init.
        return {
          TorStrings,
          Direction: Services.locale.isAppLocaleRTL ? "rtl" : "ltr",
          CountryNames: TorConnect.countryNames,
          stage: TorConnect.stage,
          quickstartEnabled: TorConnect.quickstart,
        };
      case "torconnect:get-country-codes":
        return TorConnect.getCountryCodes();
    }
    return undefined;
  }
}
