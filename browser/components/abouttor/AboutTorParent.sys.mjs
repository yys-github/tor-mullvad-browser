import { AppConstants } from "resource://gre/modules/AppConstants.sys.mjs";

const lazy = {};

ChromeUtils.defineESModuleGetters(lazy, {
  AboutTorMessage:
    "moz-src:///browser/components/abouttor/AboutTorMessage.sys.mjs",
  TorConnect: "moz-src:///toolkit/modules/TorConnect.sys.mjs",
});

const initializedActors = new Set();
const onionizePref = "torbrowser.homepage.search.onionize";
const surveyDismissVersionPref = "torbrowser.homepage.survey.dismiss_version";

/**
 * Actor parent class for the about:tor page.
 */
export class AboutTorParent extends JSWindowActorParent {
  /**
   * Whether the user has dismissed the Year End Campaign (YEC) banner this
   * session.
   *
   * @type {boolean}
   */
  static #dismissYEC = false;

  /**
   * A shuffled array of tool numbers. `null` whilst unset.
   *
   * @type {?number[]}
   */
  static #toolNums = null;
  /**
   * The current index in `#toolNums`.
   *
   * @type {number}
   */
  static #toolIndex = 0;

  /**
   * Return the pair of tools numbers to show in the next about:tor window.
   *
   * @returns {{ toolNum1: number, toolNum2: number }} - The tool number pairs.
   */
  static #getToolNumPair() {
    const numTools = 11;
    if (!this.#toolNums) {
      this.#toolNums = Array.from({ length: numTools }, (_, index) => index);
      // Shuffle the array with Fisher–Yates.
      for (let index = numTools - 1; index > 0; index--) {
        const topBound = index + 1;
        const exchange = Math.floor(Math.random() * topBound) % topBound;
        const tmpNum = this.#toolNums[index];
        this.#toolNums[index] = this.#toolNums[exchange];
        this.#toolNums[exchange] = tmpNum;
      }
    }
    const toolNum1 = this.#toolNums[this.#toolIndex % numTools];
    const toolNum2 = this.#toolNums[(this.#toolIndex + 1) % numTools];
    this.#toolIndex = (this.#toolIndex + 2) % numTools;

    return { toolNum1, toolNum2 };
  }

  /**
   * Whether this instance has a preloaded browser.
   *
   * @type {boolean}
   */
  #preloaded = false;

  /**
   * Method to be called when the browser corresponding to this actor has its
   * preloadedState attribute removed.
   */
  preloadedRemoved() {
    if (!this.#preloaded) {
      return;
    }
    this.#preloaded = false;
    // Send in the initial data now that the page is actually going to be
    // visible.
    this.sendAsyncMessage(
      "AboutTor:DelayedInitialData",
      this.#getInitialData()
    );
  }

  /**
   * Get the initial data for the page when it is about to be shown.
   *
   * @returns {object} - The initial data.
   */
  #getInitialData() {
    let appLocale = Services.locale.appLocaleAsBCP47;
    if (appLocale === "ja-JP-macos") {
      appLocale = "ja";
    }

    const { toolNum1, toolNum2 } = AboutTorParent.#getToolNumPair();

    return {
      torConnectEnabled: lazy.TorConnect.enabled,
      messageData: lazy.AboutTorMessage.getNext(),
      isStable: AppConstants.MOZ_UPDATE_CHANNEL === "release",
      searchOnionize: Services.prefs.getBoolPref(onionizePref, false),
      surveyDismissVersion: Services.prefs.getIntPref(
        surveyDismissVersionPref,
        0
      ),
      appLocale,
      dismissYEC: AboutTorParent.#dismissYEC,
      toolNum1,
      toolNum2,
    };
  }

  didDestroy() {
    initializedActors.delete(this);
  }

  receiveMessage(message) {
    switch (message.name) {
      case "AboutTor:GetInitialData": {
        // Track this actor to send future updates.
        initializedActors.add(this);

        const browser = this.browsingContext.top.embedderElement;
        if (browser?.getAttribute("preloadedState") === "preloaded") {
          // Wait until the page is actually about to be shown before sending
          // the initial data.
          // Otherwise the preloaded page might receive data that has expired by
          // the time the page is shown. And it will iterate
          // AboutTorMessage.getNext too early. See tor-browser#44314.
          this.#preloaded = true;
          return Promise.resolve(null);
        }
        return Promise.resolve(this.#getInitialData());
      }
      case "AboutTor:SetSearchOnionize":
        Services.prefs.setBoolPref(onionizePref, message.data);
        break;
      case "AboutTor:SurveyDismissed":
        // The message.data contains the version of the current survey.
        // Rather than introduce a new preference for each survey campaign we
        // reuse the same integer preference and increase its value every time
        // a new version of the survey is shown and dismissed by the user.
        // I.e. if the preference value is 2, we will not show survey version 2
        // but will show survey version 3 or higher when they are introduced.
        // It should be safe to overwrite the value since we do not expect more
        // than one active survey campaign at any given time, nor do we expect
        // the version value to decrease.
        Services.prefs.setIntPref(surveyDismissVersionPref, message.data);
        break;
      case "AboutTor:UserDismissedYEC":
        AboutTorParent.#dismissYEC = true;
        for (const actor of initializedActors) {
          if (actor === this) {
            // Don't send to ourselves.
            continue;
          }
          // Tell all existing instances to also close the banner, if they still
          // exist.
          // NOTE: If the user's new tab page is `about:tor`, this may include
          // some preloaded pages that have not been made visible yet (see
          // NewTabPagePreloading).
          try {
            actor.sendAsyncMessage("AboutTor:DismissYEC");
          } catch {}
        }
        break;
    }
    return undefined;
  }
}
