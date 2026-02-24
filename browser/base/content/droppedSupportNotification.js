"use strict";

// Show a prompt that a user's system will no longer be supported.
window.addEventListener("load", () => {
  let labelId;

  if (
    AppConstants.platform === "macosx" &&
    Services.vc.compare(
      Services.sysinfo.getProperty("version"),
      "19.0" // MacOS 10.15 begins with Darwin 19.0
    ) < 0
  ) {
    labelId =
      "dropped-support-notification-macos-version-less-than-10-15-expired";
  } else if (
    AppConstants.platform === "win" &&
    Services.vc.compare(Services.sysinfo.getProperty("version"), "10.0") < 0
  ) {
    labelId =
      "dropped-support-notification-win-os-version-less-than-10-expired";
  }

  const dismissedPref =
    "browser.dropped_support_notification_v14.dismiss_version";

  if (!labelId) {
    // User has moved the application to a newer version. They should get an
    // update beyond 13.5.
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
