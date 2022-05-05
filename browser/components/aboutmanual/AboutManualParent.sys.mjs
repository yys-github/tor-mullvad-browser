/**
 * Actor parent class for the about:manual page.
 */
export class AboutManualParent extends JSWindowActorParent {
  receiveMessage(message) {
    switch (message.name) {
      case "AboutManual:LocaleSelected": {
        const locale = message.data;
        if (
          Services.prefs
            .getStringPref("torbrowser.manual.available-locales", "")
            .split(",")
            .includes(locale)
        ) {
          Services.prefs.setStringPref("torbrowser.manual.locale", locale);
          this.browsingContext.reload(Ci.nsIWebNavigation.LOAD_FLAGS_NONE);
        }
      }
    }
  }
}
