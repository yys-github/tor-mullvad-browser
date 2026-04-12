/* Any copyright is dedicated to the Public Domain.
 * http://creativecommons.org/publicdomain/zero/1.0/ */

"use strict";

const IPC_SUPPORTED = Services.appinfo.OS !== "WINNT";

/**
 * A control port mock server.
 */
class ControlPortServer {
  #server;
  #ipcFile;
  #protocol;

  constructor(protocol, listener) {
    this.#server = Cc["@mozilla.org/network/server-socket;1"].createInstance(
      Ci.nsIServerSocket
    );

    switch (protocol) {
      case "tcp":
        this.#server.init(-1, true, -1);
        break;
      case "ipc": {
        this.#ipcFile = Services.dirsvc.get("TmpD", Ci.nsIFile);
        this.#ipcFile.append("control-port.socket");
        this.#ipcFile.createUnique(Ci.nsIFile.NORMAL_FILE_TYPE, 0o600);
        this.#ipcFile.remove(false);
        this.#server.initWithFilename(this.#ipcFile, 0o600, -1);
        break;
      }
      default:
        Assert.ok(false, `Unknown protocol ${protocol}`);
        return;
    }
    this.#protocol = protocol;

    const address =
      protocol === "tcp"
        ? `localhost:${this.#server.port}`
        : this.#ipcFile.path;
    this.#server.asyncListen({
      onSocketAccepted(_socket, transport) {
        info(`Got a new connection on ${address}.`);
        listener(new Transport(transport));
      },
      onStopListening() {
        info(`Stopped listening on ${address}.`);
      },
    });
  }

  get port() {
    if (this.#protocol !== "tcp") {
      throw new Error("Port is supported only for TCP servers.");
    }
    return this.#server.port;
  }

  get ipcFile() {
    if (this.#protocol !== "ipc") {
      throw new Error("The ipc file is supported only for IPC servers.");
    }
    return this.#ipcFile.clone();
  }

  get protocol() {
    return this.#protocol;
  }

  close() {
    this.#server.close();
    if (this.#ipcFile) {
      this.#ipcFile.remove(false);
    }
  }
}

/**
 * A socket connected to a control port client (seen from the server
 * perspective).
 */
class Transport {
  #transport;
  #inputStream;
  #scriptableInputStream;
  #outputStream;
  #lineBuffer = "";

  constructor(transport) {
    this.#transport = transport;
    this.#inputStream = transport
      .openInputStream(0, 0, 0)
      .QueryInterface(Ci.nsIAsyncInputStream);
    this.#scriptableInputStream = Cc[
      "@mozilla.org/scriptableinputstream;1"
    ].createInstance(Ci.nsIScriptableInputStream);
    this.#scriptableInputStream.init(this.#inputStream);
    this.#outputStream = transport
      .openOutputStream(0, 0, 0)
      .QueryInterface(Ci.nsIAsyncOutputStream);
  }

  read() {
    const { promise, resolve, reject } = Promise.withResolvers();
    this.#inputStream.asyncWait(
      {
        onInputStreamReady: () => {
          let available = 0;
          try {
            available = this.#scriptableInputStream.available();
          } catch (e) {
            reject(e);
            return;
          }
          if (!available) {
            reject(new Error("Connection closed"));
            return;
          }
          const str = this.#scriptableInputStream.read(available);
          resolve(str);
        },
      },
      0,
      0,
      Services.tm.mainThread
    );
    return promise;
  }

  async readLine() {
    const updateBuffer = idx => {
      const line = this.#lineBuffer.substring(0, idx);
      this.#lineBuffer = this.#lineBuffer.substring(idx + 2);
      return line;
    };
    let idx = this.#lineBuffer.indexOf("\r\n");
    if (idx != -1) {
      return updateBuffer(idx);
    }
    do {
      this.#lineBuffer += await this.read();
      idx = this.#lineBuffer.indexOf("\r\n");
    } while (idx == -1);
    return updateBuffer(idx);
  }

  write(str) {
    const { promise, resolve, reject } = Promise.withResolvers();
    this.#outputStream.asyncWait(
      {
        onOutputStreamReady: () => {
          try {
            this.#outputStream.write(str, str.length);
          } catch (e) {
            reject(e);
          }
          resolve();
        },
      },
      0,
      str.length,
      Services.tm.mainThread
    );
    return promise;
  }

  close() {
    this.#inputStream.close();
    this.#outputStream.close();
    this.#transport.close(Cr.NS_OK);
  }
}

/**
 * A wrapper to the XPCOM code we are testing for turning it into async JS.
 * It can be extended to actually handle async notifications.
 */
class ControlPortClient {
  #controlPort;
  constructor(server) {
    if (server.protocol === "tcp") {
      this.#controlPort = Services.tor.createControlPort(
        "localhost",
        server.port
      );
    } else if (server.protocol === "ipc") {
      this.#controlPort = Services.tor.createControlPortIPC(server.ipcFile);
    } else {
      throw new Error("Unknown server protocol");
    }
    const receiver = {
      onAsyncMessage: message => this.onAsyncMessage(message),
    };
    this.#controlPort.start(receiver);
  }

  sendCommand(command) {
    const { promise, resolve, reject } = Promise.withResolvers();
    this.#controlPort.sendCommand(`${command}\r\n`, {
      onMessage(message) {
        resolve(message);
      },
      onError(error) {
        reject(new Error(error));
      },
    });
    return promise;
  }

  close() {
    this.#controlPort.close();
  }

  onAsyncMessage(_message) {
    Assert.ok(false, "This test does not use async notifications.");
  }
}
