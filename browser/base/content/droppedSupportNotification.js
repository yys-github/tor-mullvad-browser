"use strict";

// Show a prompt that a user's system will no longer be supported.
window.addEventListener("load", () => {
  let labelId;

  if (
    AppConstants.platform === "linux" &&
    Services.appinfo.XPCOMABI === "x86-gcc3"
  ) {
    labelId = "dropped-support-notification-linux-32-bit";
    // TODO: switch to the expired label for the final version 15 release.
    // labelId = "dropped-support-notification-linux-32-bit-expired";
  }

  const dismissedPref =
    "browser.dropped_support_notification_v16.dismiss_version";

  if (!labelId) {
    // Avoid setting any preferences for supported versions, and clean up any
    // old values if the user ported their profile.
    Services.prefs.clearUserPref(dismissedPref);
    return;
  }

  if (
    Services.prefs.getStringPref(dismissedPref, "") ===
    AppConstants.BASE_BROWSER_VERSION
  ) {
    // Already dismissed since the last update.
    return;
  }

  let locale = Services.locale.appLocaleAsBCP47;
  if (locale === "ja-JP-macos") {
    // Convert quirk-locale to the locale used for tor project.
    locale = "ja";
  }
  // NOTE: The support page only covers a subset of locales. But they should
  // redirect to the default en-US page if the locale is not supported.
  // Locales that have support pages.
  // NOTE: /es-ES/ will redirect to /es/.
  const link = `https://support.torproject.org/${locale}/tor-browser/security/legacy-os/`;

  const buttons = [
    {
      "l10n-id": "notification-learnmore-default-label",
      link,
    },
    {
      "l10n-id": "dropped-support-notification-dismiss-button",
      callback: () => {
        Services.prefs.setStringPref(
          dismissedPref,
          AppConstants.BASE_BROWSER_VERSION
        );
      },
    },
  ];

  gNotificationBox.appendNotification(
    "dropped-support-notification",
    {
      label: { "l10n-id": labelId },
      priority: gNotificationBox.PRIORITY_WARNING_HIGH,
    },
    buttons
  );
});
