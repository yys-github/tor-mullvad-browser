/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/**
 * Converters from the JSON created by the Rust crates to the objects expected
 * by the JS tor-launcher.
 */
export class OxidationAdapters {
  static #bytesToHex(bytes) {
    return Array.from(bytes, b => b.toString(16).padStart(2, "0")).join("");
  }

  static convertCircuit(circuit) {
    const circ = {
      id: circuit.id,
      nodes: circuit.path.map(fp => OxidationAdapters.#bytesToHex(fp)),
    };
    if (circuit.conflux_id) {
      circ.confluxId = OxidationAdapters.#bytesToHex(circuit.conflux_id);
    }
    return circ;
  }

  static convertBootstrapPhase(phase) {
    return Object.fromEntries(
      Object.entries(phase).map(([key, value]) => {
        if (key === "severity") {
          key = "TYPE";
          value = value.toUpperCase();
        } else if (key === "should_warn") {
          key = "RECOMMENDATION";
          value = value ? "warn" : "ignore";
        } else {
          key = key.toUpperCase();
        }
        return [key, value];
      })
    );
  }

  static convertBridge(b) {
    const bridge = {
      transport: b.transport ?? "vanilla",
      addr: b.address,
    };
    if (b.fingerprint) {
      bridge.id = b.fingerprint;
    }
    if (b.args) {
      bridge.args = b.args;
    }
    return bridge;
  }

  static convertPluggableTransports(pts) {
    return pts.map(pt => {
      if (pt.pt_type === "exec") {
        return {
          transports: pt.transports,
          type: pt.pt_type,
          pathToBinary: pt.path_to_binary,
          options: pt.options ?? "",
        };
      }
      return {
        transports: pt.transports,
        type: pt.pt_type,
        ip: pt.ip,
        port: pt.port,
      };
    });
  }

  static convertOnionKeys(keys) {
    return keys.map(k => {
      const key = {
        address: k.address,
        keyType: k.key_type,
        keyBlob: k.key_blob,
        flags: [],
      };
      if (k.is_permanent) {
        key.flags.push("Permanent");
      }
      return key;
    });
  }
}

/**
 * A class that converts an XPCOM callback to a JS promise.
 */
class PromisifiedCallback {
  #promise;
  #resolve;
  #reject;

  constructor() {
    const { promise, resolve, reject } = Promise.withResolvers();
    this.#promise = promise;
    this.#resolve = resolve;
    this.#reject = reject;
  }

  get promise() {
    return this.#promise;
  }

  onReply(reply) {
    if (reply) {
      this.#resolve(JSON.parse(reply));
    } else {
      this.#resolve();
    }
  }

  onTorError(code, message) {
    const err = new Error(message);
    err.code = code;
    this.#reject(err);
  }

  onGenericError(message) {
    this.#reject(new Error(message));
  }
}

/**
 * Promisify an XPCOM method that has a signature that looks like:
 *   void name(<args>, in torITorReplyCallback callback);
 *
 * @param {object} obj The oject that contains the method to call
 * @param {string} method The name of the method to call
 * @param  {...any} args The arguments to pass before the callback
 * @returns {PromiseLike} A promise that will be resolved/rejected by the native
 * method
 */
export async function promisifyMethod(obj, method, ...args) {
  const cb = new PromisifiedCallback();
  obj[method](...args, cb);
  return cb.promise;
}
