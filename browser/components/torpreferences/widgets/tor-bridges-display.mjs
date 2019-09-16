const lazy = {};
ChromeUtils.defineESModuleGetters(lazy, {
  Lox: "moz-src:///toolkit/components/lox/Lox.sys.mjs",
  LoxTopics: "moz-src:///toolkit/components/lox/Lox.sys.mjs",
  moveFocusToBridgeHeading:
    "chrome://browser/content/torpreferences/config/helpers.mjs",
  openUserProvideBridgeDialog:
    "chrome://browser/content/torpreferences/config/helpers.mjs",
  TorBridgeSource: "moz-src:///toolkit/modules/TorSettings.sys.mjs",
  TorParsers: "moz-src:///toolkit/components/tor-launcher/TorParsers.sys.mjs",
  TorSettings: "moz-src:///toolkit/modules/TorSettings.sys.mjs",
});

/**
 * Show the bridge QR to the user.
 *
 * @param {string} bridgeString - The string to use in the QR.
 */
function showBridgeQr(bridgeString) {
  window.gSubDialog.open(
    "chrome://browser/content/torpreferences/bridgeQrDialog.xhtml",
    { features: "resizable=yes" },
    bridgeString
  );
}

/**
 * Post a new notification, replacing any existing one.
 *
 * @param {string} type - The notification type.
 */
async function postBridgeNotification(type) {
  let updateId;
  switch (type) {
    case "removed-one":
      updateId = "tor-bridges-update-removed-one-bridge";
      break;
    case "removed-all":
      updateId = "tor-bridges-update-removed-all-bridges";
      break;
    case "changed":
    default:
      // Generic message for when bridges change.
      updateId = "tor-bridges-update-changed-bridges";
      break;
  }
  const bridgeDisplay = document.querySelector("tor-bridges-display");
  const settingGroup = bridgeDisplay?.closest("setting-group");
  if (!settingGroup) {
    console.error("Missing a setting-group for a notification.");
    return;
  }
  if (!settingGroup.checkVisibility()) {
    // Only ping the user if the bridge settings are visible.
    // NOTE: Most operations to change the bridges will occur within the
    // connection settings. However, in principle the user could have multiple
    // setting tabs open, or they may have the settings open whilst Connection
    // Assist is setting their bridges.
    return;
  }
  const [message] = await Promise.all([
    document.l10n.formatValue(updateId),
    // Wait at least a small amount of time to actually trigger ariaNotify.
    // Otherwise Orca will ignore the notification when it almost coincides with
    // a change in focus, which is normally the case.
    new Promise(resolve => setTimeout(resolve, 500)),
  ]);
  bridgeDisplay.ariaNotify(message);
}

/**
 * Controls the bridge grid.
 */
const gBridgeGrid = {
  /**
   * The grid element.
   *
   * @type {Element?}
   */
  _grid: null,
  /**
   * The template for creating new rows.
   *
   * @type {HTMLTemplateElement?}
   */
  _rowTemplate: null,

  /**
   * @typedef {object} BridgeGridRow
   *
   * @property {Element} element - The row element.
   * @property {Element} optionsButton - The options button.
   * @property {Element} menu - The options menupopup.
   * @property {Element} statusEl - The bridge status element.
   * @property {Element} statusText - The status text.
   * @property {string} bridgeLine - The identifying bridge string for this row.
   * @property {string?} bridgeId - The ID/fingerprint for the bridge, or null
   *   if it doesn't have one.
   * @property {integer} index - The index of the row in the grid.
   * @property {boolean} connected - Whether we are connected to the bridge
   *   (recently in use for a Tor circuit).
   * @property {BridgeGridCell[]} cells - The cells that belong to the row,
   *   ordered by their column.
   */
  /**
   * @typedef {object} BridgeGridCell
   *
   * @property {Element} element - The cell element.
   * @property {Element} focusEl - The element belonging to the cell that should
   *   receive focus. Should be the cell element itself, or an interactive
   *   focusable child.
   * @property {integer} columnIndex - The index of the column this cell belongs
   *   to.
   * @property {BridgeGridRow} row - The row this cell belongs to.
   */
  /**
   * The current rows in the grid.
   *
   * @type {BridgeGridRow[]}
   */
  _rows: [],
  /**
   * The cell that should be the focus target when the user moves focus into the
   * grid, or null if the grid itself should be the target.
   *
   * @type {BridgeGridCell?}
   */
  _focusCell: null,

  /**
   * Initialize the bridge grid.
   */
  init() {
    this._grid = document.getElementById("tor-bridges-grid-display");
    // Initially, make only the grid itself part of the keyboard tab cycle.
    // matches _focusCell = null.
    this._grid.tabIndex = 0;

    this._rowTemplate = document.getElementById(
      "tor-bridges-grid-row-template"
    );

    this._grid.addEventListener("keydown", this);
    this._grid.addEventListener("mousedown", this);
    this._grid.addEventListener("focusin", this);

    this._supportedSources = [
      lazy.TorBridgeSource.BridgeDB,
      lazy.TorBridgeSource.UserProvided,
      lazy.TorBridgeSource.Lox,
    ];
  },

  /**
   * Whether the grid is visible and responsive.
   *
   * @type {boolean}
   */
  _active: false,

  /**
   * Activate and show the bridge grid.
   */
  activate() {
    if (this._active) {
      return;
    }

    this._active = true;

    this._grid.hidden = false;
  },

  /**
   * Deactivate and hide the bridge grid.
   */
  deactivate() {
    if (!this._active) {
      return;
    }

    this._active = false;

    this._forceCloseRowMenus();

    this._grid.hidden = true;
  },

  handleEvent(event) {
    if (event.type === "keydown") {
      if (event.altKey || event.shiftKey || event.metaKey || event.ctrlKey) {
        // Don't interfere with these events.
        return;
      }

      if (this._rows.some(row => row.menu.open)) {
        // Have an open menu, let the menu handle the event instead.
        return;
      }

      let numRows = this._rows.length;
      if (!numRows) {
        // Nowhere for focus to go.
        return;
      }

      let moveRow = 0;
      let moveColumn = 0;
      const isLTR = this._grid.matches(":dir(ltr)");
      switch (event.key) {
        case "ArrowDown":
          moveRow = 1;
          break;
        case "ArrowUp":
          moveRow = -1;
          break;
        case "ArrowRight":
          moveColumn = isLTR ? 1 : -1;
          break;
        case "ArrowLeft":
          moveColumn = isLTR ? -1 : 1;
          break;
        default:
          return;
      }

      // Prevent scrolling the nearest scroll container.
      event.preventDefault();

      const curCell = this._focusCell;
      let row = curCell ? curCell.row.index + moveRow : 0;
      let column = curCell ? curCell.columnIndex + moveColumn : 0;

      // Clamp in bounds.
      if (row < 0) {
        row = 0;
      } else if (row >= numRows) {
        row = numRows - 1;
      }

      const numCells = this._rows[row].cells.length;
      if (column < 0) {
        column = 0;
      } else if (column >= numCells) {
        column = numCells - 1;
      }

      const newCell = this._rows[row].cells[column];

      if (newCell !== curCell) {
        this._setFocus(newCell);
      }
    } else if (event.type === "mousedown") {
      if (event.button !== 0) {
        return;
      }
      // Move focus index to the clicked target.
      // NOTE: Since the cells and the grid have "tabindex=-1", they are still
      // click-focusable. Therefore, the default mousedown handler will try to
      // move focus to it.
      // Rather than block this default handler, we instead re-direct the focus
      // to the correct cell in the "focusin" listener.
      const newCell = this._getCellFromTarget(event.target);
      // NOTE: If newCell is null, then we do nothing here, but instead wait for
      // the focusin handler to trigger.
      if (newCell && newCell !== this._focusCell) {
        this._setFocus(newCell);
      }
    } else if (event.type === "focusin") {
      const focusCell = this._getCellFromTarget(event.target);
      if (focusCell !== this._focusCell) {
        // Focus is not where it is expected.
        // E.g. the user has clicked the edge of the grid.
        // Restore focus immediately back to the cell we expect.
        this._setFocus(this._focusCell);
      }
    }
  },

  /**
   * Return the cell that was the target of an event.
   *
   * @param {Element} element - The target of an event.
   *
   * @returns {BridgeGridCell?} - The cell that the element belongs to, or null
   *   if it doesn't belong to any cell.
   */
  _getCellFromTarget(element) {
    for (const row of this._rows) {
      for (const cell of row.cells) {
        if (cell.element.contains(element)) {
          return cell;
        }
      }
    }
    return null;
  },

  /**
   * Determine whether the document's active element (focus) is within the grid
   * or not.
   *
   * @returns {boolean} - Whether focus is within this grid or not.
   */
  _focusWithin() {
    return this._grid.contains(document.activeElement);
  },

  /**
   * Set the cell that should be the focus target of the grid, possibly moving
   * the document's focus as well.
   *
   * @param {BridgeGridCell?} cell - The cell to make the focus target, or null
   *   if the grid itself should be the target.
   * @param {boolean} [focusWithin] - Whether focus should be moved within the
   *   grid. If undefined, this will move focus if the grid currently contains
   *   the document's focus.
   */
  _setFocus(cell, focusWithin) {
    if (focusWithin === undefined) {
      focusWithin = this._focusWithin();
    }
    const prevFocusElement = this._focusCell
      ? this._focusCell.focusEl
      : this._grid;
    const newFocusElement = cell ? cell.focusEl : this._grid;

    if (prevFocusElement !== newFocusElement) {
      prevFocusElement.tabIndex = -1;
      newFocusElement.tabIndex = 0;
    }
    // Set _focusCell now, before we potentially call "focus", which can trigger
    // the "focusin" handler.
    this._focusCell = cell;

    if (focusWithin) {
      // Focus was within the grid, so we need to actively move it to the new
      // element.
      newFocusElement.focus({ preventScroll: true });
      // Scroll to the whole cell into view, rather than just the focus element.
      (cell?.element ?? newFocusElement).scrollIntoView({
        block: "nearest",
        inline: "nearest",
      });
    }
  },

  /**
   * Reset the grids focus to be the first row's first cell, if any.
   *
   * @param {boolean} [focusWithin] - Whether focus should be moved within the
   *   grid. If undefined, this will move focus if the grid currently contains
   *   the document's focus.
   */
  _resetFocus(focusWithin) {
    this._setFocus(
      this._rows.length ? this._rows[0].cells[0] : null,
      focusWithin
    );
  },

  /**
   * The bridge ID/fingerprint of the most recently used bridge (appearing in
   * the latest Tor circuit). Roughly corresponds to the bridge we are currently
   * connected to.
   *
   * null if there are no such bridges.
   *
   * @type {string?}
   */
  _connectedBridgeId: null,

  set connectedBridgeId(bridgeId) {
    if (bridgeId === this._connectedBridgeId) {
      return;
    }
    this._connectedBridgeId = bridgeId;
    for (const row of this._rows) {
      this._updateRowStatus(row);
    }
  },

  /**
   * Update the status of a row.
   *
   * @param {BridgeGridRow} row - The row to update.
   */
  _updateRowStatus(row) {
    const connected = row.bridgeId && this._connectedBridgeId === row.bridgeId;
    // NOTE: row.connected is initially undefined, so won't match `connected`.
    if (connected === row.connected) {
      return;
    }

    row.connected = connected;

    const noStatus = !connected;

    row.element.classList.toggle("hide-status", noStatus);
    row.statusEl.classList.toggle("bridge-status-none", noStatus);
    row.statusEl.classList.toggle("bridge-status-connected", connected);

    if (connected) {
      document.l10n.setAttributes(
        row.statusText,
        "tor-bridges-status-connected"
      );
    } else {
      document.l10n.setAttributes(row.statusText, "tor-bridges-status-none");
    }
  },

  /**
   * Create a new row for the grid.
   *
   * @param {string} bridgeLine - The bridge line for this row, which also acts
   *   as its ID.
   *
   * @returns {BridgeGridRow} - A new row, with then "index" unset and the
   *   "element" without a parent.
   */
  _createRow(bridgeLine) {
    let details;
    try {
      details = lazy.TorParsers.parseBridgeLine(bridgeLine);
    } catch (e) {
      console.error(`Detected invalid bridge line: ${bridgeLine}`, e);
    }
    const row = {
      element: this._rowTemplate.content.children[0].cloneNode(true),
      bridgeLine,
      bridgeId: details?.id ?? null,
      cells: [],
    };

    const emojiBlock = row.element.querySelector(".tor-bridges-emojis-block");
    const BridgeEmoji = customElements.get("tor-bridge-emoji");
    for (const cell of BridgeEmoji.createForAddress(bridgeLine)) {
      // Each emoji is its own cell, we rely on the fact that createForAddress
      // always returns four elements.
      cell.setAttribute("role", "gridcell");
      cell.classList.add("tor-bridges-grid-cell", "tor-bridges-emoji-cell");
      emojiBlock.append(cell);
    }

    for (const [columnIndex, element] of row.element
      .querySelectorAll(".tor-bridges-grid-cell")
      .entries()) {
      const focusEl =
        element.querySelector(".tor-bridges-grid-focus") ?? element;
      // Set a negative tabIndex, this makes the element click-focusable but not
      // part of the tab navigation sequence.
      focusEl.tabIndex = -1;
      row.cells.push({ element, focusEl, columnIndex, row });
    }

    const transport = details?.transport ?? "vanilla";
    const typeCell = row.element.querySelector(".tor-bridges-type-cell");
    if (transport === "vanilla") {
      document.l10n.setAttributes(typeCell, "tor-bridges-type-prefix-generic");
    } else {
      document.l10n.setAttributes(typeCell, "tor-bridges-type-prefix", {
        type: transport,
      });
    }

    row.element.querySelector(".tor-bridges-address-cell-text").textContent =
      bridgeLine;

    row.statusEl = row.element.querySelector(
      ".tor-bridges-status-cell .bridge-status-badge"
    );
    row.statusText = row.element.querySelector(".tor-bridges-status-cell-text");

    this._initRowMenu(row);

    this._updateRowStatus(row);
    return row;
  },

  /**
   * The row menu index used for generating new ids.
   *
   * @type {integer}
   */
  _rowMenuIndex: 0,
  /**
   * Generate a new id for the options menu.
   *
   * @returns {string} - The new id.
   */
  _generateRowMenuId() {
    const id = `tor-bridges-individual-options-menu-${this._rowMenuIndex}`;
    // Assume we won't run out of ids.
    this._rowMenuIndex++;
    return id;
  },

  /**
   * Initialize the shared menu for a row.
   *
   * @param {BridgeGridRow} row - The row to initialize the menu of.
   */
  _initRowMenu(row) {
    row.menu = row.element.querySelector(
      ".tor-bridges-individual-options-menu"
    );
    row.optionsButton = row.element.querySelector(
      ".tor-bridges-options-cell-button"
    );

    row.menu.id = this._generateRowMenuId();
    row.optionsButton.setAttribute("aria-controls", row.menu.id);

    row.optionsButton.addEventListener("click", event => {
      row.menu.toggle(event);
    });

    row.menu.addEventListener("hidden", () => {
      // Make sure the button receives focus again when the menu is hidden.
      // Currently, panel-list.js only does this when the menu is opened with a
      // keyboard, but this causes focus to be lost from the page if the user
      // uses a mixture of keyboard and mouse.
      row.optionsButton.focus();
    });

    const qrItem = row.menu.querySelector(
      ".tor-bridges-options-qr-one-menu-item"
    );
    const removeItem = row.menu.querySelector(
      ".tor-bridges-options-remove-one-menu-item"
    );
    row.menu.addEventListener("showing", () => {
      const show =
        this._bridgeSource === lazy.TorBridgeSource.UserProvided ||
        this._bridgeSource === lazy.TorBridgeSource.BridgeDB;
      qrItem.hidden = !show;
      removeItem.hidden = !show;
    });

    qrItem.addEventListener("click", () => {
      const bridgeLine = row.bridgeLine;
      if (!bridgeLine) {
        return;
      }
      showBridgeQr(bridgeLine);
    });
    row.menu
      .querySelector(".tor-bridges-options-copy-one-menu-item")
      .addEventListener("click", () => {
        const clipboard = Cc[
          "@mozilla.org/widget/clipboardhelper;1"
        ].getService(Ci.nsIClipboardHelper);
        clipboard.copyString(row.bridgeLine);
      });
    removeItem.addEventListener("click", () => {
      const bridgeLine = row.bridgeLine;
      const source = lazy.TorSettings.bridges.source;
      if (source !== this._bridgesVal?.source) {
        // Our value is stale, abort.
        return;
      }
      const strings = lazy.TorSettings.bridges.bridge_strings;
      const index = strings.indexOf(bridgeLine);
      if (index === -1) {
        return;
      }
      strings.splice(index, 1);

      if (strings.length) {
        lazy.TorSettings.changeSettings({
          bridges: { source, bridge_strings: strings },
        });
      } else {
        // Remove all bridges and disable.
        lazy.TorSettings.changeSettings({
          bridges: { source: lazy.TorBridgeSource.Invalid },
        });
      }
    });
  },

  /**
   * Force the row menu to close.
   */
  _forceCloseRowMenus() {
    for (const row of this._rows) {
      row.menu.hide(null, { force: true });
    }
  },

  /**
   * The known bridge source.
   *
   * Initially null to indicate that it is unset.
   *
   * @type {integer?}
   */
  _bridgeSource: null,
  /**
   * The bridge sources this is shown for.
   *
   * @type {string[]}
   */
  _supportedSources: [],

  /**
   * The bridges value, set by the setting-control element.
   *
   * @type {object}
   */
  _bridgesVal: null,

  set bridges(val) {
    if (val === null) {
      // Ignore and wait for the initial.
      return;
    }
    const initial = this._bridgesVal === null;
    this._bridgesVal = val;
    this._updateRows(initial);
  },

  /**
   * Update the grid to show the latest bridge strings.
   *
   * @param {boolean} initializing - Whether this is being called as part of
   *   initialization.
   */
  _updateRows(initializing) {
    // Store whether we have focus within the grid, before removing or hiding
    // DOM elements.
    const focusWithin = this._focusWithin();

    let lostAllBridges = false;
    let newSource = false;
    const bridgeSource = this._bridgesVal.source;
    if (bridgeSource !== this._bridgeSource) {
      newSource = true;

      this._bridgeSource = bridgeSource;

      if (this._supportedSources.includes(bridgeSource)) {
        this.activate();
      } else {
        if (this._active && bridgeSource === lazy.TorBridgeSource.Invalid) {
          lostAllBridges = true;
        }
        this.deactivate();
      }
    }

    const ordered = this._active
      ? this._bridgesVal.bridgeStrings.map(bridgeLine => {
          const row = this._rows.find(r => r.bridgeLine === bridgeLine);
          if (row) {
            return row;
          }
          return this._createRow(bridgeLine);
        })
      : [];

    // Whether we should reset the grid's focus.
    // We always reset when we have a new bridge source.
    // We reset the focus if no current Cell has focus. I.e. when adding a row
    // to an empty grid, we want the focus to move to the first item.
    // We also reset the focus if the current Cell is in a row that will be
    // removed (including if all rows are removed).
    // NOTE: In principle, if a row is removed, we could move the focus to the
    // next or previous row (in the same cell column). However, most likely if
    // the grid has the user focus, they are removing a single row using its
    // options button. In this case, returning the user to some other row's
    // options button might be more disorienting since it would not be simple
    // for them to know *which* bridge they have landed on.
    // NOTE: We do not reset the focus in other cases because we do not want the
    // user to loose their place in the grid unnecessarily.
    let resetFocus =
      newSource || !this._focusCell || !ordered.includes(this._focusCell.row);

    // Remove rows no longer needed from the DOM.
    let numRowsRemoved = 0;
    let rowAddedOrMoved = false;

    for (const row of this._rows) {
      if (!ordered.includes(row)) {
        numRowsRemoved++;
        // If the row menu was open, it will also be deleted.
        // NOTE: Since the row menu is part of the row, focusWithin will be true
        // if the menu had focus, so focus should be re-assigned.
        row.element.remove();
      }
    }

    // Go through all the rows to set their ".index" property and to ensure they
    // are in the correct position in the DOM.
    // NOTE: We could use replaceChildren to get the correct DOM structure, but
    // we want to avoid rebuilding the entire tree when a single row is added or
    // removed.
    for (const [index, row] of ordered.entries()) {
      row.index = index;
      const element = row.element;
      // Get the expected previous element, that should already be in the DOM
      // from the previous loop.
      const prevEl = index ? ordered[index - 1].element : null;

      if (
        element.parentElement === this._grid &&
        prevEl === element.previousElementSibling
      ) {
        // Already in the correct position in the DOM.
        continue;
      }

      rowAddedOrMoved = true;
      // NOTE: Any elements already in the DOM, but not in the correct position
      // will be removed and re-added by the below command.
      // NOTE: if the row has document focus, then it should remain there.
      if (prevEl) {
        prevEl.after(element);
      } else {
        this._grid.prepend(element);
      }
    }
    this._rows = ordered;

    // Restore any lost focus.
    if (resetFocus) {
      // If we are not active (and therefore hidden), we will not try and move
      // focus (activeElement), but may still change the *focusable* element for
      // when we are shown again.
      this._resetFocus(this._active && focusWithin);
    }
    // NOTE: In the case we were previously active and now inactive,
    // tor-bridges-display will have already moved the focus out of this area.

    // Notify the user if there was some change to the DOM.
    // If we are initializing, we generate no notification since there has been
    // no change in the setting.
    if (!initializing) {
      let notificationType;
      if (lostAllBridges) {
        // Just lost all bridges, and became de-active.
        notificationType = "removed-all";
      } else if (this._rows.length) {
        // Otherwise, only generate a notification if we are still active, with
        // at least one bridge.
        // I.e. do not generate a message if the new source is "builtin".
        if (newSource) {
          // A change in source.
          notificationType = "changed";
        } else if (numRowsRemoved === 1 && !rowAddedOrMoved) {
          // Only one bridge was removed. This is most likely in response to them
          // manually removing a single bridge or using the bridge row's options
          // menu.
          notificationType = "removed-one";
        } else if (numRowsRemoved || rowAddedOrMoved) {
          // Some other change. This is most likely in response to a manual edit
          // of the existing bridges.
          notificationType = "changed";
        }
        // Else, there was no change.
      }

      if (notificationType) {
        postBridgeNotification(notificationType);
      }
    }
  },
};

/**
 * Controls the built-in bridges area.
 */
const gBuiltinBridgesArea = {
  /**
   * The display area.
   *
   * @type {Element?}
   */
  _area: null,
  /**
   * The type name element.
   *
   * @type {Element?}
   */
  _nameEl: null,
  /**
   * The bridge type description element.
   *
   * @type {Element?}
   */
  _descriptionEl: null,
  /**
   * The connection status.
   *
   * @type {Element?}
   */
  _connectionStatusEl: null,

  /**
   * Initialize the built-in bridges area.
   */
  init() {
    this._area = document.getElementById("tor-bridges-built-in-display");
    this._nameEl = document.getElementById("tor-bridges-built-in-type-name");
    this._descriptionEl = document.getElementById(
      "tor-bridges-built-in-description"
    );
    this._connectionStatusEl = document.getElementById(
      "tor-bridges-built-in-connected"
    );
  },

  /**
   * Whether the built-in area is visible and responsive.
   *
   * @type {boolean}
   */
  _active: false,

  /**
   * Activate and show the built-in bridge area.
   */
  activate() {
    if (this._active) {
      return;
    }
    this._active = true;

    this._area.hidden = false;
  },

  /**
   * Deactivate and hide built-in bridge area.
   */
  deactivate() {
    if (!this._active) {
      return;
    }
    this._active = false;

    this._area.hidden = true;
  },

  /**
   * Updates the shown connected state.
   */
  _updateConnectedState() {
    this._connectionStatusEl.classList.toggle(
      "bridge-status-connected",
      this._bridgeType &&
        this._connectedBridgeId &&
        this._bridgeIds.includes(this._connectedBridgeId)
    );
  },

  /**
   * The bridges value, set by the setting-control element.
   *
   * @type {object}
   */
  _bridgesVal: null,
  set bridges(val) {
    if (val === null) {
      // Ignore and wait for the initial.
      return;
    }
    const initial = this._bridgesVal === null;
    this._bridgesVal = val;
    this._updateBridgeType(initial);
    this._updateBridgeIds();
  },

  /**
   * The currently shown bridge type. Empty if deactivated, and null if
   * uninitialized.
   *
   * @type {string?}
   */
  _bridgeType: null,
  /**
   * The strings for each known bridge type.
   *
   * @type {{[key: string]: {[key: string]: string}}}
   */
  _bridgeTypeStrings: {
    obfs4: {
      name: "tor-bridges-built-in-obfs4-name",
      description: "tor-bridges-built-in-obfs4-description",
    },
    snowflake: {
      name: "tor-bridges-built-in-snowflake-name",
      description: "tor-bridges-built-in-snowflake-description",
    },
    meek: {
      name: "tor-bridges-built-in-meek-name",
      description: "tor-bridges-built-in-meek-description",
    },
  },

  /**
   * The known bridge source.
   *
   * Initially null to indicate that it is unset.
   *
   * @type {integer?}
   */
  _bridgeSource: null,

  /**
   * Update the shown bridge type.
   *
   * @param {boolean} initializing - Whether this is being called as part of
   *   initialization.
   */
  async _updateBridgeType(initializing) {
    let lostAllBridges = false;
    let newSource = false;
    const bridgeSource = this._bridgesVal.source;
    if (bridgeSource !== this._bridgeSource) {
      newSource = true;

      this._bridgeSource = bridgeSource;

      if (bridgeSource === lazy.TorBridgeSource.BuiltIn) {
        this.activate();
      } else {
        if (this._active && bridgeSource === lazy.TorBridgeSource.Invalid) {
          lostAllBridges = true;
        }
        this.deactivate();
        // NOTE: In the case we were previously active, tor-bridges-display will
        // have already moved the focus out of this area.
      }
    }

    const bridgeType = this._active ? this._bridgesVal.builtinType : "";

    let newType = false;
    if (bridgeType !== this._bridgeType) {
      newType = true;

      this._bridgeType = bridgeType;

      const bridgeStrings = this._bridgeTypeStrings[bridgeType];
      if (bridgeStrings) {
        document.l10n.setAttributes(this._nameEl, bridgeStrings.name);
        document.l10n.setAttributes(
          this._descriptionEl,
          bridgeStrings.description
        );
      } else {
        // Unknown type, or no type.
        this._nameEl.removeAttribute("data-l10n-id");
        this._nameEl.textContent = bridgeType;
        this._descriptionEl.removeAttribute("data-l10n-id");
        this._descriptionEl.textContent = "";
      }

      this._updateConnectedState();
    }

    // Notify the user if there was some change to the type.
    // If we are initializing, we generate no notification since there has been
    // no change in the setting.
    if (!initializing) {
      let notificationType;
      if (lostAllBridges) {
        // Just lost all bridges, and became de-active.
        notificationType = "removed-all";
      } else if (this._active && (newSource || newType)) {
        // Otherwise, only generate a notification if we are still active, with
        // a bridge type.
        // I.e. do not generate a message if the new source is not "builtin".
        notificationType = "changed";
      }

      if (notificationType) {
        postBridgeNotification(notificationType);
      }
    }
  },

  /**
   * The bridge IDs/fingerprints for the built-in bridges.
   *
   * @type {Array<string>}
   */
  _bridgeIds: [],
  /**
   * Update _bridgeIds
   */
  _updateBridgeIds() {
    this._bridgeIds = [];
    for (const bridgeLine of this._bridgesVal.bridgeStrings) {
      try {
        this._bridgeIds.push(lazy.TorParsers.parseBridgeLine(bridgeLine).id);
      } catch (e) {
        console.error(`Detected invalid bridge line: ${bridgeLine}`, e);
      }
    }

    this._updateConnectedState();
  },

  /**
   * The bridge ID/fingerprint of the most recently used bridge (appearing in
   * the latest Tor circuit). Roughly corresponds to the bridge we are currently
   * connected to.
   *
   * @type {string?}
   */
  _connectedBridgeId: null,

  set connectedBridgeId(val) {
    this._connectedBridgeId = val;
    this._updateConnectedState();
  },
};

/**
 * Controls the bridge pass area.
 */
const gLoxStatus = {
  /**
   * The status area.
   *
   * @type {Element?}
   */
  _area: null,
  /**
   * The area for showing the next unlock and invites.
   *
   * @type {Element?}
   */
  _detailsArea: null,
  /**
   * The list items showing the next unlocks.
   *
   * @type {?{[key: string]: Element}}
   */
  _nextUnlockItems: null,
  /**
   * The day counter headings for the next unlock.
   *
   * One heading is shown during a search, the other is shown otherwise.
   *
   * @type {?Element[]}
   */
  _nextUnlockCounterEls: null,
  /**
   * Shows the number of remaining invites.
   *
   * @type {Element?}
   */
  _remainingInvitesEl: null,
  /**
   * The button to show the invites.
   *
   * @type {Element?}
   */
  _invitesButton: null,
  /**
   * The alert for new unlocks.
   *
   * @type {Element?}
   */
  _unlockAlert: null,
  /**
   * The list items showing the unlocks.
   *
   * @type {?{[key: string]: Element}}
   */
  _unlockItems: null,
  /**
   * The alert title.
   *
   * @type {Element?}
   */
  _unlockAlertTitle: null,
  /**
   * The alert invites item.
   *
   * @type {Element?}
   */
  _unlockAlertInvitesItem: null,
  /**
   * Button for the user to dismiss the alert.
   *
   * @type {Element?}
   */
  _unlockAlertButton: null,

  _enabled: false,

  /**
   * Initialize the bridge pass area.
   */
  init() {
    if (!lazy.Lox.enabled) {
      // Area should remain inactive and hidden.
      return;
    }

    this._enabled = true;
    this._area = document.getElementById("tor-bridges-lox-status");
    this._detailsArea = document.getElementById("tor-bridges-lox-details");
    this._nextUnlockItems = {
      gainBridges: document.getElementById(
        "tor-bridges-lox-next-unlock-gain-bridges"
      ),
      firstInvites: document.getElementById(
        "tor-bridges-lox-next-unlock-first-invites"
      ),
      moreInvites: document.getElementById(
        "tor-bridges-lox-next-unlock-more-invites"
      ),
    };
    this._nextUnlockCounterEls = Array.from(
      document.querySelectorAll(".tor-bridges-lox-next-unlock-counter")
    );
    this._remainingInvitesEl = document.getElementById(
      "tor-bridges-lox-remaining-invites"
    );
    this._invitesButton = document.getElementById(
      "tor-bridges-lox-show-invites-button"
    );
    this._unlockAlert = document.getElementById("tor-bridges-lox-unlock-alert");
    this._unlockItems = {
      gainBridges: document.getElementById(
        "tor-bridges-lox-unlock-alert-gain-bridges"
      ),
      newBridges: document.getElementById(
        "tor-bridges-lox-unlock-alert-new-bridges"
      ),
      invites: document.getElementById("tor-bridges-lox-unlock-alert-invites"),
    };
    this._unlockAlertTitle = document.getElementById(
      "tor-bridge-unlock-alert-title"
    );
    this._unlockAlertInviteItem = document.getElementById(
      "tor-bridges-lox-unlock-alert-invites"
    );
    this._unlockAlertButton = document.getElementById(
      "tor-bridges-lox-unlock-alert-button"
    );

    this._invitesButton.addEventListener("click", () => {
      window.gSubDialog.open(
        "chrome://browser/content/torpreferences/loxInviteDialog.xhtml",
        { features: "resizable=yes" }
      );
    });
    this._unlockAlertButton.addEventListener("click", () => {
      lazy.Lox.clearEventData(this._loxId);
    });

    Services.obs.addObserver(this, lazy.LoxTopics.UpdateActiveLoxId);
    Services.obs.addObserver(this, lazy.LoxTopics.UpdateEvents);
    Services.obs.addObserver(this, lazy.LoxTopics.UpdateNextUnlock);
    Services.obs.addObserver(this, lazy.LoxTopics.UpdateRemainingInvites);
    Services.obs.addObserver(this, lazy.LoxTopics.NewInvite);

    window.addEventListener(
      "unload",
      () => {
        Services.obs.removeObserver(this, lazy.LoxTopics.UpdateActiveLoxId);
        Services.obs.removeObserver(this, lazy.LoxTopics.UpdateEvents);
        Services.obs.removeObserver(this, lazy.LoxTopics.UpdateNextUnlock);
        Services.obs.removeObserver(
          this,
          lazy.LoxTopics.UpdateRemainingInvites
        );
        Services.obs.removeObserver(this, lazy.LoxTopics.NewInvite);
      },
      { once: true }
    );
  },

  observe(subject, topic) {
    switch (topic) {
      case lazy.LoxTopics.UpdateActiveLoxId:
        this._updateLoxId();
        break;
      case lazy.LoxTopics.UpdateNextUnlock:
        this._updateNextUnlock();
        break;
      case lazy.LoxTopics.UpdateEvents:
        this._updatePendingEvents();
        break;
      case lazy.LoxTopics.UpdateRemainingInvites:
        this._updateRemainingInvites();
        break;
      case lazy.LoxTopics.NewInvite:
        this._updateHaveExistingInvites();
        break;
    }
  },

  /**
   * The bridges value, set by the setting-control element.
   *
   * @type {object}
   */
  _bridgesVal: null,
  set bridges(val) {
    if (val === null) {
      // Ignore and wait for initial.
      return;
    }
    if (!this._enabled) {
      // Area should remain inactive and hidden.
      return;
    }
    this._bridgesVal = val;
    this._updateLoxId();
  },

  /**
   * The Lox id currently shown. Empty if deactivated, and null if
   * uninitialized.
   *
   * @type {string?}
   */
  _loxId: null,

  /**
   * Update the shown bridge pass.
   */
  async _updateLoxId() {
    let loxId =
      this._bridgesVal?.source === lazy.TorBridgeSource.Lox
        ? lazy.Lox.activeLoxId
        : "";
    if (loxId === this._loxId) {
      return;
    }
    this._loxId = loxId;
    this._area.hidden = !loxId;
    // We unset _nextUnlock to ensure the areas no longer use the old value for
    // the new loxId.
    this._updateNextUnlock(true);
    this._updateRemainingInvites();
    this._updateHaveExistingInvites();
    this._updatePendingEvents();
  },

  /**
   * The remaining invites shown, or null if uninitialized or no loxId.
   *
   * @type {integer?}
   */
  _remainingInvites: null,
  /**
   * Update the shown value.
   */
  _updateRemainingInvites() {
    const numInvites = this._loxId
      ? lazy.Lox.getRemainingInviteCount(this._loxId)
      : null;
    if (numInvites === this._remainingInvites) {
      return;
    }
    this._remainingInvites = numInvites;
    this._updateUnlockArea();
    this._updateInvitesArea();
  },
  /**
   * Whether we have existing invites, or null if uninitialized or no loxId.
   *
   * @type {boolean?}
   */
  _haveExistingInvites: null,
  /**
   * Update the shown value.
   */
  _updateHaveExistingInvites() {
    const haveInvites = this._loxId ? !!lazy.Lox.getInvites().length : null;
    if (haveInvites === this._haveExistingInvites) {
      return;
    }
    this._haveExistingInvites = haveInvites;
    this._updateInvitesArea();
  },
  /**
   * Details about the next unlock, or null if uninitialized or no loxId.
   *
   * @type {UnlockData?}
   */
  _nextUnlock: null,
  /**
   * Tracker id to ensure that the results from later calls to _updateNextUnlock
   * take priority over earlier calls.
   *
   * @type {integer}
   */
  _nextUnlockCallId: 0,
  /**
   * Update the shown value asynchronously.
   *
   * @param {boolean} [unset=false] - Whether to set the _nextUnlock value to
   *   null before waiting for the new value. I.e. ensure that the current value
   *   will not be used.
   */
  async _updateNextUnlock(unset = false) {
    // NOTE: We do not expect the integer to exceed the maximum integer.
    this._nextUnlockCallId++;
    const callId = this._nextUnlockCallId;
    if (unset) {
      this._nextUnlock = null;
    }
    const nextUnlock = this._loxId
      ? await lazy.Lox.getNextUnlock(this._loxId)
      : null;
    if (callId !== this._nextUnlockCallId) {
      // Replaced by another update.
      // E.g. if the _loxId changed. Or if getNextUnlock triggered
      // LoxTopics.UpdateNextUnlock.
      return;
    }
    // Should be safe to trigger the update, even when the value hasn't changed.
    this._nextUnlock = nextUnlock;
    this._updateUnlockArea();
  },
  /**
   * The list of events the user has not yet cleared, or null if uninitialized
   * or no loxId.
   *
   * @type {EventData[]?}
   */
  _pendingEvents: null,
  /**
   * Update the shown value.
   */
  _updatePendingEvents() {
    // Should be safe to trigger the update, even when the value hasn't changed.
    this._pendingEvents = this._loxId
      ? lazy.Lox.getEventData(this._loxId)
      : null;
    this._updateUnlockArea();
  },

  /**
   * Update the display of the current or next unlock.
   */
  _updateUnlockArea() {
    if (
      !this._loxId ||
      this._pendingEvents === null ||
      this._remainingInvites === null ||
      this._nextUnlock === null
    ) {
      // Uninitialized or no Lox source.
      // NOTE: This area may already be hidden by the change in Lox source,
      // but we clean up for the next non-empty id.
      this._unlockAlert.hidden = true;
      this._detailsArea.hidden = true;
      return;
    }

    // Grab focus state before changing visibility.
    const alertHadFocus = this._unlockAlert.contains(document.activeElement);
    const detailsHadFocus = this._detailsArea.contains(document.activeElement);

    const pendingEvents = this._pendingEvents;
    const showAlert = !!pendingEvents.length;
    this._unlockAlert.hidden = !showAlert;
    this._detailsArea.hidden = showAlert;

    if (showAlert) {
      // At level 0 and level 1, we do not have any invites.
      // If the user starts and ends on level 0 or 1, then overall they would
      // have had no change in their invites. So we do not want to show their
      // latest updates.
      // NOTE: If the user starts at level > 1 and ends with level 1 (levelling
      // down to level 0 should not be possible), then we *do* want to show the
      // user that they now have "0" invites.
      // NOTE: pendingEvents are time-ordered, with the most recent event
      // *last*.
      const firstEvent = pendingEvents[0];
      // NOTE: We cannot get a blockage event when the user starts at level 1 or
      // 0.
      const startingAtLowLevel =
        firstEvent.type === "levelup" && firstEvent.newLevel <= 2;
      const lastEvent = pendingEvents[pendingEvents.length - 1];
      const endingAtLowLevel = lastEvent.newLevel <= 1;

      const showInvites = !(startingAtLowLevel && endingAtLowLevel);

      let blockage = false;
      let levelUp = false;
      let bridgeGain = false;
      // Go through events, in the order that they occurred.
      for (const loxEvent of pendingEvents) {
        if (loxEvent.type === "levelup") {
          levelUp = true;
          if (loxEvent.newLevel === 1) {
            // Gain 2 bridges from level 0 to 1.
            bridgeGain = true;
          }
        } else {
          blockage = true;
        }
      }

      let alertTitleId;
      if (levelUp && !blockage) {
        alertTitleId = "tor-bridges-lox-upgrade";
      } else {
        // Show as blocked bridges replaced.
        // Even if we have a mixture of level ups as well.
        alertTitleId = "tor-bridges-lox-blocked";
      }
      document.l10n.setAttributes(this._unlockAlertTitle, alertTitleId);
      document.l10n.setAttributes(
        this._unlockAlertInviteItem,
        "tor-bridges-lox-new-invites",
        { numInvites: this._remainingInvites }
      );
      this._unlockAlert.classList.toggle(
        "lox-unlock-upgrade",
        levelUp && !blockage
      );
      this._unlockItems.gainBridges.hidden = !bridgeGain;
      this._unlockItems.newBridges.hidden = !blockage;
      this._unlockItems.invites.hidden = !showInvites;
    } else {
      // Show next unlock.
      // Number of days until the next unlock, rounded up.
      const numDays = Math.max(
        1,
        Math.ceil(
          (new Date(this._nextUnlock.date).getTime() - Date.now()) /
            (24 * 60 * 60 * 1000)
        )
      );
      for (const counterEl of this._nextUnlockCounterEls) {
        document.l10n.setAttributes(
          counterEl,
          "tor-bridges-lox-days-until-unlock",
          { numDays }
        );
      }

      // Gain 2 bridges from level 0 to 1. After that gain invites.
      this._nextUnlockItems.gainBridges.hidden =
        this._nextUnlock.nextLevel !== 1;
      this._nextUnlockItems.firstInvites.hidden =
        this._nextUnlock.nextLevel !== 2;
      this._nextUnlockItems.moreInvites.hidden =
        this._nextUnlock.nextLevel <= 2;
    }

    if (alertHadFocus && !showAlert) {
      // Alert has become hidden, move focus back up to the now revealed details
      // area.
      // NOTE: We have two headings: one shown during a search and one shown
      // otherwise. We focus the heading that is currently visible.
      // See tor-browser#43320.
      // TODO: It might be better if we could use the # named anchor to
      // re-orient the screen reader position instead of using tabIndex=-1, but
      // about:preferences currently uses the anchor for showing categories
      // only. See bugzilla bug 1799153.
      if (
        this._nextUnlockCounterEls[0].checkVisibility({
          visibilityProperty: true,
        })
      ) {
        this._nextUnlockCounterEls[0].focus();
      } else {
        this._nextUnlockCounterEls[1].focus();
      }
    } else if (detailsHadFocus && showAlert) {
      this._unlockAlertButton.focus();
    }
  },

  /**
   * Update the invites area.
   */
  _updateInvitesArea() {
    let hasInvites;
    if (
      !this._loxId ||
      this._remainingInvites === null ||
      this._haveExistingInvites === null
    ) {
      // Not initialized yet.
      hasInvites = false;
    } else {
      hasInvites = this._haveExistingInvites || !!this._remainingInvites;
    }

    if (
      !hasInvites &&
      (this._remainingInvitesEl.contains(document.activeElement) ||
        this._invitesButton.contains(document.activeElement))
    ) {
      // About to loose focus.
      // Unexpected for the lox level to loose all invites.
      // Move to the top of the details area, which should be visible if we
      // just had focus.
      this._nextUnlockCounterEl.focus();
    }
    // Hide the invite elements if we have no historic invites or a way of
    // creating new ones.
    this._remainingInvitesEl.hidden = !hasInvites;
    this._invitesButton.hidden = !hasInvites;

    if (hasInvites) {
      document.l10n.setAttributes(
        this._remainingInvitesEl,
        "tor-bridges-lox-remaining-invites",
        { numInvites: this._remainingInvites }
      );
    }
  },
};

/**
 * Controls the bridge settings.
 */
const gBridgeSettings = {
  /**
   * The display area.
   *
   * @type {Element?}
   */
  _displayEl: null,
  /**
   * The area for showing current bridges.
   *
   * @type {Element?}
   */
  _bridgesEl: null,
  /**
   * The area for sharing bridge addresses.
   *
   * @type {Element?}
   */
  _shareEl: null,
  /**
   * The area for showing no bridges.
   *
   * @type {Element?}
   */
  _noBridgesEl: null,
  /**
   * A map from the bridge source to its corresponding label.
   *
   * @type {?Map<number, Element>}
   */
  _sourceLabels: null,

  /**
   * Whether we have been initialized.
   *
   * @type {boolean}
   */
  _initialized: false,

  /**
   * Initialize the bridge settings.
   *
   * @param {Element} displayEl - The widget element we are controlling.
   */
  init(displayEl) {
    if (this._initialized) {
      return;
    }

    this._displayEl = displayEl;
    this._bridgesEl = document.getElementById("tor-bridges-current");
    this._noBridgesEl = document.getElementById("tor-bridges-none");

    this._sourceLabels = new Map([
      [
        lazy.TorBridgeSource.BuiltIn,
        document.getElementById("tor-bridges-built-in-label"),
      ],
      [
        lazy.TorBridgeSource.UserProvided,
        document.getElementById("tor-bridges-user-label"),
      ],
      [
        lazy.TorBridgeSource.BridgeDB,
        document.getElementById("tor-bridges-requested-label"),
      ],
      [
        lazy.TorBridgeSource.Lox,
        document.getElementById("tor-bridges-lox-label"),
      ],
    ]);
    this._shareEl = document.getElementById("tor-bridges-share");

    this._initBridgesMenu();
    this._initShareArea();

    gBridgeGrid.init();
    gBuiltinBridgesArea.init();
    gLoxStatus.init();
    this._initialized = true;
    // Re-trigger our current bridges value to pass on to any descendants.
    this.bridges = this._bridgesVal;
    this.connectedBridgeId = this._connectedBridgeId;
  },

  /**
   * The bridges value, set by the setting-control element.
   *
   * @type {object}
   */
  _bridgesVal: null,

  set bridges(val) {
    if (val === null) {
      // Corresponds to pending TorSettings initialization, wait for a non-null
      // value.
      return;
    }
    this._bridgesVal = val;
    if (!this._initialized) {
      return;
    }
    this._updateSource();
    this._updateBridgeStrings();
    // Pass on to descendants.
    gBridgeGrid.bridges = val;
    gBuiltinBridgesArea.bridges = val;
    gLoxStatus.bridges = val;
  },

  /**
   * The ID of the currently connected bridge, or `null` if there is none.
   *
   * @type {string?}
   */
  _connectedBridgeId: null,

  set connectedBridgeId(val) {
    this._connectedBridgeId = val;
    if (!this._initialized) {
      return;
    }
    // NOTE: This should be safe to call, even when _bridgesVal is still null.
    gBridgeGrid.connectedBridgeId = val;
    gBuiltinBridgesArea.connectedBridgeId = val;
  },

  /**
   * The shown bridge source.
   *
   * Initially null to indicate that it is unset for the first call to
   * _updateSource.
   *
   * @type {integer?}
   */
  _bridgeSource: null,
  /**
   * Whether the user is encouraged to share their bridge addresses.
   *
   * @type {boolean}
   */
  _canShare: false,

  /**
   * Update _bridgeSource.
   */
  _updateSource() {
    // NOTE: This should only ever be called after TorSettings is already
    // initialized.
    const bridgeSource = this._bridgesVal.source;
    if (bridgeSource === this._bridgeSource) {
      // Avoid re-activating an area if the source has not changed.
      return;
    }

    this._bridgeSource = bridgeSource;

    // Before hiding elements, we determine whether our region contained the
    // user focus.
    const hadFocus =
      this._bridgesEl.contains(document.activeElement) ||
      this._noBridgesEl.contains(document.activeElement);

    for (const [source, labelEl] of this._sourceLabels.entries()) {
      labelEl.hidden = source !== bridgeSource;
    }

    this._canShare =
      bridgeSource === lazy.TorBridgeSource.UserProvided ||
      bridgeSource === lazy.TorBridgeSource.BridgeDB;

    this._shareEl.hidden = !this._canShare;

    // Force the menu to close whenever the source changes.
    // NOTE: If the menu had focus then hadFocus will be true, and focus will be
    // re-assigned.
    this._forceCloseBridgesMenu();

    // Update whether we have bridges.
    this._updateHaveBridges();

    if (hadFocus) {
      // Always reset the focus to the start of the area whenever the source
      // changes.
      lazy.moveFocusToBridgeHeading(window);
    }
  },

  /**
   * Whether we have bridges or not, or null if it is unknown.
   *
   * @type {boolean?}
   */
  _haveBridges: null,

  /**
   * Update the _haveBridges value.
   */
  _updateHaveBridges() {
    const haveBridges = this._bridgesVal.haveBridges;

    if (haveBridges === this._haveBridges) {
      return;
    }

    this._haveBridges = haveBridges;

    // Add classes to show or hide the "no bridges" and "Your bridges" sections.
    this._bridgesEl.hidden = !haveBridges;
    this._noBridgesEl.hidden = haveBridges;

    this._displayEl.classList.toggle("has-tor-bridges", haveBridges);
  },

  /**
   * The bridge strings in a copy-able form.
   *
   * @type {string}
   */
  _bridgeStrings: "",
  /**
   * Whether the bridge strings should be shown as a QR code.
   *
   * @type {boolean}
   */
  _canQRBridges: false,

  /**
   * Update the stored bridge strings.
   */
  _updateBridgeStrings() {
    const bridges = this._bridgesVal.bridgeStrings;

    this._bridgeStrings = bridges.join("\n");
    // TODO: Determine what logic we want.
    this._canQRBridges = bridges.length <= 3;

    this._qrButton.disabled = !this._canQRBridges;
  },

  /**
   * Copy all the bridge addresses to the clipboard.
   */
  _copyBridges() {
    const clipboard = Cc["@mozilla.org/widget/clipboardhelper;1"].getService(
      Ci.nsIClipboardHelper
    );
    clipboard.copyString(this._bridgeStrings);
  },

  /**
   * Open the QR code dialog encoding all the bridge addresses.
   */
  _openQR() {
    if (!this._canQRBridges) {
      return;
    }
    showBridgeQr(this._bridgeStrings);
  },

  /**
   * The QR button for copying all QR codes.
   *
   * @type {Element?}
   */
  _qrButton: null,

  _initShareArea() {
    document
      .getElementById("tor-bridges-copy-addresses-button")
      .addEventListener("click", () => {
        this._copyBridges();
      });

    this._qrButton = document.getElementById("tor-bridges-qr-addresses-button");
    this._qrButton.addEventListener("click", () => {
      this._openQR();
    });
  },

  /**
   * The menu for all bridges.
   *
   * @type {Element?}
   */
  _bridgesMenu: null,

  /**
   * Initialize the menu for all bridges.
   */
  _initBridgesMenu() {
    this._bridgesMenu = document.getElementById("tor-bridges-all-options-menu");

    // NOTE: We generally assume that once the bridge menu is opened the
    // this._bridgeStrings value will not change.
    const qrItem = document.getElementById(
      "tor-bridges-options-qr-all-menu-item"
    );
    qrItem.addEventListener("click", () => {
      this._openQR();
    });

    const copyItem = document.getElementById(
      "tor-bridges-options-copy-all-menu-item"
    );
    copyItem.addEventListener("click", () => {
      this._copyBridges();
    });

    const editItem = document.getElementById(
      "tor-bridges-options-edit-all-menu-item"
    );
    editItem.addEventListener("click", () => {
      lazy.openUserProvideBridgeDialog(window, "edit");
    });

    // TODO: Do we want a different item for built-in bridges, rather than
    // "Remove all bridges"?
    document
      .getElementById("tor-bridges-options-remove-all-menu-item")
      .addEventListener("click", async () => {
        // TODO: Should we only have a warning when not built-in?
        const parentWindow =
          Services.wm.getMostRecentWindow("navigator:browser");
        const flags =
          Services.prompt.BUTTON_POS_0 *
            Services.prompt.BUTTON_TITLE_IS_STRING +
          Services.prompt.BUTTON_POS_0_DEFAULT +
          Services.prompt.BUTTON_DEFAULT_IS_DESTRUCTIVE +
          Services.prompt.BUTTON_POS_1 * Services.prompt.BUTTON_TITLE_CANCEL;

        const [titleString, bodyString, removeString] =
          await document.l10n.formatValues([
            { id: "remove-all-bridges-warning-title" },
            { id: "remove-all-bridges-warning-description" },
            { id: "remove-all-bridges-warning-remove-button" },
          ]);

        // TODO: Update the text, and remove old strings.
        const buttonIndex = Services.prompt.confirmEx(
          parentWindow,
          titleString,
          bodyString,
          flags,
          removeString,
          null,
          null,
          null,
          {}
        );

        if (buttonIndex !== 0) {
          return;
        }

        lazy.TorSettings.changeSettings({
          // This should always have the side effect of disabling bridges as
          // well.
          bridges: { source: lazy.TorBridgeSource.Invalid },
        });
      });

    this._bridgesMenu.addEventListener("showing", () => {
      qrItem.hidden = !this._canShare || !this._canQRBridges;
      editItem.hidden =
        this._bridgeSource !== lazy.TorBridgeSource.UserProvided;
    });

    const bridgesMenuButton = document.getElementById(
      "tor-bridges-all-options-button"
    );
    bridgesMenuButton.addEventListener("click", event => {
      this._bridgesMenu.toggle(event, bridgesMenuButton);
    });

    this._bridgesMenu.addEventListener("hidden", () => {
      // Make sure the button receives focus again when the menu is hidden.
      // Currently, panel-list.js only does this when the menu is opened with a
      // keyboard, but this causes focus to be lost from the page if the user
      // uses a mixture of keyboard and mouse.
      bridgesMenuButton.focus();
    });
  },

  /**
   * Force the bridges menu to close.
   */
  _forceCloseBridgesMenu() {
    this._bridgesMenu.hide(null, { force: true });
  },
};

// TODO: Replace gBridgeSettings and #tor-bridges-display-template with proper
// widgets (using MozLitElement).
/**
 * Show the current bridges.
 */
class TorBridgesDisplay extends HTMLElement {
  connectedCallback() {
    if (this.children.length) {
      return;
    }
    // Take the template children since we only expect one instance of this.
    this.replaceChildren(
      ...document.getElementById("tor-bridges-display-template").content
        .childNodes
    );
    gBridgeSettings.init(this);
  }

  set connectedBridgeId(val) {
    gBridgeSettings.connectedBridgeId = val;
  }

  set bridges(val) {
    gBridgeSettings.bridges = val;
  }

  /**
   * Focus the "Your bridges" heading, if it is visible.
   *
   * @returns {boolean} - `true` if the heading was visible and focused.
   */
  focusHeading() {
    if (!gBridgeSettings._haveBridges) {
      // Heading is hidden.
      return false;
    }
    document.getElementById("tor-bridges-current-heading-non-search").focus();
    return true;
  }
}
customElements.define("tor-bridges-display", TorBridgesDisplay);
