import {
  TorProviderState,
  TorProviderInitError,
} from "moz-src:///toolkit/components/tor-launcher/TorProviderBuilder.sys.mjs";

/**
 * @callback StateChangedCallback
 */

/**
 * The base class for tor providers.
 */
export class TorProviderBase {
  /**
   * The current provider state.
   *
   * @type {string}
   */
  #state = TorProviderState.Starting;

  /**
   * The callback for when our state changes. `null` once the `uninit` method is
   * called.
   *
   * @type {?StateChangedCallback}
   */
  #stateChangedCallback;

  /**
   * The promise to return from the `init` method.
   *
   * @type {?Promise<undefined>}
   */
  #initPromise = null;

  /**
   * The promise to return from the `uninit` method.
   *
   * @type {?Promise<undefined>}
   */
  #uninitPromise = null;

  /**
   * Create a new provider.
   *
   * @param {StateChangedCallback} stateChangedCallback - A callback to let an
   *   owner know that a provider's state has changed.
   */
  // NOTE: This should *not* be overridden by implementations.
  constructor(stateChangedCallback) {
    this.#stateChangedCallback = stateChangedCallback;
  }

  /**
   * Initialize the provider and wait for it to be `Running`.
   *
   * This is safe to call multiple times, with each call waiting for the
   * provider to be ready.
   */
  // NOTE: This should *not* be overridden by implementations.
  async init() {
    if (!this.#initPromise) {
      this.#initPromise = this._initInternal().then(
        () => {
          this.#setState(TorProviderState.Running);
        },
        error => {
          this.#setState(TorProviderState.Stopped);
          // Wrap the error in `TorProviderInitError` to let callers know that
          // this error is an initialization error.
          throw new TorProviderInitError(error);
        }
      );
    }
    return this.#initPromise;
  }

  /**
   * Uninitialize the provider and wait for it to be cleaned up.
   *
   * This is safe to call multiple times, with each call waiting for the
   * clean up.
   */
  // NOTE: This should *not* be overridden by implementations.
  async uninit() {
    if (!this.#uninitPromise) {
      this.#setState(TorProviderState.Stopped);
      this.#stateChangedCallback = null;
      this.#uninitPromise = this._uninitInternal();
    }
    return this.#uninitPromise;
  }

  /**
   * The current `TorProviderState` state of the provider.
   *
   * @type {string}
   */
  // NOTE: This should *not* be overridden by implementations.
  get state() {
    return this.#state;
  }

  /**
   * Set the state of the provider. Announcing this via a callback.
   *
   * @param {string} state - The new `TorProviderState`.
   */
  #setState(state) {
    if (state === this.#state) {
      return;
    }
    if (this.#state === TorProviderState.Stopped) {
      // Ignore any changes away from the `Stopped` state, which is unexpected
      // since implementations can only trigger the `Stopped` state, and
      // everything else is handled by `TorProviderBase`.
      return;
    }
    this.#state = state;
    this.#stateChangedCallback?.();
  }

  /**
   * An internal method to be called by implementations when they stop working.
   *
   * Optional and safe to call as part of `_uninitInternal`.
   */
  // NOTE: This should *not* be overridden by implementations.
  _stoppedInternal() {
    this.#setState(TorProviderState.Stopped);
  }

  // Implementation methods.

  /**
   * An internal initialization method for provider instances to implement.
   */
  async _initInternal() {
    throw new Error("_initInternal not implemented.");
  }

  /**
   * An internal uninitialization method for provider instances to implement.
   */
  async _uninitInternal() {
    throw new Error("_uninitInternal not implemented.");
  }

  async writeBridgeSettings(_bridges) {
    throw new Error("writeBridgeSettings not implemented.");
  }

  async writeProxySettings(_proxy) {
    throw new Error("writeProxySettings not implemented.");
  }

  async writeFirewallSettings(_firewall) {
    throw new Error("writeFirewallSettings not implemented.");
  }

  async flushSettings() {
    throw new Error("flushSettings not implemented.");
  }

  async connect() {
    throw new Error("connect not implemented.");
  }

  async stopBootstrap() {
    throw new Error("stopBootstrap not implemented.");
  }

  async newnym() {
    throw new Error("newnym not implemented.");
  }

  async getBridges() {
    throw new Error("getBridges not implemented.");
  }

  async getPluggableTransports() {
    throw new Error("getPluggableTransports not implemented.");
  }

  async onionAuthAdd(_address, _b64PrivateKey, _isPermanent) {
    throw new Error("onionAuthAdd not implemented.");
  }

  async onionAuthRemove(_address) {
    throw new Error("onionAuthRemove not implemented.");
  }

  async onionAuthViewKeys() {
    throw new Error("onionAuthViewKeys not implemented.");
  }

  get currentBridge() {
    throw new Error("currentBridge not implemented.");
  }
}
