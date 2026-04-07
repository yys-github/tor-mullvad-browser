/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

const lazy = {};
ChromeUtils.defineESModuleGetters(lazy, {
  TorLauncherUtil:
    "moz-src:///toolkit/components/tor-launcher/TorLauncherUtil.sys.mjs",
  TorProvider: "moz-src:///toolkit/components/tor-launcher/TorProvider.sys.mjs",
  TorProviderNone:
    "moz-src:///toolkit/components/tor-launcher/TorProviderNone.sys.mjs",
});

ChromeUtils.defineLazyGetter(lazy, "logger", () => {
  return console.createInstance({
    // Share preference with TorProvider.
    maxLogLevelPref: "browser.tor_provider.log_level",
    prefix: "TorProviderBuilder",
  });
});

export const TorProviderTopics = Object.freeze({
  ProviderStateChanged: "TorProviderStateChanged",
  BootstrapStatus: "TorBootstrapStatus",
  BootstrapError: "TorBootstrapError",
  TorLog: "TorLog",
  HasWarnOrErr: "TorLogHasWarnOrErr",
  BridgeChanged: "TorBridgeChanged",
  CircuitCredentialsMatched: "TorCircuitCredentialsMatched",
});

/**
 * The tracked state a provider might be in.
 */
export const TorProviderState = Object.freeze({
  Starting: "Starting",
  Running: "Running",
  Stopped: "Stopped",
});

/**
 * Wrapper error class for errors raised during TorProvider.init.
 */
export class TorProviderInitError extends Error {
  /**
   * Create a new instance.
   *
   * @param {any} error - The raised error that we want to wrap.
   */
  constructor(error) {
    super(error?.message, { cause: error });
    this.name = "TorProviderInitError";
  }
}

/**
 * Bootstrap errors raised by the TorProvider.
 */
export class TorBootstrapError extends Error {
  /**
   * Create a new instance.
   *
   * @param {object} details - Details about the error.
   * @param {string} details.summary - A summary of the error.
   * @param {string} details.phase - The bootstrap phase when the error occured.
   * @param {string} details.reason - The reason for the bootsrap failure.
   */
  constructor(details) {
    super(details.summary);
    this.name = "TorBootstrapError";
    this.phase = details.phase;
    this.reason = details.reason;
  }
}

export const TorProviders = Object.freeze({
  none: "none",
  tor: "tor",
});

/**
 * @typedef {object} LogEntry An object with a log message
 * @property {string} timestamp The local date-time stamp at which we received the message
 * @property {string} type The message level
 * @property {string} msg The message
 */

/**
 * @typedef {object} TorProviderData
 *
 * The data associated with a tor provider.
 *
 * @property {TorProviderBase} provider - The provider instance.
 * @property {Promise<undefined>} initPromise - A promise that fulfils after the
 *   previous provider is cleaned up and the new provider's initialization
 *   completes or throws an error.
 */

/**
 * The factory to get a Tor provider.
 * Currently we support only TorProvider, i.e., the one that interacts with
 * C-tor through the control port protocol.
 */
export class TorProviderBuilder {
  /**
   * Data about the current provider instance.
   *
   * @type {?TorProviderData}
   */
  static #providerData = null;

  /**
   * Whether the `uninit` method has been called.
   *
   * @type {boolean}
   */
  static #uninitialized = false;

  /**
   * Check that we are active before a public call.
   *
   * @throws {Error} Throws if we are not active.
   */
  static #checkActive() {
    if (this.#uninitialized) {
      throw new Error("TorProviderBuilder has already been uninitialized.");
    }
    if (!this.#providerData) {
      throw new Error("TorProviderBuilder has not been initialized.");
    }
  }

  /**
   * A record of the log messages from all provider instances.
   *
   * @type {LogEntry[]}
   */
  static #log = [];

  /**
   * Get a record of historic log entries.
   *
   * @returns {LogEntry[]} - The record of entries.
   */
  static getLog() {
    return structuredClone(this.#log);
  }

  /**
   * The limit on the number of log entries we should store.
   *
   * @type {integer}
   */
  static #logLimit;

  /**
   * The observer that checks for new TorLog messages.
   *
   * @type {Function}
   */
  static #logObserver;

  /**
   * Add a new log message.
   *
   * @param {LogEntry} logEntry - The log entry to add.
   */
  static #addLogEntry(logEntry) {
    if (this.#logLimit > 0 && this.#log.length >= this.#logLimit) {
      this.#log.splice(0, 1);
    }
    this.#log.push(logEntry);
  }

  /**
   * Initialize the provider of choice.
   */
  static init() {
    this.#logLimit = Services.prefs.getIntPref(
      "extensions.torlauncher.max_tor_log_entries",
      1000
    );
    this.#logObserver = subject => {
      this.#addLogEntry(subject.wrappedJSObject);
    };
    Services.obs.addObserver(this.#logObserver, TorProviderTopics.TorLog);

    // Even though initialization of the initial provider is asynchronous, we do
    // not expect the caller to await it. The reason is that any call to build()
    // will wait the initialization anyway (and re-throw any initialization
    // error).
    this.#replaceProvider();
  }

  /**
   * Replace the provider with a new instance.
   */
  static #replaceProvider() {
    // NOTE: We need to ensure that the #providerData is set as soon as
    // TorProviderBuilder.init is called.
    // I.e. it should be safe to call
    //   TorProviderBuilder.init();
    //   TorProviderBuilder.build();
    //   TorProviderBuilder.settledState();
    //   // etc
    // without any await.
    //
    // In particular, this is needed by `TorConnect.init`, which will call
    // `settledState`. It will also call `build` immediately if quickstart is
    // set. See tor-browser#41921.
    if (this.#providerData) {
      lazy.logger.info(
        `Replacing the provider with a "${this.providerType}" provider.`
      );
    } else {
      lazy.logger.info(`Creating the initial "${this.providerType}" provider.`);
    }

    let providerClass;
    switch (this.providerType) {
      case TorProviders.tor:
        providerClass = lazy.TorProvider;
        break;
      case TorProviders.none:
        providerClass = lazy.TorProviderNone;
        break;
      default:
        lazy.logger.error(`Unknown tor provider ${this.providerType}.`);
        break;
    }
    // NOTE: It should be safe to create another provider instance whilst the
    // existing one is still active. However, we will wait until the other is
    // uninitialized before we initialize the new one.
    const provider = new providerClass(() => {
      this.#notifyStateChanged(provider);
    });
    const prevProviderData = this.#providerData;
    // NOTE: We want `#providerData` to be set prior to our call to
    // `provider.init`, so we create the `initPromise` prior to setting it.
    const { promise: initPromise, resolve, reject } = Promise.withResolvers();
    this.#providerData = { provider, initPromise };
    // Let observers know we are restarting the provider.
    this.#notifyStateChanged(provider);

    // Run the rest of the init in an async operation that will cause
    // `initPromise` to settle.
    // NOTE: `#cleanupProviderData` should not throw, unlike `provider.init()`,
    // which may throw.
    // NOTE: We wait for `#cleanupProviderData` to complete before calling
    // `provider.init()` in case the implementation relies on this.
    this.#cleanupProviderData(prevProviderData).finally(() => {
      provider.init().then(resolve, reject);
    });
  }

  /**
   * Notify any listeners that the state of the current provider has changed.
   *
   * @param {TorProviderBase} provider - The provider who's state has changed.
   */
  static #notifyStateChanged(provider) {
    if (this.#uninitialized) {
      // Do not signal the final state changes when we uninitialize.
      return;
    }
    if (provider !== this.#providerData.provider) {
      // Delayed call from an old provider. Ignore.
      return;
    }

    Services.obs.notifyObservers(
      null,
      TorProviderTopics.ProviderStateChanged,
      provider.state
    );

    this.#promptProviderState(false);
  }

  /**
   * Check the given provider's state.
   *
   * If the provider is no longer the current one, it is considered to be
   * "Stopped".
   *
   * If it is still the current provider, the state will be updated and
   * returned.
   *
   * @param {TorProviderBase} provider - The provider to check.
   * @returns {string} - The `TorProviderState` state for the provider.
   */
  static #checkProviderState(provider) {
    if (this.#providerData?.provider !== provider) {
      // Replaced.
      lazy.logger.debug("The checked provider has been replaced.");
      return TorProviderState.Stopped;
    }
    return this.#providerData.provider.state;
  }

  /**
   * Cleanup the given provider data.
   *
   * @param {?TorProviderData} providerData - The data to clean up.
   */
  static async #cleanupProviderData(providerData) {
    if (!providerData) {
      return;
    }
    try {
      await providerData.initPromise;
    } catch {}

    // Call `uninit` to clean up, even if `init` threw.
    // Should be safe to call more than once (via `uninit`).
    try {
      await providerData.provider.uninit();
    } catch (error) {
      lazy.logger.error("Error in uninitializing provider", error);
    }
  }

  static uninit() {
    this.#uninitialized = true;

    // NOTE: `uninit` should not be followed by any further calls to public
    // methods. So we can clear the `#providerData` without keeping it for any
    // future provider instances to wait on.
    const providerData = this.#providerData;
    this.#providerData = null;
    this.#cleanupProviderData(providerData);

    Services.obs.removeObserver(this.#logObserver, TorProviderTopics.TorLog);
  }

  /**
   * Request the current instance of the Tor provider.
   *
   * This method will wait for the system to be initialized before returning the
   * provider.
   *
   * This will throw any initialization errors of the provider, if it had any.
   * This will also throw if the provider is no longer active.
   *
   * @returns {TorProvider} A TorProvider instance
   */
  static async build() {
    this.#checkActive();
    if (this.#providerData.provider instanceof lazy.TorProviderNone) {
      throw new Error(
        "Tor Browser has been configured to use only the proxy functionalities."
      );
    }

    const { provider, initPromise } = this.#providerData;
    // initPromise may throw.
    await initPromise;
    if (this.#checkProviderState(provider) !== TorProviderState.Running) {
      lazy.logger.warn("Request was made for a provider that has stopped.");
      // TODO: Wait for the new instance instead?
      throw new TorProviderInitError(
        new Error("Provider is no longer active.")
      );
    }
    return provider;
  }

  /**
   * Get the state of the current provider instance. Waits until the provider
   * has finished initialisation first.
   *
   * If the provider has been replaced, the Stopped state will be returned.
   *
   * @returns {string} - The `TorProviderState` state for the provider that
   *   existed when this method was called.
   */
  static async settledState() {
    this.#checkActive();
    const { provider, initPromise } = this.#providerData;
    try {
      await initPromise;
    } catch {}
    return this.#checkProviderState(provider);
  }

  /**
   * Replace the current provider instance with a new provider.
   */
  static replace() {
    this.#checkActive();
    this.#replaceProvider();
  }

  // TODO: Remove firstWindowLoaded, #uiReady, #prompting, #promptProviderState
  // and use TorConnect instead. tor-browser#43570.
  /**
   * Check if the provider has been succesfully initialized when the first
   * browser window is shown.
   * This is a workaround we need because ideally we would like the tor process
   * to start as soon as possible, to avoid delays in the about:torconnect page,
   * but we should modify TorConnect and about:torconnect to handle this case
   * there with a better UX.
   */
  static firstWindowLoaded() {
    this.#promptProviderState(true);
  }

  /**
   * Tell whether the browser UI is ready.
   * We ignore any errors until it is because we cannot show them.
   *
   * @type {boolean}
   */
  static #uiReady = false;

  /**
   * Whether we are prompting the user for a restart of the provider.
   *
   * @type {boolean}
   */
  static #prompting = false;

  /**
   * Prompt the user to restart the provider, if this is necessary.
   *
   * @param {boolean} uiReady - Whether this is being called for the first time
   *   when the UI is ready.
   */
  static async #promptProviderState(uiReady) {
    if (uiReady) {
      this.#uiReady = true;
    }
    if (this.#providerData.provider.state === TorProviderState.Running) {
      // Nothing to wait for.
      return;
    }
    if (!this.#uiReady) {
      lazy.logger.warn(
        "Seen exit, but not doing anything because the UI is not ready yet."
      );
      return;
    }
    if (this.#prompting) {
      // Already prompting, so don't duplicate.
      return;
    }

    this.#prompting = true;
    let waitForInit = uiReady;
    let retry = true;
    try {
      while (retry) {
        if (waitForInit) {
          try {
            await this.#providerData.initPromise;
          } catch {}
        }
        if (
          this.#providerData.provider.state === TorProviderState.Stopped &&
          lazy.TorLauncherUtil.showRestartPrompt(uiReady)
        ) {
          waitForInit = true;
          this.replace();
        } else {
          retry = false;
        }
      }
    } finally {
      this.#prompting = false;
    }
  }

  /**
   * Return the provider chosen by the user.
   * This function checks the TOR_PROVIDER environment variable and if it is a
   * known provider, it returns its associated value.
   * Otherwise, if it is not valid, the C tor implementation is chosen as the
   * default one.
   *
   * @returns {string} An entry from TorProviders
   */
  static get providerType() {
    // TODO: Add a preference to permanently save this without and avoid always
    // using an environment variable.
    let provider = TorProviders.tor;
    const kEnvName = "TOR_PROVIDER";
    if (
      Services.env.exists(kEnvName) &&
      Services.env.get(kEnvName) in TorProviders
    ) {
      provider = TorProviders[Services.env.get(kEnvName)];
    }
    return provider;
  }
}
