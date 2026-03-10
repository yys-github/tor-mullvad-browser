/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

const lazy = {};
ChromeUtils.defineESModuleGetters(lazy, {
  EventDispatcher: "resource://gre/modules/Messaging.sys.mjs",
  TorConnect: "resource://gre/modules/TorConnect.sys.mjs",
  TorConnectTopics: "resource://gre/modules/TorConnect.sys.mjs",
  TorSettingsTopics: "resource://gre/modules/TorSettings.sys.mjs",
  TorProviderBuilder: "resource://gre/modules/TorProviderBuilder.sys.mjs",
  TorProviderTopics: "resource://gre/modules/TorProviderBuilder.sys.mjs",
  TorSettings: "resource://gre/modules/TorSettings.sys.mjs",
  SecurityLevelPrefs: "resources://gre/modules/SecurityLevel.sys.mjs",
});

const Prefs = Object.freeze({
  logLevel: "browser.tor_android.log_level",
});

const logger = console.createInstance({
  maxLogLevelPref: Prefs.logLevel,
  prefix: "TorAndroidIntegration",
});

const EmittedEvents = Object.freeze({
  settingsReady: "GeckoView:Tor:SettingsReady",
  settingsChanged: "GeckoView:Tor:SettingsChanged",
  connectStageChanged: "GeckoView:Tor:ConnectStageChanged",
  bootstrapProgress: "GeckoView:Tor:BootstrapProgress",
  bootstrapComplete: "GeckoView:Tor:BootstrapComplete",
  torLogs: "GeckoView:Tor:Logs",
});

const ListenedEvents = Object.freeze({
  securityLevelGet: "GeckoView:Tor:SecurityLevelGet",
  securityLevelSetBeforeRestart: "GeckoView:Tor:SecurityLevelSetBeforeRestart",
  settingsGet: "GeckoView:Tor:SettingsGet",
  // The data is passed directly to TorSettings.
  settingsSet: "GeckoView:Tor:SettingsSet",
  bootstrapBegin: "GeckoView:Tor:BootstrapBegin",
  // Optionally takes a countryCode, as data.countryCode.
  bootstrapBeginAuto: "GeckoView:Tor:BootstrapBeginAuto",
  bootstrapCancel: "GeckoView:Tor:BootstrapCancel",
  startAgain: "GeckoView:Tor:StartAgain",
  quickstartGet: "GeckoView:Tor:QuickstartGet",
  quickstartSet: "GeckoView:Tor:QuickstartSet",
  regionNamesGet: "GeckoView:Tor:RegionNamesGet",
  shouldShowTorConnectGet: "GeckoView:Tor:ShouldShowTorConnect",
});

/**
 * The implementation for the global `TorAndroidIntegration` object.
 */
class TorAndroidIntegrationImpl {
  #initialized = false;

  /**
   * Register our listeners.
   * We want this function to block GeckoView initialization, so it should not be
   * async. Any async task should be moved to #deferredInit, instead.
   */
  init() {
    if (this.#initialized) {
      logger.warn("Something tried to initilize us again.");
      return;
    }
    this.#initialized = true;

    lazy.EventDispatcher.instance.registerListener(
      this,
      Object.values(ListenedEvents)
    );

    Services.obs.addObserver(this, lazy.TorProviderTopics.TorLog);

    for (const topic in lazy.TorConnectTopics) {
      Services.obs.addObserver(this, lazy.TorConnectTopics[topic]);
    }

    for (const topic in lazy.TorSettingsTopics) {
      Services.obs.addObserver(this, lazy.TorSettingsTopics[topic]);
    }

    lazy.TorProviderBuilder.init();
    // On Android immediately call firstWindowLoaded. This should be safe to
    // call since it will await the initialisation of the TorProvider set up
    // by TorProviderBuilder.init.
    lazy.TorProviderBuilder.firstWindowLoaded();

    this.#deferredInit();
  }

  /**
   * Perform our init tasks that should not block the initialization of
   * GeckoView. This function will not be awaited, so errors can only be logged.
   */
  async #deferredInit() {
    try {
      await lazy.TorSettings.init();
      await lazy.TorConnect.init();
    } catch (e) {
      logger.error("Cannot initialize TorSettings or TorConnect", e);
    }
  }

  observe(subj, topic) {
    switch (topic) {
      case lazy.TorConnectTopics.StageChange:
        lazy.EventDispatcher.instance.sendRequest({
          type: EmittedEvents.connectStageChanged,
          stage: subj.wrappedJSObject ?? "",
        });
        break;
      case lazy.TorConnectTopics.RegionNamesChange:
        // TODO: Respond to change in region names if we are showing a
        // TorConnectStage that uses them.
        break;
      case lazy.TorConnectTopics.BootstrapProgress:
        lazy.EventDispatcher.instance.sendRequest({
          type: EmittedEvents.bootstrapProgress,
          progress: subj.wrappedJSObject.progress ?? 0,
          hasWarnings: subj.wrappedJSObject.hasWarnings ?? false,
        });
        break;
      case lazy.TorConnectTopics.BootstrapComplete:
        lazy.EventDispatcher.instance.sendRequest({
          type: EmittedEvents.bootstrapComplete,
        });
        break;
      case lazy.TorProviderTopics.TorLog:
        lazy.EventDispatcher.instance.sendRequest({
          type: EmittedEvents.torLogs,
          logType: subj.wrappedJSObject.type ?? "",
          message: subj.wrappedJSObject.msg ?? "",
          timestamp: subj.wrappedJSObject.timestamp ?? "",
        });
        break;
      case lazy.TorSettingsTopics.Ready:
        lazy.EventDispatcher.instance.sendRequest({
          type: EmittedEvents.settingsReady,
          settings: lazy.TorSettings.getSettings(),
        });
        break;
      case lazy.TorSettingsTopics.SettingsChanged:
        // For Android we push also the settings object to avoid a round trip on
        // the event dispatcher.
        lazy.EventDispatcher.instance.sendRequest({
          type: EmittedEvents.settingsChanged,
          changes: subj.wrappedJSObject.changes ?? [],
          settings: lazy.TorSettings.getSettings(),
        });
        break;
    }
  }

  async onEvent(event, data, callback) {
    logger.debug(`Received event ${event}`, data);
    try {
      switch (event) {
        case ListenedEvents.securityLevelGet:
          // "standard"/"safer"/"safest"
          // TODO: Switch to securityLevelSummary to allow android to handle
          // "custom" security level. tor-browser#43819
          callback?.onSuccess(lazy.SecurityLevelPrefs.securityLevel);
          break;
        case ListenedEvents.securityLevelSetBeforeRestart:
          lazy.SecurityLevelPrefs.setSecurityLevelBeforeRestart(data.levelName);
          // Let the caller know that the setting is applied and the browser
          // should be restarted now.
          // NOTE: The caller must wait for this callback before triggering
          // the restart.
          callback?.onSuccess();
          break;
        case ListenedEvents.settingsGet:
          callback?.onSuccess(lazy.TorSettings.getSettings());
          return;
        case ListenedEvents.settingsSet:
          // TODO: Handle setting throw? This can throw if data.settings is
          // incorrectly formatted, but more like it can throw when the settings
          // fail to be passed onto the TorProvider. tor-browser#43405.
          await lazy.TorSettings.changeSettings(data.settings);
          break;
        case ListenedEvents.bootstrapBegin:
          lazy.TorConnect.beginBootstrapping();
          break;
        case ListenedEvents.bootstrapBeginAuto:
          // TODO: The countryCode should be set to "automatic" by the caller
          // rather than `null`, so we can just pass in `data.countryCode`
          // directly.
          lazy.TorConnect.beginBootstrapping(data.countryCode || "automatic");
          break;
        case ListenedEvents.bootstrapCancel:
          lazy.TorConnect.cancelBootstrapping();
          break;
        case ListenedEvents.startAgain:
          lazy.TorConnect.startAgain();
          break;
        case ListenedEvents.quickstartGet:
          callback?.onSuccess(lazy.TorConnect.quickstart);
          return;
        case ListenedEvents.quickstartSet:
          lazy.TorConnect.quickstart = data.enabled;
          break;
        case ListenedEvents.regionNamesGet:
          callback?.onSuccess(lazy.TorConnect.getRegionNames());
          return;
        case ListenedEvents.shouldShowTorConnectGet:
          callback?.onSuccess(lazy.TorConnect.shouldShowTorConnect());
          return;
      }
      callback?.onSuccess();
    } catch (e) {
      logger.error(`Error while handling event ${event}`, e);
      callback?.onError(e);
    }
  }
}

export const TorAndroidIntegration = new TorAndroidIntegrationImpl();
