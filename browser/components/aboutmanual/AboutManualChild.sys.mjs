/**
 * Actor child class for the about:manual page.
 */
export class AboutManualChild extends JSWindowActorChild {
  handleEvent(event) {
    switch (event.type) {
      case "ManualLocaleSelected": {
        const locale = event.detail;
        if (typeof locale === "string") {
          this.sendAsyncMessage("AboutManual:LocaleSelected", locale);
        }
      }
    }
  }
}
