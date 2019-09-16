import { MozBoxBase } from "chrome://global/content/lit-utils.mjs";
import {
  classMap,
  html,
  ifDefined,
} from "chrome://global/content/vendor/lit.all.mjs";

const lazy = {};
ChromeUtils.defineESModuleGetters(lazy, {
  TorConnectParent:
    "moz-src:///browser/components/torconnect/TorConnectParent.sys.mjs",
});

/**
 * Widget for displaying the current internet or Tor connection status.
 *
 * @tagname tor-connection-status
 * @property {"internet" | "tor"} statusType - The type of status this widget is
 *   showing.
 * @property {string} value - The current status value.
 */
class TorConnectionStatus extends MozBoxBase {
  static properties = {
    statusType: { type: String, attribute: "status-type" },
    value: { type: String },
    _nameL10nId: { type: String },
    _valueL10nId: { type: String },
    _iconWarning: { type: Boolean },
    _connectButton: { type: Boolean },
  };

  /**
   * A map from the status type to it's configuration. Each configuration is
   * used to set the internal properties depending on the current value.
   *
   * @type {{[key: string]: object}}
   */
  static #config = {
    internet: {
      nameL10nId: "tor-connection-internet-status-label",
      values: {
        online: {
          iconSrc: "chrome://browser/content/torconnect/network.svg",
          valueL10nId: "tor-connection-internet-status-online",
        },
        offline: {
          iconSrc: "chrome://browser/content/torconnect/network-broken.svg",
          valueL10nId: "tor-connection-internet-status-offline",
        },
        unknown: {
          iconSrc: "chrome://browser/content/torconnect/network.svg",
          valueL10nId: "tor-connection-internet-status-unknown",
        },
      },
    },
    tor: {
      nameL10nId: "tor-connection-network-status-label",
      values: {
        connected: {
          iconSrc: "chrome://browser/content/torconnect/tor-connect.svg",
          valueL10nId: "tor-connection-network-status-connected",
        },
        "not-connected": {
          iconSrc: "chrome://browser/content/torconnect/tor-connect-broken.svg",
          valueL10nId: "tor-connection-network-status-not-connected",
          connectButton: true,
        },
        "potentially-blocked": {
          iconSrc: "chrome://browser/content/torconnect/tor-connect-broken.svg",
          valueL10nId: "tor-connection-network-status-blocked",
          iconWarning: true,
          connectButton: true,
        },
      },
    },
  };

  /**
   * Whether we had focus prior to an update.
   *
   * @type {boolean}
   */
  #hadFocus = false;

  willUpdate() {
    const config = TorConnectionStatus.#config[this.statusType];
    const valueConfig = config?.values[this.value];
    this._nameL10nId = config?.nameL10nId;
    this._valueL10nId = valueConfig?.valueL10nId;
    this.iconSrc = valueConfig?.iconSrc;
    this._iconWarning = valueConfig?.iconWarning ?? false;
    this._connectButton = valueConfig?.connectButton ?? false;
    this.#hadFocus = !!this.shadowRoot.activeElement;
  }

  updated() {
    if (this.#hadFocus && !this.shadowRoot.activeElement) {
      // Focus is lost. We move focus to the search input.
      window.gSearchResultsPane.searchInput.focus();
    }
    this.#hadFocus = false;
  }

  connectButtonClick() {
    lazy.TorConnectParent.open({ beginBootstrapping: "soft" });
  }

  connectButtonTemplate() {
    if (!this._connectButton) {
      return "";
    }
    return html`
      <moz-button
        data-l10n-id="tor-connection-status-connect-button"
        @click=${this.connectButtonClick}
      ></moz-button>
    `;
  }

  // override MozBoxBase.labelTemplate.
  labelTemplate() {
    if (!this._nameL10nId) {
      return "";
    }
    // NOTE: We purposefully include whitespace between the inner <span>
    // elements so their text is separated.
    return html`<div class="label tor-status-label">
      <p>
        <span
          class="tor-status-name"
          data-l10n-id=${this._nameL10nId}
        ></span>
        <span data-l10n-id=${ifDefined(this._valueL10nId)}></span>
      </p>
      ${this.connectButtonTemplate()}
    </span> `;
  }

  render() {
    if (!this._nameL10nId) {
      return "";
    }
    // NOTE: textTemplate comes from MozBoxBase, which will include an icon
    // followed by our label.
    return html`
      ${this.stylesTemplate()}
      <link
        rel="stylesheet"
        href="chrome://browser/content/torpreferences/widgets/tor-connection-status.css"
      />
      <div
        class=${classMap({
          "tor-status-container": true,
          "tor-status-icon-warning": this._iconWarning,
        })}
      >
        ${super.textTemplate()}
      </div>
    `;
  }
}
customElements.define("tor-connection-status", TorConnectionStatus);
