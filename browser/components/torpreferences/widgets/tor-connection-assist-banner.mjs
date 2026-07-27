import { MozLitElement } from "chrome://global/content/lit-utils.mjs";
import { html } from "chrome://global/content/vendor/lit.all.mjs";

const lazy = {};
ChromeUtils.defineESModuleGetters(lazy, {
  TorConnect: "moz-src:///toolkit/modules/TorConnect.sys.mjs",
  TorConnectParent:
    "moz-src:///browser/components/torconnect/TorConnectParent.sys.mjs",
});

const TOR_CONNECT_HREF = "about:torconnect";

/**
 * Widget for displaying a Connection Assist banner.
 *
 * @tagname tor-connection-status
 */
class TorConnectionAssistBanner extends MozLitElement {
  render() {
    return html`
      <moz-message-bar
        role="complementary"
        type="warning"
        @click=${this.#handleClick}
      >
        <span
          slot="message"
          data-l10n-id="tor-bridges-connection-assist-message"
        >
          <a
            id="link"
            data-l10n-name="link"
            href=${TOR_CONNECT_HREF}
            target="_blank"
          ></a>
        </span>
      </moz-message-bar>
    `;
  }

  #handleClick(event) {
    if (!this.shadowRoot.getElementById("link")?.contains(event.target)) {
      return;
    }
    event.preventDefault();
    if (!lazy.TorConnect.inConnectionAssistStage) {
      // Switch to the "ChooseRegion" stage to reflect "Connection Assist".
      lazy.TorConnect.chooseRegion();
    }
    lazy.TorConnectParent.open();
  }
}
customElements.define(
  "tor-connection-assist-banner",
  TorConnectionAssistBanner
);
