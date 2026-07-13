const lazy = {};
ChromeUtils.defineESModuleGetters(lazy, {
  TorBridgeSource: "moz-src:///toolkit/modules/TorSettings.sys.mjs",
  TorConnectParent:
    "moz-src:///browser/components/torconnect/TorConnectParent.sys.mjs",
  TorSettings: "moz-src:///toolkit/modules/TorSettings.sys.mjs",
});

/**
 * Force the focus to move to the bridge heading.
 *
 * @param {Window} win - The preferences window.
 * @param {boolean} [forceTopHeading=false] - Force the focus to move to the
 *   top "Bridges" setting heading.
 */
export function moveFocusToBridgeHeading(win, forceTopHeading = false) {
  // Move focus to the start of the relevant section, which is a heading.
  // They have tabindex="-1" so should be focusable, even though they are not
  // part of the usual tab navigation.
  // TODO: It might be better if we could use the # named anchor to
  // re-orient the screen reader position instead of using tabIndex=-1, but
  // about:preferences currently uses the anchor for showing categories
  // only. See bugzilla bug 1799153.
  if (
    forceTopHeading ||
    !win.document.getElementById("torBridgesDisplay").focusHeading()
  ) {
    win.document
      .querySelector('setting-group[groupid="torBridges"] moz-fieldset')
      .focusHeading();
  }
}

/**
 * Open a bridge dialog that will change the users bridges.
 *
 * @param {Window} win - The preferences window.
 * @param {string} url - The url of the dialog to open.
 * @param {object?} inputData - The input data to send to the dialog window.
 * @param {Function} onAccept - The method to call if the bridge dialog was
 *   accepted by the user. This will be passed a "result" object containing
 *   data set by the dialog. This should return a promise that resolves once
 *   the bridge settings have been set, or null if the settings have not
 *   been applied.
 */
export function openBridgeDialog(win, url, inputData, onAccept) {
  const result = { accepted: false, connect: false };
  let savedSettings = null;
  win.gSubDialog.open(
    url,
    {
      features: "resizable=yes",
      closingCallback: () => {
        if (!result.accepted) {
          return;
        }
        savedSettings = onAccept(result);
        if (!savedSettings) {
          // No change in settings.
          return;
        }
        if (!result.connect) {
          // Do not open about:torconnect.
          return;
        }

        // Wait until the settings are applied before bootstrapping.
        // NOTE: Saving the settings should also cancel any existing bootstrap
        // attempt first. See tor-browser#41921.
        savedSettings.then(() => {
          // The bridge dialog button is "connect" when Tor is not
          // bootstrapped, so do the connect.

          // Start Bootstrapping, which should use the configured bridges.
          // NOTE: We do this regardless of any previous TorConnect Error.
          lazy.TorConnectParent.open({ beginBootstrapping: "hard" });
        });
      },
      // closedCallback should be called after gSubDialog has already
      // re-assigned focus back to the document.
      closedCallback: () => {
        if (!savedSettings) {
          return;
        }
        // Wait until the settings have changed, so that the UI could
        // respond, then move focus.
        savedSettings.then(() => {
          moveFocusToBridgeHeading(win);
        });
      },
    },
    result,
    inputData
  );
}

/**
 * Open the user provide dialog.
 *
 * @param {Window} win - The preferences window.
 * @param {string} mode - The mode to open the dialog in: "add", "replace" or
 *   "edit".
 */
export function openUserProvideBridgeDialog(win, mode) {
  openBridgeDialog(
    win,
    "chrome://browser/content/torpreferences/provideBridgeDialog.xhtml",
    { mode },
    result => {
      const loxId = result.loxId;
      if (!loxId && !result.addresses?.length) {
        return null;
      }
      const bridges = { enabled: true };
      if (loxId) {
        bridges.source = lazy.TorBridgeSource.Lox;
        bridges.lox_id = loxId;
      } else {
        bridges.source = lazy.TorBridgeSource.UserProvided;
        bridges.bridge_strings = result.addresses;
      }
      return lazy.TorSettings.changeSettings({ bridges });
    }
  );
}
