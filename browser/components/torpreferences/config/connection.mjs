import { SettingGroupManager } from "chrome://browser/content/preferences/config/SettingGroupManager.mjs";
import { Preferences } from "chrome://global/content/preferences/Preferences.mjs";

const lazy = {};
ChromeUtils.defineESModuleGetters(lazy, {
  InternetStatus: "moz-src:///toolkit/modules/TorConnect.sys.mjs",
  TorConnect: "moz-src:///toolkit/modules/TorConnect.sys.mjs",
  TorConnectStage: "moz-src:///toolkit/modules/TorConnect.sys.mjs",
  TorConnectTopics: "moz-src:///toolkit/modules/TorConnect.sys.mjs",
});

SettingGroupManager.registerGroups({
  connectionStatus: {
    inProgress: true,
    l10nId: "tor-connection-internet-status-group",
    supportPage: "tor-manual:about",
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
