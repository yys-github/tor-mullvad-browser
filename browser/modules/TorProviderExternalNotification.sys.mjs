/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

const lazy = {};

ChromeUtils.defineESModuleGetters(lazy, {
  BrowserWindowTracker: "resource:///modules/BrowserWindowTracker.sys.mjs",
  TorConnect: "moz-src:///toolkit/modules/TorConnect.sys.mjs",
  TorProviderBuilder:
    "moz-src:///toolkit/components/tor-launcher/TorProviderBuilder.sys.mjs",
  TorProviderState:
    "moz-src:///toolkit/components/tor-launcher/TorProviderBuilder.sys.mjs",
  TorProviderTopics:
    "moz-src:///toolkit/components/tor-launcher/TorProviderBuilder.sys.mjs",
});

export const TorProviderExternalNotification = {
  /**
   * Whether we have been initialized.
   *
   * @type {boolean}
   */
  _initialized: false,

  /**
   * Callback for browser-window-delayed-startup.
   *
   * @param {Window} win - The browser window that was just loaded.
   */
  delayedStartup(win) {
    if (lazy.TorConnect.enabled) {
      // Let TorConnect handle the ProviderStateChanged.
      return;
    }

    if (!this._initialized) {
      this._initialized = true;
      this._setProviderState(lazy.TorProviderBuilder.currentState());
      Services.obs.addObserver(
        this,
        lazy.TorProviderTopics.ProviderStateChanged
      );
    } else {
      // Maybe show the notification in the new window.
      this._updateNotification(win);
    }
  },

  /**
   * Callback for browser-window-unload.
   *
   * @param {Window} win - The browser window that is being unloaded.
   */
  unload(win) {
    if (this._notification?.window === win) {
      this._notification = null;
    }
  },

  observe(subject, topic, data) {
    switch (topic) {
      case lazy.TorProviderTopics.ProviderStateChanged:
        this._setProviderState(data);
        break;
    }
  },

  /**
   * The last seen `TorProviderState` state.
   *
   * @type {?string}
   */
  _providerState: null,

  /**
   * Set the provider state.
   *
   * @param {string} state - The `TorProviderState` state we are now in.
   */
  _setProviderState(state) {
    if (this._providerState === state) {
      // No change, ignore.
      return;
    }

    this._providerState = state;
    // A new instance of a provider, so ignore any previously dismissed
    // notifications.
    this._notificationUserDismissed = false;
    this._updateNotification();
  },

  /**
   * The notification that is currently shown.
   *
   * @type {?{ window: Window, elementPromise: Promise<NotificationMessage> }}
   */
  _notification: null,
  /**
   * Whether the user has dismissed the notification for the current TorProvider
   * instance.
   *
   * @type {boolean}
   */
  _notificationUserDismissed: false,

  /**
   * Update the notification depending on the latest `TorProviderState`.
   *
   * @param {Window} [win] - A newly opened browser window to maybe show the
   *   notification in. If not given, the notification might be shown in the
   *   current top window.
   */
  _updateNotification(win) {
    if (this._providerState === lazy.TorProviderState.Starting) {
      this._updateNotificationButton(
        "tor-external-process-error-retry-button-retrying"
      );
      return;
    }

    if (this._providerState === lazy.TorProviderState.Running) {
      if (this._notification) {
        // If there is a notification, close it.
        const { elementPromise } = this._notification;
        this._notification = null;
        elementPromise.then(el => el.remove());
      }
      return;
    }
    // Else, _providerState is TorProviderState.Stopped.

    if (this._notificationUserDismissed) {
      // User has already dismissed this in a different window.
      return;
    }

    if (this._notification) {
      // Already showing.
      this._updateNotificationButton("tor-external-process-error-retry-button");
      return;
    }

    if (!win) {
      win = lazy.BrowserWindowTracker.getTopWindow();
      if (!win) {
        // Wait for a window.
        return;
      }
    }

    const buttons = [
      {
        supportPage: "tor-manual:get-in-touch__bug-or-feedback",
      },
      {
        "l10n-id": "tor-external-process-error-retry-button",
        callback: () => {
          if (this._providerState === lazy.TorProviderState.Stopped) {
            lazy.TorProviderBuilder.replace();
          }
          // Keep open.
          return true;
        },
      },
    ];

    const notification = {
      window: win,
      elementPromise: win.gNotificationBox.appendNotification(
        "tor-provider-external-process-stopped",
        {
          label: { "l10n-id": "tor-external-process-error-message" },
          priority: win.gNotificationBox.PRIORITY_WARNING_HIGH,
          eventCallback: event => {
            switch (event) {
              case "dismissed":
                this._notificationUserDismissed = true;
                break;
              case "disconnected":
                if (this._notification === notification) {
                  this._notification = null;
                }
                break;
            }
          },
        },
        buttons
      ),
    };
    this._notification = notification;
  },

  /**
   * Update the notification button if it exists.
   *
   * @param {string} l10nId - The Fluent ID for the string we want to show in
   *   the button.
   */
  async _updateNotificationButton(l10nId) {
    if (!this._notification) {
      return;
    }
    const state = this._providerState;
    const notification = this._notification;
    const notificationEl = await notification.elementPromise;

    if (this._providerState !== state || this._notification !== notification) {
      // Replaced.
      return;
    }

    notificationEl.buttonContainer.firstElementChild.setAttribute(
      "data-l10n-id",
      l10nId
    );
  },
};
