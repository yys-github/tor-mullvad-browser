/**
 * A TorConnect status shown in the application title bar.
 */
var gTorConnectTitlebarStatus = {
  /**
   * The status elements and their labels.
   *
   * @type {{status: Element, label: Element}[]}
   */
  _elements: [],
  /**
   * Whether we are connected, or null if the connection state is not yet known.
   *
   * @type {boolean?}
   */
  connected: null,

  /**
   * Initialize the component.
   */
  init() {
    const { TorStrings } = ChromeUtils.importESModule(
      "resource://gre/modules/TorStrings.sys.mjs"
    );

    this._strings = TorStrings.torConnect;

    this._elements = Array.from(
      document.querySelectorAll(".tor-connect-titlebar-status"),
      element => {
        return {
          status: element,
          label: element.querySelector(".tor-connect-titlebar-status-label"),
        };
      }
    );
    // The title also acts as an accessible name for the role="status".
    for (const { status } of this._elements) {
      status.setAttribute("title", this._strings.titlebarStatusName);
    }

    Services.obs.addObserver(this, TorConnectTopics.StageChange);

    this._torConnectStateChanged();
  },

  /**
   * De-initialize the component.
   */
  uninit() {
    Services.obs.removeObserver(this, TorConnectTopics.StageChange);
  },

  observe(subject, topic) {
    switch (topic) {
      case TorConnectTopics.StageChange:
        this._torConnectStateChanged();
        break;
    }
  },

  /**
   * Callback for when the TorConnect state changes.
   */
  _torConnectStateChanged() {
    let textId;
    let connected = false;
    let potentiallyBlocked = false;
    switch (TorConnect.stageName) {
      case TorConnectStage.Disabled:
        // Hide immediately.
        this._setHidden(true);
        return;
      case TorConnectStage.Bootstrapped:
        textId = "titlebarStatusConnected";
        connected = true;
        break;
      case TorConnectStage.Bootstrapping:
        textId = "titlebarStatusConnecting";
        break;
      default:
        if (TorConnect.potentiallyBlocked) {
          textId = "titlebarStatusPotentiallyBlocked";
          potentiallyBlocked = true;
        } else {
          textId = "titlebarStatusNotConnected";
        }
        break;
    }
    for (const { label } of this._elements) {
      label.textContent = this._strings[textId];
    }
    if (this.connected !== connected) {
      // When we are transitioning from
      //   this.connected = false
      // to
      //   this.connected = true
      // we want to animate the transition from the not connected state to the
      // connected state (provided prefers-reduced-motion is not set).
      //
      // If instead we are transitioning directly from the initial state
      //   this.connected = null
      // to
      //   this.connected = true
      // we want to immediately show the connected state without any transition.
      //
      // In both cases, the status will eventually be hidden.
      //
      // We only expect this latter case when opening a new window after
      // bootstrapping has already completed. See tor-browser#41850.
      for (const { status } of this._elements) {
        status.classList.toggle(
          "tor-connect-status-animate-transition",
          connected && this.connected !== null
        );
        status.classList.toggle("tor-connect-status-connected", connected);
      }
      this.connected = connected;
      if (connected) {
        this._startHiding();
      } else {
        // We can leave the connected state when we are no longer Bootstrapped
        // because the underlying tor process exited early and needs a
        // restart. In this case we want to re-show the status.
        this._stopHiding();
      }
    }
    for (const { status } of this._elements) {
      status.classList.toggle(
        "tor-connect-status-potentially-blocked",
        potentiallyBlocked
      );
    }
  },

  /**
   * Hide or show the status.
   *
   * @param {boolean} hide - Whether to hide the status.
   */
  _setHidden(hide) {
    for (const { status } of this._elements) {
      status.hidden = hide;
    }
  },

  /**
   * Mark the component to be hidden after some delay.
   */
  _startHiding() {
    if (this._hidingTimeout) {
      // Already hiding.
      return;
    }
    this._hidingTimeout = setTimeout(() => {
      this._setHidden(true);
    }, 5000);
  },

  /**
   * Re-show the component immediately.
   */
  _stopHiding() {
    if (this._hidingTimeout) {
      clearTimeout(this._hidingTimeout);
      this._hidingTimeout = 0;
    }
    this._setHidden(false);
  },
};
