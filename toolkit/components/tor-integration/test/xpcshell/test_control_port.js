/* Any copyright is dedicated to the Public Domain.
 * http://creativecommons.org/publicdomain/zero/1.0/ */

"use strict";

const { OxidizedControlPort } = ChromeUtils.importESModule(
  "moz-src:///toolkit/components/tor-integration/OxidizedControlPort.sys.mjs"
);

/**
 * Mock server to handle control port commands.
 */
class MockServer {
  #transport;
  noPassword = false;
  listeners = [];
  conf = new Map();
  onionAuth = new Map();
  events = [];

  constructor(transport) {
    this.#transport = transport;
  }

  async start() {
    info("Starting a mock server");
    let line;
    while ((line = await this.#transport.readLine())) {
      if (line.endsWith("\r\n")) {
        line = line.slice(0, -2);
      }
      const spaceIdx = line.indexOf(" ");
      const command = spaceIdx !== -1 ? line.slice(0, spaceIdx) : line;
      const args = spaceIdx !== -1 ? line.slice(spaceIdx + 1) : "";
      switch (command.toUpperCase()) {
        case "AUTHENTICATE":
          await this.#authenticate(args);
          break;
        case "GETINFO":
          await this.#getInfo(args);
          break;
        case "GETCONF":
          await this.#getConf(args);
          break;
        case "SETCONF":
          await this.#setConf(args);
          break;
        case "ONION_CLIENT_AUTH_VIEW":
          await this.#onionView();
          break;
        case "ONION_CLIENT_AUTH_ADD":
          await this.#onionAdd(args);
          break;
        case "ONION_CLIENT_AUTH_REMOVE":
          await this.#onionRemove(args);
          break;
        case "SETEVENTS":
          await this.#setEvents(args);
          break;
        case "SAVECONF":
        case "TAKEOWNERSHIP":
        case "RESETCONF":
        case "SIGNAL":
          await this.write("250 OK\r\n");
          break;
        default:
          await this.write(`510 Unrecognized command "${command}"\r\n`);
          break;
      }
    }
  }

  async write(str) {
    await this.#transport.write(str);
  }

  close() {
    this.#transport.close();
  }

  async #authenticate(args) {
    if (args == "01234567" || (this.noPassword && !args)) {
      await this.write("250 OK\r\n");
      return;
    }
    await this.write(`515 Authentication failed: wrong password ${args}\r\n`);
    this.close();
  }

  async #getInfo(args) {
    // We support only a single keyword in this mock server.
    if (args === "status/bootstrap-phase") {
      await this.#bootstrapPhase();
    } else if (args.startsWith("ns/id/")) {
      await this.#nodeInfo(args.substring(6));
    } else if (args.startsWith("ip-to-country/")) {
      await this.#ipToCountry(args.substring("ip-to-country/".length));
    } else if (args === "net/listeners/socks") {
      await this.#socksListeners();
    } else if (args === "circuit-status") {
      await this.#circuitStatus();
    } else {
      await this.write(`552 Unrecognized key "${args}"\r\n`);
    }
  }

  async #bootstrapPhase() {
    await this.write(
      '250-status/bootstrap-phase=NOTICE BOOTSTRAP PROGRESS=100 TAG=done SUMMARY="Done"\r\n'
    );
    await this.write("250 OK\r\n");
  }

  async #nodeInfo(id) {
    const reply =
      `250+ns/id/${id}=\r\n` +
      "r nickname RGiAFtNSR33prk7Csp+CQCVNgOg rLB4YR4Xkky0sutW2EvMJKmWj/y 2038-01-01 00:00:00 12.34.56.78 9001 0\r\n" +
      "a [2001:0000:123:4567::89ab]:9001\r\n" +
      "s Fast Guard Running Stable V2Dir Valid\r\n" +
      "w Bandwidth=5000\r\n" +
      ".\r\n" +
      "250 OK\r\n";
    await this.write(reply);
  }

  async #ipToCountry(ip) {
    const reply = code =>
      this.write(`250-ip-to-country/${ip}=${code}\r\n250 OK\r\n`);
    if (ip === "1.2.3.4") {
      await reply("au");
    } else if (ip === "12.34.56.78") {
      await reply("US");
    } else if (ip === "127.0.0.1") {
      // tor would say ??, but we use it to trigger a parsing error.
      await reply("invalid");
    } else {
      await reply("??");
    }
  }

  async #socksListeners() {
    await this.write(
      `250-net/listeners/socks=${this.listeners.join(" ")}\r\n250 OK\r\n`
    );
  }

  async #circuitStatus() {
    const reply =
      "250+circuit-status=\r\n" +
      "67 BUILT $1111111111111111111111111111111111111111~R1,$2222222222222222222222222222222222222222~Re2,$3333333333333333333333333333333333333333~Relay3 CONFLUX_ID=cccccccccccccccccccccccccccccccc\r\n" +
      "68 BUILT $4444444444444444444444444444444444444444~Rel4,$5555555555555555555555555555555555555555~Rela5,$3333333333333333333333333333333333333333~Relay3 CONFLUX_ID=cccccccccccccccccccccccccccccccc\r\n" +
      "6fF BUILT $1111111111111111111111111111aAbBcCdDeEfF,$7777777777777777777777777777777777777777,$8888888888888888888888888888888888888888\r\n" +
      "111 OTHER_STATUS SHOULD=BE_IGNORED\r\n" +
      ".\r\n" +
      "250 OK\r\n";
    await this.write(reply);
  }

  async #getConf(key) {
    switch (key.toLowerCase()) {
      case "bridge":
        await this.#getBridges();
        break;
      case "clienttransportplugin":
        this.#getClientTransportPlugin();
        break;
      default:
        await this.write(`552 Unrecognized configuration key "${key}"\r\n`);
    }
  }

  async #getBridges() {
    const reply =
      "250-Bridge=obfs4 1.2.3.4:443 1111111111111111111111111111111111111111 cert=IiE5kF0SGOWNDF2nN2RQA5iMszRJdvHA3/6JNr0vPZRO5ra2GslNPT46c1Hkj4O5O0a8/g iat-mode=0\r\n" +
      "250-Bridge=obfs4 123.4.56.78:80 2222222222222222222222222222222222222222 cert=tTo5rWWGawTCMsEHM3DKVPzFni8d0BZib6xtcPVk6F2spWavYyZ5FXqlsaq8exih/6n+UD iat-mode=1\r\n" +
      "250-Bridge=snowflake 5.6.7.8:443\r\n" +
      "250-Bridge=snowflake 12.3.4.5:443 some=param\r\n" +
      "250 Bridge=123.45.6.78:9050\r\n";
    await this.write(reply);
  }

  async #getClientTransportPlugin() {
    const reply =
      "250-ClientTransportPlugin=meek_lite,obfs2,obfs3,obfs4,scramblesuit,webtunnel exec ./TorBrowser/Tor/PluggableTransports/lyrebird\r\n" +
      "250-ClientTransportPlugin=snowflake exec .\\TorBrowser\\Tor\\PluggableTransports\\lyrebird\r\n" +
      "250-ClientTransportPlugin=conjure exec ./TorBrowser/Tor/PluggableTransports/conjure-client -registerURL https://registration.refraction.network/api\r\n" +
      "250-ClientTransportPlugin=pt1 socks4 127.0.0.1:9052\r\n" +
      "250 ClientTransportPlugin=pt2 socks5 [::1]:9053\r\n";
    await this.write(reply);
  }

  async #setConf(args) {
    // Notice: this does not handle escaped quotes.
    const re = /(\w+)(?:=(\d+)|="([^"]*)")?/g;
    const conf = new Map();
    for (const m of args.matchAll(re)) {
      const key = m[1];
      // Special key we handled differently to simulate a failure.
      if (key.toUpperCase() === "INVALID") {
        await this.write(
          `552 Unrecognized option: Unknown option '${key}'.  Failing.\r\n`
        );
        return;
      }
      const intVal = m[2] !== undefined ? parseInt(m[2]) : undefined;
      const value = intVal ?? m[3] ?? null;
      const existing = conf.get(key);
      if (existing === undefined) {
        conf.set(key, value);
      } else if (Array.isArray(existing)) {
        existing.push(value);
      } else {
        conf.set(key, [existing, value]);
      }
    }
    conf.forEach((v, k) => this.conf.set(k, v));
    await this.write("250 OK\r\n");
  }

  async #onionView() {
    let reply = "250-ONION_CLIENT_AUTH_VIEW\r\n";
    for (const [address, info] of this.onionAuth) {
      reply += `250-CLIENT ${address} ${info.key} Flags=${info.flags}\r\n`;
    }
    reply += "250 OK\r\n";
    await this.write(reply);
  }

  async #onionAdd(args) {
    args = args.split(" ");
    const address = args.shift();
    const info = { key: args.shift() };
    while (args.length) {
      const arg = args.shift();
      if (arg.startsWith("Flags=")) {
        info.flags = arg.substring(6);
      }
    }
    this.onionAuth.set(address, info);
    await this.write("250 OK\r\n");
  }

  async #onionRemove(args) {
    if (this.onionAuth.delete(args)) {
      await this.write("250 OK\r\n");
    } else {
      await this.write(`251 No credentials for "args"\r\n`);
    }
  }

  async #setEvents(args) {
    this.events = args.split(" ");
    await this.write("250 OK\r\n");
  }
}

async function runTest(func) {
  const run = async (server, client) => {
    try {
      const serverTransport = await server.listenOnce();
      const controlServer = new MockServer(serverTransport);
      const serverLoopDone = controlServer.start();

      try {
        await func(controlServer, client);
      } finally {
        try {
          serverTransport.close();
        } catch (e) {
          info(`Failed to close the server-side transport: ${e}`);
        }
        try {
          await serverLoopDone;
        } catch (e) {
          info(`Exception on the server loop: ${e}`);
        }
      }
    } finally {
      try {
        client?.close();
      } catch (e) {
        info(`Failed to close the client: ${e}`);
      }
      try {
        server?.close();
      } catch (e) {
        info(`Failed to close the server: ${e}`);
      }
    }
  };

  {
    info("Running with TCP");
    const server = new ControlPortServer("tcp");
    const client = OxidizedControlPort.fromSocketAddress(
      "127.0.0.1",
      server.port
    );
    await run(server, client);
  }

  if (IPC_SUPPORTED) {
    info("Running with Unix-domain sockets.");
    const server = new ControlPortServer("ipc");
    const client = OxidizedControlPort.fromIpcFile(server.ipcFile);
    await run(server, client);
  }
}

add_task(async function test_authenticate_happy() {
  await runTest(async (_server, client) => {
    try {
      await client.authenticate([0x01, 0x23, 0x45, 0x67]);
      Assert.ok(true, "Authentication succeeded.");
    } catch (e) {
      Assert.ok(false, `Authenticate failed: ${e.message}.`);
    }
  });
});

add_task(async function test_authenticate_failure() {
  await runTest(async (_server, client) => {
    let failed = false;
    try {
      await client.authenticate([]);
    } catch (e) {
      Assert.ok(true, `Authenticate failed as expected: ${e.message}.`);
      failed = true;
    }
    Assert.ok(failed, "Wrong authentication succeeded.");
  });

  await runTest(async (_server, client) => {
    let failed = false;
    try {
      await client.authenticate([0xde, 0xad, 0xbe, 0xef]);
    } catch (e) {
      Assert.ok(true, `Authenticate failed as expected: ${e.message}.`);
      failed = true;
    }
    Assert.ok(failed, "Wrong authentication succeeded.");
  });
});

add_task(async function test_authenticate_no_password() {
  await runTest(async (server, client) => {
    server.noPassword = true;
    try {
      await client.authenticate([]);
      Assert.ok(true, "Authentication succeeded.");
    } catch (e) {
      Assert.ok(false, `Authenticate failed: ${e.message}.`);
    }
  });
});

add_task(async function test_bootstrap_phase() {
  await runTest(async (_server, client) => {
    // TODO: Add various phases to the mock server.
    let phase = await client.getBootstrapPhase();
    Assert.equal(phase.TYPE, "NOTICE");
    Assert.equal(phase.PROGRESS, 100);
    Assert.equal(phase.TAG, "done");
    Assert.equal(phase.SUMMARY, "Done");
  });
});

add_task(async function test_node_address() {
  await runTest(async (_server, client) => {
    const addresses = await client.getNodeAddresses(
      "1111111111111111111111111111111111111111"
    );
    Assert.equal(addresses.length, 2);
    Assert.equal(addresses[0], "12.34.56.78");
    Assert.equal(addresses[1], "2001:0:123:4567::89ab");
  });
});

add_task(async function test_ip_country() {
  await runTest(async (_server, client) => {
    // Test we do not change the case of the code.
    Assert.equal(await client.getIPCountry("1.2.3.4"), "au");
    Assert.equal(await client.getIPCountry("12.34.56.78"), "US");
    Assert.equal(await client.getIPCountry("123.4.56.78"), "??");

    let failed = true;
    try {
      // The real tor would not fail, it is a peculiarity of our mock server.
      await client.getIPCountry("127.0.0.1");
      failed = false;
    } catch (e) {
      Assert.ok(true, `Got the expected failure: ${e.message}`);
    }
    Assert.ok(failed, "Failed when expected");
  });
});

add_task(async function test_socks_listeners() {
  await runTest(async (server, client) => {
    // Before bootstrap: no listeners
    server.listeners = [];
    let listeners = await client.getSocksListeners();
    Assert.equal(listeners.length, 0);

    // Normal TCP
    server.listeners = ['"127.0.0.1:9150"'];
    listeners = await client.getSocksListeners();
    Assert.deepEqual(listeners, [{ host: "127.0.0.1", port: 9150 }]);

    // Unix sockets
    server.listeners = ['"unix:/run/user/1000/Tor/socks.socket"'];
    listeners = await client.getSocksListeners();
    Assert.deepEqual(listeners, [
      { ipcPath: "/run/user/1000/Tor/socks.socket" },
    ]);

    // Multiple
    server.listeners = ['"127.0.0.1:9050"', '"127.0.0.1:9150"'];
    listeners = await client.getSocksListeners();
    Assert.deepEqual(listeners, [
      { host: "127.0.0.1", port: 9050 },
      { host: "127.0.0.1", port: 9150 },
    ]);

    // IPv6
    server.listeners = ['"[2001:0db8:7000:0000:0000:dead:beef:1234]:9150"'];
    listeners = await client.getSocksListeners();
    Assert.deepEqual(listeners, [
      { host: "2001:db8:7000::dead:beef:1234", port: 9150 },
    ]);
  });
});

add_task(async function test_circuits() {
  await runTest(async (_server, client) => {
    const circs = await client.getCircuits();
    Assert.deepEqual(circs, [
      {
        id: "67",
        nodes: [
          "1111111111111111111111111111111111111111",
          "2222222222222222222222222222222222222222",
          "3333333333333333333333333333333333333333",
        ],
        confluxId: "cccccccccccccccccccccccccccccccc",
      },
      {
        id: "68",
        nodes: [
          "4444444444444444444444444444444444444444",
          "5555555555555555555555555555555555555555",
          "3333333333333333333333333333333333333333",
        ],
        confluxId: "cccccccccccccccccccccccccccccccc",
      },
      {
        id: "6fF",
        nodes: [
          "1111111111111111111111111111aabbccddeeff",
          "7777777777777777777777777777777777777777",
          "8888888888888888888888888888888888888888",
        ],
      },
    ]);
  });
});

add_task(async function test_bridges() {
  await runTest(async (_server, client) => {
    const bridges = await client.getBridges();
    Assert.deepEqual(bridges, [
      {
        transport: "obfs4",
        addr: "1.2.3.4:443",
        id: "1111111111111111111111111111111111111111",
        args: "cert=IiE5kF0SGOWNDF2nN2RQA5iMszRJdvHA3/6JNr0vPZRO5ra2GslNPT46c1Hkj4O5O0a8/g iat-mode=0",
      },
      {
        transport: "obfs4",
        addr: "123.4.56.78:80",
        id: "2222222222222222222222222222222222222222",
        args: "cert=tTo5rWWGawTCMsEHM3DKVPzFni8d0BZib6xtcPVk6F2spWavYyZ5FXqlsaq8exih/6n+UD iat-mode=1",
      },
      {
        transport: "snowflake",
        addr: "5.6.7.8:443",
      },
      {
        transport: "snowflake",
        addr: "12.3.4.5:443",
        args: "some=param",
      },
      {
        transport: "vanilla",
        addr: "123.45.6.78:9050",
      },
    ]);
  });
});

add_task(async function test_pluggable_transports() {
  await runTest(async (_server, client) => {
    const pts = await client.getPluggableTransports();
    Assert.deepEqual(pts, [
      {
        transports: [
          "meek_lite",
          "obfs2",
          "obfs3",
          "obfs4",
          "scramblesuit",
          "webtunnel",
        ],
        type: "exec",
        pathToBinary: "./TorBrowser/Tor/PluggableTransports/lyrebird",
        options: "",
      },
      {
        transports: ["snowflake"],
        type: "exec",
        pathToBinary: ".\\TorBrowser\\Tor\\PluggableTransports\\lyrebird",
        options: "",
      },
      {
        transports: ["conjure"],
        type: "exec",
        pathToBinary: "./TorBrowser/Tor/PluggableTransports/conjure-client",
        options: "-registerURL https://registration.refraction.network/api",
      },
      { transports: ["pt1"], type: "socks4", ip: "127.0.0.1", port: "9052" },
      { transports: ["pt2"], type: "socks5", ip: "[::1]", port: "9053" },
    ]);
  });
});

add_task(async function test_set_conf() {
  await runTest(async (server, client) => {
    const entries = [
      ["boolean", true],
      ["integer", 3],
      ["string", "value"],
      ["array", ["val1", "val2", "val3"]],
      ["null", null],
    ];
    Assert.equal(server.conf.size, 0, "Initially the conf map is empty.");
    await client.setConf(entries);
    // Booleans are converted to 0 and 1.
    entries[0][1] = 1;
    Assert.deepEqual(Array.from(server.conf.entries()), entries);
  });
});

add_task(async function test_set_conf_bad() {
  await runTest(async (_server, client) => {
    let failed = false;
    try {
      await client.setConf([["INVALID", true]]);
    } catch (e) {
      failed = true;
      Assert.equal(e.code, 552);
      Assert.equal(
        e.message,
        "Unrecognized option: Unknown option 'INVALID'.  Failing."
      );
    }
    Assert.ok(failed, "We expected the SETCONF call to fail.");
  });
});

add_task(async function test_set_network_enabled() {
  await runTest(async (server, client) => {
    await client.setNetworkEnabled(false);
    Assert.equal(server.conf.get("DisableNetwork"), 1);
    await client.setNetworkEnabled(true);
    Assert.equal(server.conf.get("DisableNetwork"), 0);
  });
});

add_task(async function test_onion_auth_flow() {
  await runTest(async (server, client) => {
    Assert.equal(server.onionAuth.size, 0);
    await client.onionAuthAdd(
      "pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd",
      "r9aD8uLN/kDI2Bmk633RvwgB/9uwAxNzhrHgJcs8I7wP",
      false
    );
    Assert.equal(server.onionAuth.size, 1);
    Assert.deepEqual(
      server.onionAuth.get(
        "PG6MMJIYJMCRSSLVYKFWNNTLARU7P5SVN6Y2YMMJU6NUBXNDF4PSCRYD"
      ),
      { key: "x25519:r9aD8uLN/kDI2Bmk633RvwgB/9uwAxNzhrHgJcs8I7wP" }
    );
    Assert.deepEqual(await client.onionAuthViewKeys(), [
      {
        address: "PG6MMJIYJMCRSSLVYKFWNNTLARU7P5SVN6Y2YMMJU6NUBXNDF4PSCRYD",
        keyType: "x25519",
        keyBlob: "r9aD8uLN/kDI2Bmk633RvwgB/9uwAxNzhrHgJcs8I7wP",
        flags: [],
      },
    ]);

    await client.onionAuthAdd(
      "PG6MMJiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd",
      "r9aD8uLN/kDI2Bmk633RvwgB/9uwAxNzhrHgJcs8I7wP",
      true
    );
    Assert.equal(server.onionAuth.size, 1);
    Assert.deepEqual(
      server.onionAuth.get(
        "PG6MMJIYJMCRSSLVYKFWNNTLARU7P5SVN6Y2YMMJU6NUBXNDF4PSCRYD"
      ),
      {
        key: "x25519:r9aD8uLN/kDI2Bmk633RvwgB/9uwAxNzhrHgJcs8I7wP",
        flags: "Permanent",
      }
    );
    Assert.deepEqual(await client.onionAuthViewKeys(), [
      {
        address: "PG6MMJIYJMCRSSLVYKFWNNTLARU7P5SVN6Y2YMMJU6NUBXNDF4PSCRYD",
        keyType: "x25519",
        keyBlob: "r9aD8uLN/kDI2Bmk633RvwgB/9uwAxNzhrHgJcs8I7wP",
        flags: ["Permanent"],
      },
    ]);

    await client.onionAuthRemove(
      "PG6MMJIYJMcrSSLVykfwnnTlARu7P5sVN6y2yMmjU6NUBxndf4pscryD"
    );
    Assert.equal(server.onionAuth.size, 0);
    Assert.deepEqual(await client.onionAuthViewKeys(), []);

    // Removing again should not throw.
    await client.onionAuthRemove(
      "PG6MMJIYJMcrSSLVykfwnnTlARu7P5sVN6y2yMmjU6NUBxndf4pscryD"
    );
  });
});

add_task(async function test_onion_auth_add_bad() {
  await runTest(async (_server, client) => {
    let failed = false;
    try {
      await client.onionAuthAdd(
        "invalidonionaddress",
        "r9aD8uLN/kDI2Bmk633RvwgB/9uwAxNzhrHgJcs8I7wP",
        true
      );
    } catch (e) {
      Assert.ok(true, `The ONION_CLIENT_AUTH_ADD request failed: ${e.message}`);
      Assert.equal(e.result, Cr.NS_ERROR_ILLEGAL_VALUE);
      failed = true;
    }
    Assert.ok(
      failed,
      "This ONION_CLIENT_AUTH_ADD request was expected to fail."
    );
  });

  await runTest(async (_server, client) => {
    let failed = false;
    try {
      await client.onionAuthAdd(
        "PG6MMJIYJMCRSSLVYKFWNNTLARU7P5SVN6Y2YMMJU6NUBXNDF4PSCRYD",
        "not!base$64:(",
        true
      );
    } catch (e) {
      Assert.ok(true, `The ONION_CLIENT_AUTH_ADD request failed: ${e.message}`);
      failed = true;
    }
    Assert.ok(
      failed,
      "This ONION_CLIENT_AUTH_ADD request was expected to fail."
    );
  });
});

add_task(async function test_onion_auth_remove_bad() {
  await runTest(async (_server, client) => {
    let failed = false;
    try {
      await client.onionAuthAdd("invalidonionaddress");
    } catch (e) {
      Assert.ok(
        true,
        `The ONION_CLIENT_AUTH_REMOVE request failed: ${e.message}`
      );
      Assert.equal(e.result, Cr.NS_ERROR_ILLEGAL_VALUE);
      failed = true;
    }
    Assert.ok(
      failed,
      "This ONION_CLIENT_AUTH_REMOVE request was expected to fail."
    );
  });
});

add_task(async function test_onion_auth_view_bad() {
  const runCommand = async client => {
    let failed = false;
    try {
      await client.onionAuthView();
    } catch (e) {
      Assert.ok(
        true,
        `The ONION_CLIENT_AUTH_VIEW request failed: ${e.message}`
      );
      failed = true;
    }
    Assert.ok(
      failed,
      "This ONION_CLIENT_AUTH_VIEW request was expected to fail."
    );
  };

  await runTest(async (server, client) => {
    server.onionAuth.set("invalidoniondomain", {
      key: "x25519:r9aD8uLN/kDI2Bmk633RvwgB/9uwAxNzhrHgJcs8I7wP",
      flags: "Permanent",
    });
    await runCommand(client);
  });

  await runTest(async (server, client) => {
    server.onionAuth.set(
      "PG6MMJIYJMCRSSLVYKFWNNTLARU7P5SVN6Y2YMMJU6NUBXNDF4PSCRYD",
      {
        key: "x25519:not!base:(64$$$$",
        flags: "",
      }
    );
    await runCommand(client);
  });
});

add_task(async function test_set_events() {
  await runTest(async (server, client) => {
    Assert.equal(server.events.length, 0);
    const events = ["NOTICE", "INFO"];
    await client.setEvents(events);
    Assert.deepEqual(server.events, events);
  });
});

add_task(async function test_simple_commands() {
  await runTest(async (_server, client) => {
    await client.flushSettings();
    await client.takeOwnership();
    await client.resetOwningControllerProcess();
    await client.newnym();
  });
});

/**
 * A notification listener that will be used for tests.
 * It implements TorEventHandler as defined in TorControlPort.sys.mjs.
 */
class NotificationListener {
  #nextType;
  #resolve;
  #reject;

  #checkType(type) {
    const next = this.#nextType;
    this.#nextType = null;
    if (type !== next) {
      this.#reject(new Error(`Wrong type: got ${type}, expected ${next}`));
      return false;
    }
    return true;
  }

  setup(type) {
    this.#nextType = type;
    return new Promise((resolve, reject) => {
      this.#resolve = resolve;
      this.#reject = reject;
    });
  }

  onLogMessage(severity, message) {
    if (this.#checkType("logMessage")) {
      this.#resolve({ severity, message });
    }
  }

  onBootstrapStatus(status) {
    if (this.#checkType("bootstrapStatus")) {
      this.#resolve(status);
    }
  }

  onCircuitBuilt(circuit) {
    if (this.#checkType("circuitBuilt")) {
      this.#resolve(circuit);
    }
  }

  onCircuitClosed(id) {
    if (this.#checkType("circuitClosed")) {
      this.#resolve(id);
    }
  }

  onStreamSentConnect(streamId, circuitId, username, password) {
    if (this.#checkType("streamSentConnect")) {
      this.#resolve({
        streamId,
        circuitId,
        username,
        password,
      });
    }
  }

  onClosed() {
    if (this.#checkType("closed")) {
      this.#resolve();
    }
  }
}

add_task(async function test_log_notifications() {
  await runTest(async (server, client) => {
    const listener = new NotificationListener();
    client.setEventHandler(listener);
    const testLog = async (severity, message) => {
      const test = listener.setup("logMessage");
      await server.write(`650 ${severity} ${message}\r\n`);
      Assert.deepEqual(await test, {
        severity,
        message,
      });
    };
    await testLog("DEBUG", "Some debug info");
    await testLog("INFO", "An info message");
    await testLog("NOTICE", "A notice");
    await testLog("WARN", "An warning, so not too bad");
    await testLog("ERR", "A bad error!!");
  });
});

add_task(async function test_circuit_built_notifications() {
  await runTest(async (server, client) => {
    const listener = new NotificationListener();
    client.setEventHandler(listener);
    {
      const test = listener.setup("circuitBuilt");
      await server.write(
        "650 CIRC abc123 BUILT $1111111111111111111111111111111111111111\r\n"
      );
      Assert.deepEqual(await test, {
        id: "abc123",
        nodes: ["1111111111111111111111111111111111111111"],
      });
    }
    {
      const test = listener.setup("circuitBuilt");
      await server.write(
        "650 CIRC 456 BUILT $aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,$bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,$cccccccccccccccccccccccccccccccccccccccc\r\n"
      );
      Assert.deepEqual(await test, {
        id: "456",
        nodes: [
          "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
          "cccccccccccccccccccccccccccccccccccccccc",
        ],
      });
    }
    {
      const test = listener.setup("circuitBuilt");
      await server.write(
        "650 CIRC conflux BUILT $1111111111111111111111111111111111111111~guard,$2222222222222222222222222222222222222222~mid,$3333333333333333333333333333333333333333~exit CONFLUX_ID=aabbccddeeff00112233445566778899\r\n"
      );
      Assert.deepEqual(await test, {
        id: "conflux",
        nodes: [
          "1111111111111111111111111111111111111111",
          "2222222222222222222222222222222222222222",
          "3333333333333333333333333333333333333333",
        ],
        confluxId: "aabbccddeeff00112233445566778899",
      });
    }
  });
});

add_task(async function test_circuit_closed_notification() {
  await runTest(async (server, client) => {
    const listener = new NotificationListener();
    client.setEventHandler(listener);
    const test = listener.setup("circuitClosed");
    await server.write("650 CIRC qwerty CLOSED\r\n");
    Assert.equal(await test, "qwerty");
  });
});

add_task(async function test_sent_connect_notifications() {
  await runTest(async (server, client) => {
    const listener = new NotificationListener();
    client.setEventHandler(listener);
    {
      const test = listener.setup("streamSentConnect");
      await server.write("650 STREAM 1 SENTCONNECT 0 1.2.3.4:443\r\n");
      Assert.deepEqual(await test, {
        streamId: "1",
        circuitId: "0",
        username: null,
        password: null,
      });
    }
    {
      const test = listener.setup("streamSentConnect");
      await server.write(
        '650 STREAM 1ab SENTCONNECT xyz 1.2.3.4:443 SOCKS_USERNAME="user" SOCKS_PASSWORD="pass"\r\n'
      );
      Assert.deepEqual(await test, {
        streamId: "1ab",
        circuitId: "xyz",
        username: "user",
        password: "pass",
      });
    }
  });
});

add_task(async function test_closed_notification() {
  await runTest(async (server, client) => {
    const listener = new NotificationListener();
    client.setEventHandler(listener);
    const test = listener.setup("closed");
    server.close();
    await test;
  });
});
