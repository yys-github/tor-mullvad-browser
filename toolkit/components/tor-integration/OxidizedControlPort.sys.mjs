/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

import {
  OxidationAdapters,
  promisifyMethod,
} from "moz-src:///toolkit/components/tor-integration/OxidationAdapters.sys.mjs";

/**
 * An adapter between torITorNotificationListener and TorEventHandler.
 */
class NotificationAdapter {
  #actualHandler;

  constructor(handler) {
    this.#actualHandler = handler;
  }

  logMessage(severity, message) {
    this.#actualHandler.onLogMessage(severity, message);
  }

  bootstrapStatus(status) {
    this.#actualHandler.onBootstrapStatus(
      OxidationAdapters.convertBootstrapPhase(JSON.parse(status))
    );
  }

  circuitBuilt(circuit) {
    this.#actualHandler.onCircuitBuilt(
      OxidationAdapters.convertCircuit(JSON.parse(circuit))
    );
  }

  circuitClosed(id) {
    this.#actualHandler.onCircuitClosed(id);
  }

  streamSentConnect(stream) {
    stream = JSON.parse(stream);
    this.#actualHandler.onStreamSentConnect(
      stream.stream_id,
      stream.circuit_id,
      stream.socks_username,
      stream.socks_password
    );
  }

  connectionClosed() {
    this.#actualHandler.onClosed();
  }
}

/**
 * An adapter to allow TorProvider use the Rust implementation of the
 * controller.
 */
export class OxidizedControlPort {
  #native;

  static fromIpcFile(ipcFile) {
    return new OxidizedControlPort(Services.tor.createControlPortIPC(ipcFile));
  }

  static fromSocketAddress(host, port) {
    return new OxidizedControlPort(Services.tor.createControlPort(host, port));
  }

  constructor(native) {
    this.#native = native;
  }

  setEventHandler(eventHandler) {
    this.#native.setNotificationListener(new NotificationAdapter(eventHandler));
  }

  close() {
    this.#native?.close();
    this.#native = null;
  }

  get isOpen() {
    return !!this.#native;
  }

  authenticate(password) {
    return promisifyMethod(this.#native, "authenticate", Array.from(password));
  }

  async getBootstrapPhase() {
    return OxidationAdapters.convertBootstrapPhase(
      await promisifyMethod(this.#native, "getBootstrapPhase")
    );
  }

  async getNodeAddresses(id) {
    const addrs = await promisifyMethod(this.#native, "getNodeAddresses", id);
    const rv = [addrs.ipv4];
    if (addrs.ipv6) {
      rv.push(addrs.ipv6);
    }
    return rv;
  }

  async getIPCountry(ip) {
    return promisifyMethod(this.#native, "getIpCountry", ip);
  }

  getSocksListeners() {
    return promisifyMethod(this.#native, "getSocksListeners");
  }

  async getCircuits() {
    const circuits = await promisifyMethod(this.#native, "getCircuits");
    return circuits.map(OxidationAdapters.convertCircuit);
  }

  async getBridges() {
    const bridges = await promisifyMethod(this.#native, "getBridges");
    return bridges.map(OxidationAdapters.convertBridge);
  }

  async getPluggableTransports() {
    return OxidationAdapters.convertPluggableTransports(
      await promisifyMethod(this.#native, "getPluggableTransports")
    );
  }

  setConf(values) {
    return promisifyMethod(this.#native, "setConf", JSON.stringify(values));
  }

  setNetworkEnabled(enabled) {
    return promisifyMethod(this.#native, "setNetworkEnabled", enabled);
  }

  flushSettings() {
    return promisifyMethod(this.#native, "flushSettings");
  }

  async onionAuthViewKeys() {
    return OxidationAdapters.convertOnionKeys(
      await promisifyMethod(this.#native, "onionAuthViewKeys")
    );
  }

  onionAuthAdd(address, b64PrivateKey, isPermanent) {
    return promisifyMethod(
      this.#native,
      "onionAuthAdd",
      address,
      b64PrivateKey,
      isPermanent
    );
  }

  onionAuthRemove(address) {
    return promisifyMethod(this.#native, "onionAuthRemove", address);
  }

  takeOwnership() {
    return promisifyMethod(this.#native, "takeOwnership");
  }

  resetOwningControllerProcess() {
    return promisifyMethod(this.#native, "resetOwningControllerProcess");
  }

  newnym() {
    return promisifyMethod(this.#native, "newnym");
  }

  setEvents(types) {
    return promisifyMethod(this.#native, "setEvents", types);
  }
}
