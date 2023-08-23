/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at htp://mozilla.org/MPL/2.0/. */

import { html, ifDefined } from "../vendor/lit.all.mjs";
import { MozBaseInputElement } from "../lit-utils.mjs";
// eslint-disable-next-line import/no-unassigned-import
import "chrome://global/content/elements/moz-label.mjs";

/**
 * A simple toggle element that can be used to switch between two states.
 *
 * @tagname moz-toggle
 * @property {boolean} pressed - Whether or not the element is pressed.
 * @property {boolean} disabled - Whether or not the element is disabled.
 * @property {string} label - The label text.
 * @property {string} description - The description text.
 * @property {string} ariaLabel - The aria-label text when there is no visible label.
 * @property {string} ariaDescription - The aria-description text when there is no visible description.
 * @slot support-link - Used to append a moz-support-link to the description.
 * @fires toggle
 *  Custom event indicating that the toggle's pressed state has changed.
 */
export default class MozToggle extends MozBaseInputElement {
  static properties = {
    pressed: { type: Boolean, reflect: true },
    // Extension for tor-browser. Used for tor-browser#41333.
    title: { type: String, attribute: "title" },
  };

  static activatedProperty = "pressed";

  get buttonEl() {
    return this.inputEl;
  }

  constructor() {
    super();
    this.pressed = false;
  }

  handleClick() {
    this.pressed = !this.pressed;
    this.dispatchOnUpdateComplete(
      new CustomEvent("toggle", {
        bubbles: true,
        composed: true,
      })
    );
  }

  inputTemplate() {
    const { pressed, disabled, ariaLabel, handleClick } = this;
    let ariaDescription = undefined;
    if (!this.hasDescription) {
      ariaDescription = this.ariaDescription;
      if (
        !ariaDescription &&
        this.title &&
        this.title !== (ariaLabel || this.label)
      ) {
        // For tor-browser, if we have a title we use it as the
        // aria-description. Used for tor-browser#41333.
        // Only set the description using the title if it differs from the
        // accessible name derived from the label (ariaLabel || this.label).
        ariaDescription = this.title;
      }
    }

    return html`<button
      id="input"
      part="button"
      type="button"
      class="toggle-button"
      name=${this.name}
      value=${this.value}
      ?disabled=${disabled}
      aria-pressed=${pressed}
      aria-label=${ifDefined(ariaLabel ?? undefined)}
      aria-describedby="description"
      aria-description=${ifDefined(ariaDescription)}
      accesskey=${ifDefined(this.accessKey)}
      @click=${handleClick}
    ></button>`;
  }

  inputStylesTemplate() {
    return html`<link
      rel="stylesheet"
      href="chrome://global/content/elements/moz-toggle.css"
    />`;
  }
}
customElements.define("moz-toggle", MozToggle);
