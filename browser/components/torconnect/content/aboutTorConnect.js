// Copyright (c) 2021, The Tor Project, Inc.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// populated in AboutTorConnect.init()
let TorStrings = {};

const UIStates = Object.freeze({
  ConnectToTor: "ConnectToTor",
  Offline: "Offline",
  ConnectionAssist: "ConnectionAssist",
  CouldNotLocate: "CouldNotLocate",
  LocationConfirm: "LocationConfirm",
  FinalError: "FinalError",
});

const BreadcrumbStatus = Object.freeze({
  Hidden: "hidden",
  Disabled: "disabled",
  Default: "default",
  Active: "active",
});

/**
 * The controller for the about:torconnect page.
 */
class AboutTorConnect {
  selectors = Object.freeze({
    textContainer: {
      icon: "#tor-connect-icon",
      longContentText: "#connectLongContentText",
    },
    progress: {
      description: "p#connectShortDescText",
      meter: "div#progressBar",
    },
    breadcrumbs: {
      container: "#breadcrumbs",
      connectToTor: {
        link: "#connect-to-tor",
        label: "#connect-to-tor .breadcrumb-label",
      },
      connectionAssist: {
        link: "#connection-assist",
        label: "#connection-assist .breadcrumb-label",
      },
      tryBridge: {
        link: "#try-bridge",
        label: "#try-bridge .breadcrumb-label",
      },
    },
    viewLog: {
      button: "#viewLogButton",
    },
    quickstart: {
      container: "div#quickstartContainer",
      toggle: "#quickstartToggle",
    },
    buttons: {
      restart: "button#restartButton",
      configure: "button#configureButton",
      cancel: "button#cancelButton",
      connect: "button#connectButton",
      tryBridge: "button#tryBridgeButton",
      locationDropdownLabel: "#locationDropdownLabel",
      locationDropdown: "form#locationDropdown",
      locationDropdownSelect: "#regions-select",
    },
  });

  elements = Object.freeze({
    icon: document.querySelector(this.selectors.textContainer.icon),
    heading: document.getElementById("tor-connect-heading"),
    longContentText: document.querySelector(
      this.selectors.textContainer.longContentText
    ),
    progressDescription: document.querySelector(
      this.selectors.progress.description
    ),
    progressMeter: document.querySelector(this.selectors.progress.meter),
    breadcrumbContainer: document.querySelector(
      this.selectors.breadcrumbs.container
    ),
    connectToTorLink: document.querySelector(
      this.selectors.breadcrumbs.connectToTor.link
    ),
    connectToTorLabel: document.querySelector(
      this.selectors.breadcrumbs.connectToTor.label
    ),
    connectionAssistLink: document.querySelector(
      this.selectors.breadcrumbs.connectionAssist.link
    ),
    connectionAssistLabel: document.querySelector(
      this.selectors.breadcrumbs.connectionAssist.label
    ),
    tryBridgeLink: document.querySelector(
      this.selectors.breadcrumbs.tryBridge.link
    ),
    tryBridgeLabel: document.querySelector(
      this.selectors.breadcrumbs.tryBridge.label
    ),
    viewLogButton: document.querySelector(this.selectors.viewLog.button),
    quickstartContainer: document.querySelector(
      this.selectors.quickstart.container
    ),
    quickstartToggle: document.querySelector(this.selectors.quickstart.toggle),
    restartButton: document.querySelector(this.selectors.buttons.restart),
    configureButton: document.querySelector(this.selectors.buttons.configure),
    cancelButton: document.querySelector(this.selectors.buttons.cancel),
    connectButton: document.querySelector(this.selectors.buttons.connect),
    locationDropdownLabel: document.querySelector(
      this.selectors.buttons.locationDropdownLabel
    ),
    locationDropdown: document.querySelector(
      this.selectors.buttons.locationDropdown
    ),
    locationDropdownSelect: document.querySelector(
      this.selectors.buttons.locationDropdownSelect
    ),
    firstRegionOption: document.getElementById("first-region-option"),
    frequentRegionsOptionGroup: document.getElementById(
      "frequent-regions-option-group"
    ),
    fullRegionsOptionGroup: document.getElementById(
      "full-regions-option-group"
    ),
    tryBridgeButton: document.querySelector(this.selectors.buttons.tryBridge),
    restartProviderButton: document.getElementById("restart-provider-button"),
    restartProviderFailed: document.getElementById("restart-provider-failed"),
  });

  /**
   * The currently shown stage, or `null` if the page in uninitialised.
   *
   * @type {?string}
   */
  shownStage = null;

  /**
   * A promise that resolves to a list of region names and frequent regions, or
   * `null` if this needs to be re-fetched from the TorConnectParent.
   *
   * @type {?Promise<object>}
   */
  regions = null;

  /**
   * The option value that *should* be selected when the list of regions is
   * populated.
   *
   * @type {string}
   */
  selectedRegion = "";

  /**
   * Whether the user requested a cancellation of the bootstrap from *this*
   * page.
   *
   * @type {boolean}
   */
  userCancelled = false;

  /**
   * Start a normal bootstrap attempt.
   *
   * @param {boolean} userClickedConnect - Whether this request was triggered by
   *   the user clicking the "Connect" button on the "Start" page.
   */
  beginBootstrapping(userClickedConnect) {
    RPMSendAsyncMessage("torconnect:begin-bootstrapping", {
      userClickedConnect,
    });
  }

  /**
   * Start an auto bootstrap attempt.
   *
   * @param {string} regionCode - The region code to use for the bootstrap, or
   *   "automatic".
   */
  beginAutoBootstrapping(regionCode) {
    RPMSendAsyncMessage("torconnect:begin-bootstrapping", {
      regionCode,
    });
  }

  /**
   * Try and cancel the current bootstrap attempt.
   */
  cancelBootstrapping() {
    RPMSendAsyncMessage("torconnect:cancel-bootstrapping");
    this.userCancelled = true;
  }

  /*
  Element helper methods
  */

  show(element, primary = false) {
    element.classList.toggle("primary", primary);
    element.removeAttribute("hidden");
  }

  hide(element) {
    element.setAttribute("hidden", "true");
  }

  hideButtons() {
    this.hide(this.elements.quickstartContainer);
    this.hide(this.elements.restartButton);
    this.hide(this.elements.configureButton);
    this.hide(this.elements.cancelButton);
    this.hide(this.elements.connectButton);
    this.hide(this.elements.locationDropdownLabel);
    this.hide(this.elements.locationDropdown);
    this.hide(this.elements.tryBridgeButton);
  }

  setTitle(title, className) {
    this.elements.heading.textContent = title;
    this.elements.icon.className = "torconnect-icon";
    if (className) {
      this.elements.icon.classList.add(className);
    }
  }

  setLongText(...args) {
    this.elements.longContentText.textContent = "";
    this.elements.longContentText.append(...args);
  }

  setBreadcrumbsStatus(connectToTor, connectionAssist, tryBridge) {
    this.elements.breadcrumbContainer.classList.remove("hide-breadcrumbs");
    for (const { listEl, status } of [
      { listEl: this.elements.connectToTorLink, status: connectToTor },
      { listEl: this.elements.connectionAssistLink, status: connectionAssist },
      { listEl: this.elements.tryBridgeLink, status: tryBridge },
    ]) {
      let hidden = false;
      let disabled = false;
      let active = false;
      switch (status) {
        case BreadcrumbStatus.Hidden:
          hidden = true;
          break;
        case BreadcrumbStatus.Disabled:
          disabled = true;
          break;
        case BreadcrumbStatus.Active:
          active = true;
          break;
      }
      // NOTE: Hiding or disabling elements can cause them to loose focus.
      // However, with each stage change the focus should already be moved.
      listEl.hidden = hidden;
      const linkEl = listEl.querySelector(".breadcrumb-item");
      linkEl.classList.toggle("breadcrumb-disabled", disabled);
      linkEl.classList.toggle("breadcrumb-active", active);
      if (disabled) {
        linkEl.removeAttribute("tabindex");
        linkEl.setAttribute("aria-disabled", "true");
      } else {
        linkEl.setAttribute("tabindex", "0");
        linkEl.removeAttribute("aria-disabled", "true");
      }
      if (active) {
        linkEl.setAttribute("aria-current", "step");
      } else {
        linkEl.removeAttribute("aria-current");
      }
    }
  }

  hideBreadcrumbs() {
    this.elements.breadcrumbContainer.classList.add("hide-breadcrumbs");
  }

  getLocalizedStatus(status) {
    const aliases = {
      conn_dir: "conn",
      handshake_dir: "onehop_create",
      conn_or: "enough_dirinfo",
      handshake_or: "ap_conn",
    };
    if (status in aliases) {
      status = aliases[status];
    }
    return TorStrings.torConnect.bootstrapStatus[status] ?? status;
  }

  getMaybeLocalizedError(error) {
    switch (error.code) {
      case "Offline":
        return TorStrings.torConnect.offline;
      case "BootstrapError": {
        if (!error.phase || !error.reason) {
          return TorStrings.torConnect.torBootstrapFailed;
        }
        let status = this.getLocalizedStatus(error.phase);
        const reason =
          TorStrings.torConnect.bootstrapWarning[error.reason] ?? error.reason;
        return TorStrings.torConnect.bootstrapFailedDetails
          .replace("%1$S", status)
          .replace("%2$S", reason);
      }
      case "CannotDetermineCountry":
        return TorStrings.torConnect.cannotDetermineCountry;
      case "NoSettingsForCountry":
        return TorStrings.torConnect.noSettingsForCountry;
      case "AllSettingsFailed":
        return TorStrings.torConnect.autoBootstrappingAllFailed;
      case "ExternaError":
        // A standard JS error, or something for which we do probably do not
        // have a translation. Returning the original message is the best we can
        // do.
        return error.message;
      default:
        console.warn(`Unknown error code: ${error.code}`, error);
        return error.message || error.code;
    }
  }

  /**
   * The connect button that was focused just prior to a bootstrap attempt, if
   * any.
   *
   * @type {?Element}
   */
  preBootstrappingFocus = null;

  /**
   * The stage that was shown on this page just prior to a bootstrap attempt.
   *
   * @type {?string}
   */
  preBootstrappingStage = null;

  /*
  These methods update the UI based on the current TorConnect state
  */

  /**
   * Update the shown stage.
   *
   * @param {ConnectStage} stage - The new stage to show.
   * @param {boolean} [focusConnect=false] - Whether to try and focus the
   *   connect button, if we are in the Start stage.
   */
  updateStage(stage, focusConnect = false) {
    if (stage.name === this.shownStage) {
      return;
    }

    const prevStage = this.shownStage;
    this.shownStage = stage.name;
    // Make a request to change the selected region in the next call to
    // selectRegionOption.
    this.selectedRegion = stage.defaultRegion;

    let prevPageEl = document.querySelector(
      ".torconnect-stage-content.show-stage"
    );
    let pageEl = document.getElementById("connectPageContainer");
    for (const maybePageEl of document.querySelectorAll(
      ".torconnect-stage-content"
    )) {
      if (maybePageEl.dataset.stageName === stage.name) {
        pageEl = maybePageEl;
        break;
      }
    }
    // By default we want to reset the focus to the top of the page when
    // changing the displayed page since we want a user to read the new page
    // before activating a control.
    let moveFocus = pageEl.querySelector(".torconnect-heading");

    if (stage.name === "Bootstrapping") {
      this.preBootstrappingStage = prevStage;
      this.preBootstrappingFocus = null;
      if (focusConnect && stage.isQuickstart) {
        // If this is the initial automatic bootstrap triggered by the
        // quickstart preference, treat as if the previous shown stage was
        // "Start" and the user clicked the "Connect" button.
        // Then, if the user cancels, the focus should still move to the
        // "Connect" button.
        this.preBootstrappingStage = "Start";
        this.preBootstrappingFocus = this.elements.connectButton;
      } else if (this.elements.connectButton.contains(document.activeElement)) {
        this.preBootstrappingFocus = this.elements.connectButton;
      } else if (
        this.elements.tryBridgeButton.contains(document.activeElement)
      ) {
        this.preBootstrappingFocus = this.elements.tryBridgeButton;
      }
    } else {
      if (
        this.userCancelled &&
        prevStage === "Bootstrapping" &&
        stage.name === this.preBootstrappingStage &&
        this.preBootstrappingFocus &&
        this.elements.cancelButton.contains(document.activeElement)
      ) {
        // If returning back to the same stage after the user tried to cancel
        // bootstrapping from within this page, then we restore the focus to the
        // connect button to allow the user to quickly re-try.
        // If the bootstrap was cancelled for any other reason, we reset the
        // focus as usual.
        moveFocus = this.preBootstrappingFocus;
      }
      // Clear the Bootstrapping variables.
      this.preBootstrappingStage = null;
      this.preBootstrappingFocus = null;
    }

    // Clear the recording of the cancellation request.
    this.userCancelled = false;

    let isLoaded = true;
    let showProgress = false;
    let showLog = false;
    switch (stage.name) {
      case "Disabled":
        console.error("Should not be open when TorConnect is disabled");
        break;
      case "Loading":
        // Unexpected for this page to open so early.
        console.warn("Page opened whilst loading");
        isLoaded = false;
        break;
      case "ProviderStopped":
        this.hideBreadcrumbs();
        this.updateProviderStatus(stage.providerStatus, true);
        break;
      case "Start":
        this.showStart(stage.tryAgain, stage.potentiallyBlocked);
        if (focusConnect) {
          moveFocus = this.elements.connectButton;
        }
        break;
      case "Bootstrapping":
        showProgress = true;
        this.showBootstrapping(
          stage.bootstrapTrigger,
          stage.tryAgain,
          stage.potentiallyBlocked
        );
        // Always focus the cancel button.
        moveFocus = this.elements.cancelButton;
        break;
      case "Offline":
        showLog = true;
        this.showOffline();
        break;
      case "ChooseRegion":
        showLog = true;
        this.showChooseRegion(stage.error);
        break;
      case "RegionNotFound":
        showLog = true;
        this.showRegionNotFound();
        break;
      case "ConfirmRegion":
        showLog = true;
        this.showConfirmRegion(stage.error);
        break;
      case "FinalError":
        showLog = true;
        this.showFinalError(stage.error);
        break;
      case "Bootstrapped":
        showProgress = true;
        this.showBootstrapped();
        break;
      default:
        console.error(`Unknown stage ${stage.name}`);
        break;
    }

    if (showProgress) {
      this.show(this.elements.progressMeter);
    } else {
      this.hide(this.elements.progressMeter);
    }

    this.updateBootstrappingStatus(stage.bootstrappingStatus);

    if (showLog) {
      this.show(this.elements.viewLogButton);
    } else {
      this.hide(this.elements.viewLogButton);
    }

    prevPageEl?.classList.remove("show-stage");
    pageEl.classList.add("show-stage");
    document.body.classList.toggle("loaded", isLoaded);
    moveFocus.focus();
  }

  /**
   * The stored data for the `ProviderStopped` page content. `null` whilst
   * uninitialized.
   *
   * @type {object}
   */
  _providerStoppedData = null;

  /**
   * Called whenever the tor provider status may have changed. Only relevant in
   * the `ProviderStopped` stage.
   *
   * @param {ProviderStatus} providerStatus - The latest provider status.
   * @param {boolean} enteringStage - Whether this is being called because we
   *   have just entered the `ProviderStopped` stage as the initial stage of the
   *   page, or transitioning from another stage.
   */
  updateProviderStatus(providerStatus, enteringStage) {
    if (this.shownStage !== "ProviderStopped") {
      // Ignore.
      return;
    }
    if (enteringStage) {
      // Reset the data:
      // + When entering the stage.
      // + When opening the page in this initial stage.
      this._providerStoppedData = {
        // Set to `null` to make sure we don't early exit below.
        isRestarting: null,
        forceShowRestarting: false,
        canShowFailedAlert: false,
        // We set the maybeConfigIssue when entering the stage, but do not attempt
        // to update it between failed attempts.
        maybeConfigIssue: providerStatus.maybeConfigIssue,
      };
    }

    // We show the UI as restarting if the state is not "Stopped", which
    // includes both "Starting" and "Running". In the latter case (which is
    // unexpected) we still want to show the state as loading before we enter a
    // different shownStage.
    const restarting = providerStatus.state !== "Stopped";

    const data = this._providerStoppedData;

    if (restarting === data.isRestarting) {
      // No change in the recorded state. Don't change the UI or adjust any
      // timeouts.
      return;
    }

    data.isRestarting = restarting;
    if (restarting) {
      // Whilst we remain in this `ProviderStopped` stage, we will now show the
      // failed alert when we re-enter the "Stopped" state.
      data.canShowFailedAlert = true;

      if (!data.forceShowRestarting) {
        // Force the restarting UI to show for at least a certain amount of
        // time. Even if we fail early, we continue to show the restarting UI
        // for a short time to make sure the user has enough time to visually
        // see the restarting UI before we return to the error state.
        data.forceShowRestarting = true;
        setTimeout(() => {
          if (
            data !== this._providerStoppedData ||
            this.shownStage !== "ProviderStopped"
          ) {
            // We left or re-entered this stage whilst waiting. Do nothing.
            return;
          }
          data.forceShowRestarting = false;
          this.updateProviderStoppedContent();
        }, 1000);
      }
      // Else, we already have a pending timeout.
      // E.g., consider the following timeline, where "R" represents entering
      // the "Starting" state, "S" represents entering the "Stopped" state,
      // "T0" represents the start of the forcedShowRestarting timeout, and "T1"
      // represents the completion:
      //
      // |--------------+-----+-----+-----+----+------------------>
      // S0             R0    S1    R1    S2
      //                T0                     T1
      // |--stopped UI--|--restarting UI-------|--stopped UI-------
      //
      //
      // Similarly with S2 delayed:
      //
      // |--------------+-----+-----+----------+----+------------->
      // S0             R0    S1    R1              S2
      //                T0                     T1
      // |--stopped UI--|--restarting UI------------|--stopped UI--
      //
      //
      // I.e. we restart once (R0) which starts the timeout. Then we stop (S1) and
      // restart (R1) again (though unlikely, this could happen if the user
      // opens another `about:torconnect` tab after S1 and presses the restart
      // button). Rather than create a new timeout, which would delay the
      // restart display for even longer, we maintain the current one.
    }

    this.updateProviderStoppedContent();
  }

  /**
   * Whether we are currently *showing* the tor provider as restarting in the
   * UI.
   *
   * @type {boolean}
   */
  restartingProvider = false;

  /**
   * Called to update the page content for the `ProviderStopped` stage.
   */
  updateProviderStoppedContent() {
    const {
      isRestarting,
      forceShowRestarting,
      canShowFailedAlert,
      maybeConfigIssue,
    } = this._providerStoppedData;

    // Hide any of the debugging information related to configuration issues if
    // we do not suspect that this could be the cause.
    document.body.classList.toggle(
      "hide-provider-config-debugging",
      !maybeConfigIssue
    );

    const showRestarting = isRestarting || forceShowRestarting;
    this.elements.restartProviderFailed.hidden =
      showRestarting || !canShowFailedAlert;

    this.restartingProvider = showRestarting;
    // The button is functionally disabled, but can still keep focus.
    // TODO: tor-browser#45050. Disabled attribute is waiting on bugzilla bug
    // 1927727.
    // this.elements.restartProviderButton.disabled = showRestarting;
    this.elements.restartProviderButton.iconSrc = showRestarting
      ? "chrome://global/skin/icons/loading.svg"
      : null;
    document.l10n.setAttributes(
      this.elements.restartProviderButton,
      showRestarting
        ? "tor-connect-tor-not-working-restarting-button"
        : "tor-connect-tor-not-working-restart-button"
    );
  }

  updateBootstrappingStatus(data) {
    this.elements.progressMeter.style.setProperty(
      "--progress-percent",
      `${data.progress}%`
    );
    if (this.shownStage === "Bootstrapping" && data.hasWarning) {
      // When bootstrapping starts, we hide the log button, but we re-show it if
      // we get a warning.
      this.show(this.elements.viewLogButton);
    }
  }

  updateQuickstart(enabled) {
    this.elements.quickstartToggle.pressed = enabled;
  }

  showBootstrapped() {
    this.setTitle(TorStrings.torConnect.torConnected, "");
    this.setLongText(TorStrings.settings.torPreferencesDescription);
    this.elements.progressDescription.textContent = "";
    this.hideButtons();
  }

  showStart(tryAgain, potentiallyBlocked) {
    this.setTitle(TorStrings.torConnect.torConnect, "");
    this.setLongText(TorStrings.settings.torPreferencesDescription);
    this.elements.progressDescription.textContent = "";
    this.hideButtons();
    this.show(this.elements.quickstartContainer);
    this.show(this.elements.configureButton);
    this.show(this.elements.connectButton, true);
    this.elements.connectButton.textContent = tryAgain
      ? TorStrings.torConnect.tryAgain
      : TorStrings.torConnect.torConnectButton;
    if (potentiallyBlocked) {
      this.setBreadcrumbsStatus(
        BreadcrumbStatus.Active,
        BreadcrumbStatus.Default,
        BreadcrumbStatus.Disabled
      );
    } else {
      this.hideBreadcrumbs();
    }
  }

  showBootstrapping(trigger, tryAgain, potentiallyBlocked) {
    let title = "";
    let description = "";
    const breadcrumbs = [
      BreadcrumbStatus.Default,
      potentiallyBlocked ? BreadcrumbStatus.Default : BreadcrumbStatus.Hidden,
      potentiallyBlocked ? BreadcrumbStatus.Disabled : BreadcrumbStatus.Hidden,
    ];
    switch (trigger) {
      case "Start":
      case "Offline":
        breadcrumbs[0] = BreadcrumbStatus.Active;
        title = tryAgain
          ? TorStrings.torConnect.tryAgain
          : TorStrings.torConnect.torConnecting;
        description = TorStrings.settings.torPreferencesDescription;
        break;
      case "ChooseRegion":
        breadcrumbs[2] = BreadcrumbStatus.Active;
        title = TorStrings.torConnect.tryingBridge;
        description = TorStrings.torConnect.assistDescription;
        break;
      case "RegionNotFound":
        breadcrumbs[2] = BreadcrumbStatus.Active;
        title = TorStrings.torConnect.tryingBridgeAgain;
        description = TorStrings.torConnect.errorLocationDescription;
        break;
      case "ConfirmRegion":
        breadcrumbs[2] = BreadcrumbStatus.Active;
        title = TorStrings.torConnect.tryingBridgeAgain;
        description = TorStrings.torConnect.isLocationCorrectDescription;
        break;
      default:
        console.warn("Unrecognized bootstrap trigger", trigger);
        break;
    }
    this.setTitle(title, "");
    this.showConfigureConnectionLink(description);
    this.elements.progressDescription.textContent = "";
    if (tryAgain || potentiallyBlocked) {
      this.setBreadcrumbsStatus(...breadcrumbs);
    } else {
      this.hideBreadcrumbs();
    }
    this.hideButtons();
    this.show(this.elements.cancelButton);
  }

  showOffline() {
    this.setTitle(TorStrings.torConnect.noInternet, "offline-icon");
    this.setLongText(TorStrings.torConnect.noInternetDescription);
    this.elements.progressDescription.textContent =
      TorStrings.torConnect.offline;
    this.setBreadcrumbsStatus(
      BreadcrumbStatus.Default,
      BreadcrumbStatus.Active,
      BreadcrumbStatus.Hidden
    );
    this.hideButtons();
    this.show(this.elements.configureButton);
    this.show(this.elements.connectButton, true);
    this.elements.connectButton.textContent = TorStrings.torConnect.tryAgain;
  }

  showChooseRegion(error) {
    this.setTitle(TorStrings.torConnect.couldNotConnect, "assist-icon");
    this.showConfigureConnectionLink(TorStrings.torConnect.assistDescription);
    this.elements.progressDescription.textContent =
      this.getMaybeLocalizedError(error);
    this.setBreadcrumbsStatus(
      BreadcrumbStatus.Default,
      BreadcrumbStatus.Active,
      BreadcrumbStatus.Disabled
    );
    this.showLocationForm(true, TorStrings.torConnect.tryBridge);
  }

  showRegionNotFound() {
    this.setTitle(TorStrings.torConnect.errorLocation, "location-icon");
    this.showConfigureConnectionLink(
      TorStrings.torConnect.errorLocationDescription
    );
    this.elements.progressDescription.textContent =
      TorStrings.torConnect.cannotDetermineCountry;
    this.setBreadcrumbsStatus(
      BreadcrumbStatus.Default,
      BreadcrumbStatus.Active,
      BreadcrumbStatus.Disabled
    );
    this.showLocationForm(false, TorStrings.torConnect.tryBridge);
  }

  showConfirmRegion(error) {
    this.setTitle(TorStrings.torConnect.isLocationCorrect, "location-icon");
    this.showConfigureConnectionLink(
      TorStrings.torConnect.isLocationCorrectDescription
    );
    this.elements.progressDescription.textContent =
      this.getMaybeLocalizedError(error);
    this.setBreadcrumbsStatus(
      BreadcrumbStatus.Default,
      BreadcrumbStatus.Active,
      BreadcrumbStatus.Disabled
    );
    this.showLocationForm(false, TorStrings.torConnect.tryAgain);
  }

  showFinalError(error) {
    this.setTitle(TorStrings.torConnect.finalError, "assist-icon");
    this.setLongText(TorStrings.torConnect.finalErrorDescription);
    this.elements.progressDescription.textContent =
      this.getMaybeLocalizedError(error);
    this.setBreadcrumbsStatus(
      BreadcrumbStatus.Default,
      BreadcrumbStatus.Default,
      BreadcrumbStatus.Active
    );
    this.hideButtons();
    this.show(this.elements.restartButton);
    this.show(this.elements.configureButton, true);
  }

  showConfigureConnectionLink(text) {
    const pieces = text.split("%S");
    const link = document.createElement("a");
    link.textContent = TorStrings.torConnect.configureConnection;
    link.setAttribute("href", "#");
    link.addEventListener("click", e => {
      e.preventDefault();
      RPMSendAsyncMessage("torconnect:open-tor-preferences");
    });
    if (pieces.length > 1) {
      const first = pieces.shift();
      this.setLongText(first, link, ...pieces);
    } else {
      this.setLongText(text);
    }
  }

  /**
   * Try and select the region specified in `selectedRegion`.
   */
  selectRegionOption() {
    // NOTE: If the region appears in both the frequent list and the full list,
    // then this will select the region option in
    // frequentRegionsOptionGroup, even if the user had prior selected the
    // option from fullRegionsOptionGroup. But the overall value should be the
    // same.
    this.elements.locationDropdownSelect.value = this.selectedRegion;
    if (this.elements.locationDropdownSelect.selectedIndex === -1) {
      // Select the first, as a fallback. E.g. in RegionNotFound the
      // selectedRegion may still be "automatic", but this is no longer
      // available.
      this.elements.locationDropdownSelect.selectedIndex = 0;
    }
    this.validateRegion();
  }

  /**
   * Ensure that the current selected region is valid for the shown stage.
   */
  validateRegion() {
    this.elements.tryBridgeButton.toggleAttribute(
      "disabled",
      !this.elements.locationDropdownSelect.value
    );
  }

  /**
   * Populate the full list of regions, if necessary.
   */
  async populateDelayedRegionOptions() {
    if (this.regions) {
      // Already populated, or about to populate.
      return;
    }

    this.regions = RPMSendQuery("torconnect:get-regions");
    const regions = this.regions;
    const { names, frequent } = await regions;

    if (regions !== this.regions) {
      // Replaced by a new call.
      return;
    }

    this.setRegionOptions(
      this.elements.frequentRegionsOptionGroup,
      frequent.map(code => [code, names[code]])
    );

    this.setRegionOptions(
      this.elements.fullRegionsOptionGroup,
      Object.entries(names)
    );

    // Now that the list has been re-populated we want to re-select the
    // requested region.
    this.selectRegionOption();
  }

  /**
   * Set the shown region options.
   *
   * @param {HTMLOptGroupElement} group - The group to set the children of.
   * @param {[string, string|undefined][]} regions - The list of region
   *   key-value entries to fill the group with. The key is the region code and
   *   the value is the region's localised name.
   */
  setRegionOptions(group, regions) {
    const regionNodes = regions
      .sort(([_code1, name1], [_code2, name2]) => name1.localeCompare(name2))
      .map(([code, name]) => {
        const option = document.createElement("option");
        option.value = code;
        // If the name is unexpectedly empty or undefined we use the code
        // instead.
        option.textContent = name || code;
        return option;
      });
    group.replaceChildren(...regionNodes);
  }

  showLocationForm(isChoose, buttonLabel) {
    this.hideButtons();

    this.elements.firstRegionOption.textContent = isChoose
      ? TorStrings.torConnect.automatic
      : TorStrings.torConnect.selectCountryRegion;
    this.elements.firstRegionOption.value = isChoose ? "automatic" : "";

    // Try and select the region now, prior to waiting for
    // populateDelayedRegionOptions.
    this.selectRegionOption();

    // Async fill the rest of the region options, if needed.
    this.populateDelayedRegionOptions();

    this.show(this.elements.locationDropdownLabel);
    this.show(this.elements.locationDropdown);
    this.elements.locationDropdownLabel.classList.toggle("error", !isChoose);
    this.show(this.elements.tryBridgeButton, true);
    if (buttonLabel !== undefined) {
      this.elements.tryBridgeButton.textContent = buttonLabel;
    }
  }

  initElements(direction) {
    document.documentElement.setAttribute("dir", direction);

    for (const [linkEl, command] of [
      [
        this.elements.connectToTorLink,
        () => {
          RPMSendAsyncMessage("torconnect:start-again");
        },
      ],
      [
        this.elements.connectionAssistLink,
        () => {
          RPMSendAsyncMessage("torconnect:choose-region");
        },
      ],
      // Final link does nothing.
    ]) {
      // NOTE: These link should never be both visible and disabled.
      // NOTE: We allow re-selecting this link when it is already the
      // "aria-current" item, to allow the user to reset to the beginning of
      // Connection Assist.
      linkEl.addEventListener("click", () => {
        command();
      });
      linkEl.addEventListener("keydown", event => {
        if (event.key !== "Enter") {
          return;
        }
        command();
      });
    }

    this.elements.connectToTorLabel.textContent =
      TorStrings.torConnect.torConnect;
    this.elements.connectionAssistLabel.textContent =
      TorStrings.torConnect.breadcrumbAssist;
    this.elements.tryBridgeLabel.textContent =
      TorStrings.torConnect.breadcrumbTryBridge;

    this.hide(this.elements.viewLogButton);
    this.elements.viewLogButton.textContent = TorStrings.torConnect.viewLog;
    this.elements.viewLogButton.addEventListener("click", () => {
      RPMSendAsyncMessage("torconnect:view-tor-logs");
    });

    this.elements.quickstartToggle.addEventListener("toggle", () => {
      const quickstart = this.elements.quickstartToggle.pressed;
      RPMSendAsyncMessage("torconnect:set-quickstart", quickstart);
    });
    this.elements.quickstartToggle.setAttribute(
      "label",
      TorStrings.settings.quickstartCheckbox
    );

    this.elements.restartButton.textContent =
      TorStrings.torConnect.restartTorBrowser;
    this.elements.restartButton.addEventListener("click", () => {
      RPMSendAsyncMessage("torconnect:restart");
    });

    this.elements.restartProviderButton.addEventListener("click", () => {
      if (this.restartingProvider) {
        // Ignore clicks whilst the button is shown as "Restarting".
        return;
      }
      RPMSendAsyncMessage("torconnect:restart-provider");
    });

    this.elements.configureButton.textContent =
      TorStrings.torConnect.torConfigure;
    this.elements.configureButton.addEventListener("click", () => {
      RPMSendAsyncMessage("torconnect:open-tor-preferences");
    });

    this.elements.cancelButton.textContent = TorStrings.torConnect.cancel;
    this.elements.cancelButton.addEventListener("click", () => {
      this.cancelBootstrapping();
    });

    this.elements.connectButton.textContent =
      TorStrings.torConnect.torConnectButton;
    this.elements.connectButton.addEventListener("click", () => {
      // Record as userClickedConnect if we are in the Start stage.
      this.beginBootstrapping(this.shownStage === "Start");
    });

    this.elements.locationDropdownSelect.addEventListener("change", () => {
      // Overwrite the stage requested selectedRegion.
      // NOTE: This should not fire in response to a programmatic change in
      // value.
      // E.g. if the user selects a region, then changes locale, we want the
      // same region to be re-selected after the option list is rebuilt.
      this.selectedRegion = this.elements.locationDropdownSelect.value;

      this.validateRegion();
    });

    this.elements.locationDropdownLabel.textContent =
      TorStrings.torConnect.unblockInternetIn;

    this.elements.frequentRegionsOptionGroup.setAttribute(
      "label",
      TorStrings.torConnect.frequentLocations
    );
    this.elements.fullRegionsOptionGroup.setAttribute(
      "label",
      TorStrings.torConnect.otherLocations
    );

    this.elements.tryBridgeButton.textContent = TorStrings.torConnect.tryBridge;
    this.elements.tryBridgeButton.addEventListener("click", () => {
      const value = this.elements.locationDropdownSelect.value;
      if (value) {
        this.beginAutoBootstrapping(value);
      }
    });

    // Prevent repeat triggering on keydown when the Enter key is held down.
    //
    // Without this, holding down Enter will continue to trigger the button's
    // click event until the user stops holding. This means that a user can
    // accidentally re-trigger a button several times. And if focus moves to a
    // new button it can also get triggered, despite not receiving the initial
    // keydown event.
    //
    // E.g. If the user presses down Enter on the "Connect" button it will
    // trigger and focus will move to the "Cancel" button. This should prevent
    // the user accidentally triggering the "Cancel" button if they hold down
    // Enter for a little bit too long.
    for (const button of document.body.querySelectorAll("button")) {
      button.addEventListener("keydown", event => {
        // If the keydown is a repeating Enter event, ignore it.
        // NOTE: If firefox uses wayland display (rather than xwayland), the
        // "repeat" event is always "false" so this will not work.
        // See bugzilla bug 1784438. Also see bugzilla bug 1594003.
        // Currently tor browser uses xwayland by default on linux.
        if (event.key === "Enter" && event.repeat) {
          event.preventDefault();
        }
      });
    }
  }

  initObservers() {
    // TorConnectParent feeds us state blobs to we use to update our UI
    RPMAddMessageListener("torconnect:stage-change", ({ data }) => {
      this.updateStage(data);
    });
    RPMAddMessageListener("torconnect:provider-status-change", ({ data }) => {
      this.updateProviderStatus(data, false);
    });
    RPMAddMessageListener("torconnect:bootstrap-progress", ({ data }) => {
      this.updateBootstrappingStatus(data);
    });
    RPMAddMessageListener("torconnect:quickstart-change", ({ data }) => {
      this.updateQuickstart(data);
    });
    RPMAddMessageListener("torconnect:region-names-change", () => {
      // Reset the regions list.
      this.regions = null;
      if (!this.elements.locationDropdown.hidden) {
        // Re-populate immediately.
        this.populateDelayedRegionOptions();
      }
      // Else, wait until we show the region select to re-populate.
    });
  }

  initKeyboardShortcuts() {
    document.onkeydown = evt => {
      // unfortunately it looks like we still haven't standardized keycodes to
      // integers, so we must resort to a string compare here :(
      // see https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/code for relevant documentation
      if (evt.code === "Escape") {
        this.cancelBootstrapping();
      }
    };
  }

  async init() {
    let args = await RPMSendQuery("torconnect:get-init-args");

    // various constants
    TorStrings = Object.freeze(args.TorStrings);

    this.initElements(args.Direction);
    this.initObservers();
    this.initKeyboardShortcuts();

    // If we have previously opened about:torconnect and the user tried the
    // "Connect" button we want to focus the "Connect" button for easy
    // activation.
    // Otherwise, we do not want to focus it for first time users so they can
    // read the full page first.
    const focusConnect = args.userHasEverClickedConnect;
    this.updateStage(args.stage, focusConnect);
    this.updateQuickstart(args.quickstartEnabled);
  }
}

const aboutTorConnect = new AboutTorConnect();
aboutTorConnect.init();
