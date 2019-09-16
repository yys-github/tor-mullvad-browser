"use strict";

{
  /**
   * Element to display a single bridge emoji, with a localized name.
   */
  class BridgeEmoji extends HTMLElement {
    /**
     * The instances that are active (in the DOM).
     *
     * @type {Set<BridgeEmoji>}
     */
    static #activeInstances = new Set();

    /**
     * A promise that resolves with a list of emoji strings, in a specific
     * order. Each BridgeEmoji instance has an `#index` in this list, which
     * points to its corresponding emoji.
     *
     * @type {Promise<string[]>}
     */
    static #emojiListPromise = fetch(
      "chrome://browser/content/torpreferences/bridgemoji/bridge-emojis.json"
    ).then(response => response.json());

    /**
     * @typedef {{[key: string]: string}} EmojiAnnotations
     *
     * A map from an emoji's codepoint to a locale's annotation.
     */
    /**
     * A promise that resolves with a map from locales to their annotations.
     *
     * @type {Promise<{[key: string]: EmojiAnnotattions}>}
     */
    static #annotationsPromise = fetch(
      "chrome://browser/content/torpreferences/bridgemoji/annotations.json"
    ).then(response => response.json());

    /**
     * @typedef {object} EmojiLocaleDetails
     *
     * @property {string[]} [emojiList] - A list of emojis as strings, in a
     *   specific order.
     * @property {EmojiAnnotations} [annotations] - The annotations for the
     *   locale.
     * @property {string} [unknownString] - The string to use for emojis with
     *   undefined annotations.
     */
    /**
     * The cached locale details.
     *
     * @type {EmojiLocaleDetails}
     */
    static #emojiLocaleDetails = {};

    /**
     * A promise that resolves when the previous call to appLocalesChanged
     * completes.
     *
     * @type {Promise?}
     */
    static #prevAppLocalesChangedCall = null;

    /**
     * Update the locale used for bridge emoji widget.
     */
    static async appLocalesChanged() {
      // Introduce a queue to ensure calls apply in the order they are invoked,
      // so the last seen locale will eventually be the applied one.
      const { promise, resolve } = Promise.withResolvers();
      const prevAppLocalesChangedCall = this.#prevAppLocalesChangedCall;
      this.#prevAppLocalesChangedCall = promise;
      try {
        await prevAppLocalesChangedCall;
        await this.#appLocalesChangedInternal();
      } finally {
        resolve();
      }
    }

    static async #appLocalesChangedInternal() {
      let [emojiAnnotations, emojiList, unknownString] = await Promise.all([
        this.#annotationsPromise,
        this.#emojiListPromise,
        // Grab the string for the new locale.
        document.l10n.formatValue("tor-bridges-emoji-unknown"),
      ]);

      let langCode;
      // Find the first desired locale we have annotations for.
      // Add "en" as a fallback.
      for (const bcp47 of [...Services.locale.appLocalesAsBCP47, "en"]) {
        langCode = bcp47;
        if (langCode in emojiAnnotations) {
          break;
        }
        // Remove everything after the dash, if there is one.
        langCode = bcp47.replace(/-.*/, "");
        if (langCode in emojiAnnotations) {
          break;
        }
      }

      this.#emojiLocaleDetails = {
        emojiList,
        annotations: emojiAnnotations[langCode],
        unknownString,
      };
      for (const inst of this.#activeInstances) {
        inst.update();
      }
    }

    /**
     * Update the bridge emoji to show their corresponding emoji with an
     * annotation that matches the current locale.
     */
    update() {
      const { emojiList, annotations, unknownString } =
        BridgeEmoji.#emojiLocaleDetails;

      if (!this.#active || !emojiList || !annotations || !unknownString) {
        return;
      }

      const doc = this.ownerDocument;
      const emoji = emojiList[this.#index];
      let emojiName;
      if (!emoji) {
        // Unexpected.
        this.#img.removeAttribute("src");
      } else {
        const cp = emoji.codePointAt(0).toString(16);
        this.#img.setAttribute(
          "src",
          `chrome://browser/content/torpreferences/bridgemoji/svgs/${cp}.svg`
        );
        emojiName = annotations[cp];
      }
      if (!emojiName) {
        doc.defaultView.console.error(`No emoji for index ${this.#index}`);
        emojiName = unknownString;
      }
      doc.l10n.setAttributes(this.#img, "tor-bridges-emoji-image", {
        emojiName,
      });
    }

    /**
     * The index for this bridge emoji.
     *
     * @type {integer?}
     */
    #index = null;
    /**
     * Whether we are active (i.e. in the DOM).
     *
     * @type {boolean}
     */
    #active = false;
    /**
     * The image element.
     *
     * @type {HTMLImgElement?}
     */
    #img = null;

    constructor(index) {
      super();
      this.#index = index;
    }

    connectedCallback() {
      if (!this.#img) {
        this.#img = this.ownerDocument.createElement("img");
        this.#img.classList.add("tor-bridges-emoji-icon");
        this.#img.setAttribute("alt", "");
        this.appendChild(this.#img);
      }

      this.#active = true;
      BridgeEmoji.#activeInstances.add(this);
      this.update();
    }

    disconnectedCallback() {
      this.#active = false;
      BridgeEmoji.#activeInstances.delete(this);
    }

    /**
     * Create four bridge emojis for the given address.
     *
     * @param {string} bridgeLine - The bridge address.
     *
     * @returns {BridgeEmoji[4]} - The bridge emoji elements.
     */
    static createForAddress(bridgeLine) {
      // JS uses UTF-16. While most of these emojis are surrogate pairs, a few
      // ones fit one UTF-16 character. So we could not use neither indices,
      // nor substr, nor some function to split the string.
      // FNV-1a implementation that is compatible with other languages
      const prime = 0x01000193;
      const offset = 0x811c9dc5;
      let hash = offset;
      const encoder = new TextEncoder();
      for (const byte of encoder.encode(bridgeLine)) {
        hash = Math.imul(hash ^ byte, prime);
      }

      return [
        ((hash & 0x7f000000) >> 24) | (hash < 0 ? 0x80 : 0),
        (hash & 0x00ff0000) >> 16,
        (hash & 0x0000ff00) >> 8,
        hash & 0x000000ff,
      ].map(index => new BridgeEmoji(index));
    }
  }

  customElements.define("tor-bridge-emoji", BridgeEmoji);

  {
    const appLocalesChanged = BridgeEmoji.appLocalesChanged.bind(BridgeEmoji);
    Services.obs.addObserver(appLocalesChanged, "intl:app-locales-changed");
    window.addEventListener(
      "unload",
      () => {
        Services.obs.removeObserver(
          appLocalesChanged,
          "intl:app-locales-changed"
        );
      },
      { once: true }
    );
    appLocalesChanged();
  }
}
