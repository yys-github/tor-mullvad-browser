import { SettingGroupManager } from "chrome://browser/content/preferences/config/SettingGroupManager.mjs";
import { Preferences } from "chrome://global/content/preferences/Preferences.mjs";

const lazy = {};
ChromeUtils.defineESModuleGetters(lazy, {
  InternetStatus: "moz-src:///toolkit/modules/TorConnect.sys.mjs",
  openBridgeDialog:
    "chrome://browser/content/torpreferences/config/helpers.mjs",
  openUserProvideBridgeDialog:
    "chrome://browser/content/torpreferences/config/helpers.mjs",
  TorBridgeSource: "moz-src:///toolkit/modules/TorSettings.sys.mjs",
  TorConnect: "moz-src:///toolkit/modules/TorConnect.sys.mjs",
  TorConnectStage: "moz-src:///toolkit/modules/TorConnect.sys.mjs",
  TorConnectTopics: "moz-src:///toolkit/modules/TorConnect.sys.mjs",
  TorProviderBuilder:
    "moz-src:///toolkit/components/tor-launcher/TorProviderBuilder.sys.mjs",
  TorProviderState:
    "moz-src:///toolkit/components/tor-launcher/TorProviderBuilder.sys.mjs",
  TorProviderTopics:
    "moz-src:///toolkit/components/tor-launcher/TorProviderBuilder.sys.mjs",
  TorSettings: "moz-src:///toolkit/modules/TorSettings.sys.mjs",
  TorSettingsTopics: "moz-src:///toolkit/modules/TorSettings.sys.mjs",
});

// TODO: Change to GetLoxBridges if Lox enabled, and the account is set up.
const TELEGRAM_USER_NAME = "GetBridgesBot";
const TELEGRAM_HREF = `https://t.me/${TELEGRAM_USER_NAME}`;

const TOR_BRIDGES_URL_NAME = "bridges.torproject.org";
const TOR_BRIDGES_HREF = "https://bridges.torproject.org";

const TOR_BRIDGES_EMAIL = "bridges@torproject.org";

SettingGroupManager.registerGroups({
  connectionStatus: {
    inProgress: true,
    l10nId: "tor-connection-internet-status-group",
    supportPage: "tor-manual:getting-started__about-tor-browser",
    headingLevel: 2,
    items: [
      {
        id: "connectionStatusGroup",
        control: "moz-box-group",
        items: [
          {
            id: "internetStatus",
            control: "tor-connection-status",
            controlAttrs: { "status-type": "internet" },
          },
          {
            id: "torStatus",
            control: "tor-connection-status",
            controlAttrs: { "status-type": "tor" },
          },
        ],
      },
      {
        id: "connectAutomatically",
        l10nId: "tor-connection-quickstart-checkbox",
        control: "moz-toggle",
      },
    ],
  },
  torBridges: {
    inProgress: true,
    l10nId: "tor-bridges-group",
    supportPage: "tor-manual:bridges",
    headingLevel: 2,
    controlAttrs: { "focusable-heading": true },
    items: [
      {
        id: "bridgesEnabled",
        l10nId: "tor-bridges-use-bridges",
        control: "moz-toggle",
      },
      {
        id: "torBridgesDisplay",
        control: "tor-bridges-display",
      },
      {
        id: "newBridgesGroup",
        control: "moz-fieldset",
        controlAttrs: {
          headinglevel: 3,
        },
        options: [
          {
            control: "moz-box-group",
            items: [
              {
                id: "builtinBridges",
                l10nId: "tor-bridges-choose-built-in-button",
                control: "moz-box-button",
              },
              {
                id: "userProvidedBridges",
                l10nId: "tor-bridges-enter-bridges-button",
                control: "moz-box-button",
              },
            ],
          },
        ],
      },
      {
        id: "findMoreBridgesGroup",
        l10nId: "tor-bridges-find-more-group",
        control: "moz-fieldset",
        controlAttrs: {
          headinglevel: 3,
        },
        options: [
          {
            control: "moz-box-item",
            options: [
              {
                id: "torBridgesRequestBanner",
                control: "article",
                options: [
                  {
                    control: "img",
                    controlAttrs: {
                      alt: "",
                      src: "chrome://browser/content/torpreferences/bridge-bot.svg",
                    },
                  },
                  {
                    control: "p",
                    l10nId: "tor-bridges-request-from-browser2",
                  },
                  {
                    // NOTE: We use the wrapping `div` to simply switch from the
                    // `options` context to the `items` context, with the latter
                    // wrapping the elements in a setting-control.
                    control: "div",
                    items: [
                      {
                        id: "requestBridges",
                        l10nId: "tor-bridges-request-button2",
                        control: "moz-button",
                      },
                    ],
                  },
                ],
              },
            ],
          },
          {
            control: "moz-box-group",
            options: [
              {
                l10nId: "tor-bridges-source-telegram-link",
                l10nArgs: { telegramUserName: TELEGRAM_USER_NAME },
                control: "moz-box-link",
                iconSrc:
                  "chrome://browser/content/torpreferences/telegram-logo.svg",
                controlAttrs: {
                  href: TELEGRAM_HREF,
                },
              },
              {
                l10nId: "tor-bridges-source-web-link",
                l10nArgs: { url: TOR_BRIDGES_URL_NAME },
                control: "moz-box-link",
                iconSrc: "chrome://browser/content/torconnect/network.svg",
                controlAttrs: {
                  href: TOR_BRIDGES_HREF,
                },
              },
              {
                l10nId: "tor-bridges-source-email-link",
                l10nArgs: { address: TOR_BRIDGES_EMAIL },
                control: "moz-box-item",
                iconSrc: "chrome://browser/content/torpreferences/mail.svg",
              },
            ],
          },
        ],
      },
    ],
  },
});

Preferences.addSetting({
  id: "connectionStatusGroup",
});

Preferences.addSetting({
  id: "internetStatus",
  setup(emitChange) {
    Services.obs.addObserver(
      emitChange,
      lazy.TorConnectTopics.InternetStatusChange
    );
    return () => {
      Services.obs.removeObserver(
        emitChange,
        lazy.TorConnectTopics.InternetStatusChange
      );
    };
  },
  get() {
    switch (lazy.TorConnect.internetStatus) {
      case lazy.InternetStatus.Online:
        return "online";
      case lazy.InternetStatus.Offline:
        return "offline";
    }
    return "unknown";
  },
});

Preferences.addSetting({
  id: "torStatus",
  setup(emitChange) {
    Services.obs.addObserver(emitChange, lazy.TorConnectTopics.StageChange);
    return () => {
      Services.obs.removeObserver(
        emitChange,
        lazy.TorConnectTopics.StageChange
      );
    };
  },
  get() {
    if (lazy.TorConnect.stageName === lazy.TorConnectStage.Bootstrapped) {
      return "connected";
    }
    if (lazy.TorConnect.potentiallyBlocked) {
      return "potentially-blocked";
    }
    return "not-connected";
  },
});

Preferences.addSetting({
  id: "connectAutomatically",
  setup(emitChange) {
    Services.obs.addObserver(
      emitChange,
      lazy.TorConnectTopics.QuickstartChange
    );
    return () => {
      Services.obs.removeObserver(
        emitChange,
        lazy.TorConnectTopics.QuickstartChange
      );
    };
  },
  get() {
    return lazy.TorConnect.quickstart;
  },
  set(val) {
    lazy.TorConnect.quickstart = val;
  },
});

Preferences.addSetting({
  id: "torSettingsReady",
  _ready: false,
  setup(emitChange) {
    if (!lazy.TorSettings.enabled) {
      // Remain in `false`.
      return;
    }
    // Most likely, TorSettings will already be initialised.
    if (lazy.TorSettings.initialized) {
      this._ready = true;
      return;
    }
    // Else, wait for it to be initialised.
    lazy.TorSettings.initializedPromise.then(
      () => {
        this._ready = true;
        emitChange();
      },
      error => {
        // No change in state.
        console.error("TorSettings failed to initialize.", error);
      }
    );
  },
  get() {
    return this._ready;
  },
});

Preferences.addSetting({
  id: "torBridgesRaw",
  deps: ["torSettingsReady"],
  _value: null,
  setup(emitChange) {
    const observer = subject => {
      const { changes } = subject.wrappedJSObject;
      // NOTE: We do not include "bridges.lox_id" in the changes. Instead, any
      // widgets should wait for LoxTopics.UpdateActiveLoxId to ensure that the
      // Lox module has responded to the change in ID strictly *before* we do.
      // In particular, we want to make sure the invites and event data has been
      // cleared.
      if (
        changes.includes("bridges.source") ||
        changes.includes("bridges.bridge_strings") ||
        changes.includes("bridges.builtin_type")
      ) {
        // Reset.
        this._value = null;
        emitChange();
      }
    };
    Services.obs.addObserver(observer, lazy.TorSettingsTopics.SettingsChanged);
    return () => {
      Services.obs.removeObserver(
        observer,
        lazy.TorSettingsTopics.SettingsChanged
      );
    };
  },
  get(_pref, { torSettingsReady }) {
    if (this._value === null) {
      if (!torSettingsReady.value) {
        // TorSettings getter will throw.
        return null;
      }
      const source = lazy.TorSettings.bridges.source;
      // Cache a value.
      this._value = {
        haveBridges: source !== lazy.TorBridgeSource.Invalid,
        source,
        builtinType: lazy.TorSettings.bridges.builtin_type,
        bridgeStrings: lazy.TorSettings.bridges.bridge_strings,
      };
    }
    return this._value;
  },
});

Preferences.addSetting({
  id: "connectedBridgeId",
  _value: null,
  setup(emitChange) {
    const observer = async () => {
      // NOTE: It should be safe for this method to be called concurrently.
      let bridge = null;
      try {
        if (
          lazy.TorProviderBuilder.currentState() ===
          lazy.TorProviderState.Running
        ) {
          bridge = (await lazy.TorProviderBuilder.build()).currentBridge;
        }
        // Else, bridge is `null` whilst the provider is not running.
      } catch (e) {
        console.warn("Could not get current bridge", e);
      }
      const prevVal = this._value;
      this._value = bridge?.fingerprint ?? null;
      if (prevVal !== this._value) {
        emitChange();
      }
    };
    Services.obs.addObserver(observer, lazy.TorProviderTopics.BridgeChanged);
    // NOTE: BridgeChanged is only fired directly by the provider instances,
    // rather than by TorProviderBuilder. In particular, it will not fire when
    // the previous provider had a bridge and the new one does not, because
    // neither provider saw a change in their own bridge. But from the user's
    // point of view, the overall change would mean that the current bridge has
    // changed.
    // Moreover, we want to show no connected bridge whilst we are missing a
    // provider. Therefore, we also need to listen for a change in provider and
    // its state.
    // TODO: Maybe this logic should be moved to TorProviderBuilder itself if it
    // is ever needed by other parts of the UI.
    Services.obs.addObserver(
      observer,
      lazy.TorProviderTopics.ProviderStateChanged
    );
    // Get the initial value.
    observer();

    return () => {
      Services.obs.removeObserver(
        observer,
        lazy.TorProviderTopics.ProviderStateChanged
      );
      Services.obs.removeObserver(
        observer,
        lazy.TorProviderTopics.BridgeChanged
      );
    };
  },
  get() {
    return this._value;
  },
});

Preferences.addSetting({
  id: "bridgesEnabled",
  deps: ["torSettingsReady", "torBridgesRaw"],
  setup(emitChange) {
    const observer = subject => {
      const { changes } = subject.wrappedJSObject;
      if (changes.includes("bridges.enabled")) {
        emitChange();
      }
    };
    Services.obs.addObserver(observer, lazy.TorSettingsTopics.SettingsChanged);
    return () => {
      Services.obs.removeObserver(
        observer,
        lazy.TorSettingsTopics.SettingsChanged
      );
    };
  },
  get(_prefVal, { torSettingsReady }) {
    if (!torSettingsReady.value) {
      // TorSettings.bridges will throw before TorSettings has finished
      // initialisation.
      return false;
    }
    return lazy.TorSettings.bridges.enabled;
  },
  set(val) {
    lazy.TorSettings.changeSettings({
      bridges: { enabled: val },
    });
  },
  visible({ torSettingsReady }) {
    return torSettingsReady.value;
  },
  disabled({ torBridgesRaw }) {
    return !torBridgesRaw.value?.haveBridges;
  },
});

Preferences.addSetting({
  id: "torBridgesDisplay",
  deps: ["torSettingsReady", "torBridgesRaw", "connectedBridgeId"],
  getControlConfig(config, { torBridgesRaw, connectedBridgeId }) {
    config.controlAttrs = {
      ...config.controlAttrs,
      // Set the `bridges` and `connnectedBridgeId` object *properties* (rather
      // than attributes) by using the `.` prefix.
      ".bridges": torBridgesRaw.value,
      ".connectedBridgeId": connectedBridgeId.value,
    };
    return config;
  },
  visible({ torSettingsReady }) {
    return torSettingsReady.value;
  },
});

Preferences.addSetting({
  id: "newBridgesGroup",
  deps: ["torSettingsReady", "torBridgesRaw"],
  getControlConfig(config, { torBridgesRaw }) {
    config.l10nId = torBridgesRaw.value?.haveBridges
      ? "tor-bridges-replace-bridges-group"
      : "tor-bridges-add-bridges-group";
    return config;
  },
  visible({ torSettingsReady }) {
    return torSettingsReady.value;
  },
});

Preferences.addSetting({
  id: "builtinBridges",
  onUserClick() {
    lazy.openBridgeDialog(
      window,
      "chrome://browser/content/torpreferences/builtinBridgeDialog.xhtml",
      null,
      result => {
        if (!result.type) {
          return null;
        }
        return lazy.TorSettings.changeSettings({
          bridges: {
            enabled: true,
            source: lazy.TorBridgeSource.BuiltIn,
            builtin_type: result.type,
          },
        });
      }
    );
  },
});

Preferences.addSetting({
  id: "userProvidedBridges",
  deps: ["torBridgesRaw"],
  onUserClick(_event, { torBridgesRaw }) {
    lazy.openUserProvideBridgeDialog(
      window,
      torBridgesRaw.value?.haveBridges ? "replace" : "add"
    );
  },
});

Preferences.addSetting({
  id: "findMoreBridgesGroup",
  deps: ["torSettingsReady"],
  visible({ torSettingsReady }) {
    return torSettingsReady.value;
  },
});

Preferences.addSetting({
  id: "requestBridges",
  onUserClick() {
    lazy.openBridgeDialog(
      window,
      "chrome://browser/content/torpreferences/requestBridgeDialog.xhtml",
      null,
      result => {
        if (!result.bridges?.length) {
          return null;
        }
        return lazy.TorSettings.changeSettings({
          bridges: {
            enabled: true,
            source: lazy.TorBridgeSource.BridgeDB,
            bridge_strings: result.bridges,
          },
        });
      }
    );
  },
});
