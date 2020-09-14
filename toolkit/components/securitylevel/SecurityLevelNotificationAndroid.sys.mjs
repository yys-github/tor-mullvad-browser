const lazy = {};

ChromeUtils.defineESModuleGetters(lazy, {
  EventDispatcher: "resource://gre/modules/Messaging.sys.mjs",
  SecurityLevelPrefs:
    "moz-src:///toolkit/components/securitylevel/SecurityLevel.sys.mjs",
});

const logger = console.createInstance({
  maxLogLevel: "Info",
  prefix: "SecurityLevelNotificationAndroid",
});

const EmittedEvents = Object.freeze({
  tryRestartBrowser: "GeckoView:Tor:TryRestartBrowser",
  securityLevelCustom: "GeckoView:Tor:SecurityLevelCustom",
});

/**
 * Mostly copied from browser/modules/SecurityLevelNotification.sys.mjs
 * Interface for showing the security level notifications on Android.
 */
export const SecurityLevelNotificationAndroid = {
  /**
   * Whether we have already been initialised
   *
   * @type {boolean}
   */
  _initialized: false,

  /**
   * Called when the UI is ready to show a notification.
   */
  ready() {
    logger.info("ready() called for SecurityLevelNotificationAndroid");
    if (this._initialized) {
      return;
    }
    this._initialized = true;
    lazy.SecurityLevelPrefs.setNotificationHandler(this);
  },

  /**
   * Send restart notification to the android layer
   *
   * @returns {boolean} - Whether we are restarting the browser.
   */
  async tryRestartBrowser() {
    // TODO: maybe implement. See tor-browser#45028.
    logger.info("tryRestartBrowser() called, but not yet implemented.");
    return false;
  },

  /**
   * Show or re-show the custom security notification.
   *
   * @param {Function} userDismissedCallback - The callback for when the user
   *   dismisses the notification.
   */
  async showCustomWarning(userDismissedCallback) {
    logger.info("showCustomWarning() called");
    let result = await lazy.EventDispatcher.instance.sendRequestForResult(
      EmittedEvents.securityLevelCustom,
      { isCustom: true }
    );
    logger.info(
      `result.userDismissedCustomWarning is ${result.userDismissedCustomWarning}`
    );
    if (result.userDismissedCustomWarning) {
      logger.info("userDismissedCallback() called");
      userDismissedCallback();
    }
  },
};
