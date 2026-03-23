/* Any copyright is dedicated to the Public Domain.
 * http://creativecommons.org/publicdomain/zero/1.0/ */

"use strict";

const { OxidationAdapters, PromisifiedCallback } = ChromeUtils.importESModule(
  "moz-src:///toolkit/components/tor-integration/OxidationAdapters.sys.mjs"
);

add_task(async function test_callback_happy() {
  {
    const cb = new PromisifiedCallback();
    cb.onReply("");
    Assert.strictEqual(await cb.promise, undefined);
  }
  {
    const cb = new PromisifiedCallback();
    cb.onReply("1234");
    Assert.strictEqual(await cb.promise, 1234);
  }
  {
    const cb = new PromisifiedCallback();
    cb.onReply('"1234"');
    Assert.strictEqual(await cb.promise, "1234");
  }
  {
    const cb = new PromisifiedCallback();
    cb.onReply('"a string"');
    Assert.strictEqual(await cb.promise, "a string");
  }
  {
    const cb = new PromisifiedCallback();
    cb.onReply('["an", "array"]');
    Assert.deepEqual(await cb.promise, ["an", "array"]);
  }
  {
    const cb = new PromisifiedCallback();
    cb.onReply('{"an": "object"}');
    Assert.deepEqual(await cb.promise, { an: "object" });
  }
});

add_task(async function test_callback_tor_error() {
  let hadError = false;
  try {
    const cb = new PromisifiedCallback();
    cb.onTorError(400, "Some error message");
    await cb.promise;
  } catch (e) {
    hadError = true;
    Assert.equal(e.code, 400);
    Assert.equal(e.message, "Some error message");
  }
  Assert.ok(hadError, "We did not get an error, but we expected one.");
});

add_task(async function test_callback_generic_error() {
  const message = "A more specific error message, maybe not machine-friendly.";
  let hadError = false;
  try {
    const cb = new PromisifiedCallback();
    cb.onGenericError(message);
    await cb.promise;
  } catch (e) {
    hadError = true;
    Assert.equal(e.message, message);
  }
  Assert.ok(hadError, "We did not get an error, but we expected one.");
});

add_task(function test_adapter_circuit() {
  Assert.deepEqual(
    OxidationAdapters.convertCircuit({
      id: "123",
      path: [
        [1, 2, 3],
        [4, 5, 6],
        [10, 16, 32],
      ],
    }),
    {
      id: "123",
      nodes: ["010203", "040506", "0a1020"],
    }
  );

  Assert.deepEqual(
    OxidationAdapters.convertCircuit({
      id: "abc",
      path: [
        [1, 2, 3],
        [4, 5, 6],
        [10, 16, 32],
      ],
      conflux_id: [0xaa, 0xbb, 0xcc, 0xe, 0xf, 1, 2, 3],
    }),
    {
      id: "abc",
      nodes: ["010203", "040506", "0a1020"],
      confluxId: "aabbcc0e0f010203",
    }
  );
});

add_task(function test_adapter_bootstrap_phase() {
  Assert.deepEqual(
    OxidationAdapters.convertBootstrapPhase({
      severity: "WARN",
      should_warn: true,
      progress: 25,
      tag: "orconn",
      summary: "Connection to an onion router",
      reason: "No route to host",
    }),
    {
      TYPE: "WARN",
      RECOMMENDATION: "warn",
      PROGRESS: 25,
      TAG: "orconn",
      SUMMARY: "Connection to an onion router",
      REASON: "No route to host",
    }
  );

  Assert.deepEqual(
    OxidationAdapters.convertBootstrapPhase({
      severity: "NOTICE",
      should_warn: false,
      progress: 75,
      tag: "loading_descriptors",
      summary: "Loading relay descriptors",
    }),
    {
      TYPE: "NOTICE",
      RECOMMENDATION: "ignore",
      PROGRESS: 75,
      TAG: "loading_descriptors",
      SUMMARY: "Loading relay descriptors",
    }
  );
});

add_task(function test_adapter_bridge() {
  Assert.deepEqual(
    OxidationAdapters.convertBridge({ address: "192.168.1.3:443" }),
    {
      transport: "vanilla",
      addr: "192.168.1.3:443",
    }
  );

  Assert.deepEqual(
    OxidationAdapters.convertBridge({ address: "[2001:dead::beef]:9010" }),
    {
      transport: "vanilla",
      addr: "[2001:dead::beef]:9010",
    }
  );

  Assert.deepEqual(
    OxidationAdapters.convertBridge({
      transport: "meek",
      address: "192.168.1.3:443",
    }),
    {
      transport: "meek",
      addr: "192.168.1.3:443",
    }
  );

  Assert.deepEqual(
    OxidationAdapters.convertBridge({
      transport: "meek",
      address: "192.168.1.3:443",
      args: null,
    }),
    {
      transport: "meek",
      addr: "192.168.1.3:443",
    }
  );

  Assert.deepEqual(
    OxidationAdapters.convertBridge({
      transport: "meek",
      address: "192.168.1.3:443",
      args: "",
    }),
    {
      transport: "meek",
      addr: "192.168.1.3:443",
    }
  );

  Assert.deepEqual(
    OxidationAdapters.convertBridge({
      transport: "meek",
      address: "192.168.1.3:443",
      fingerprint: null,
    }),
    {
      transport: "meek",
      addr: "192.168.1.3:443",
    }
  );

  Assert.deepEqual(
    OxidationAdapters.convertBridge({
      transport: "snowflake",
      address: "192.168.1.4:443",
      args: "some=param another=one",
    }),
    {
      transport: "snowflake",
      addr: "192.168.1.4:443",
      args: "some=param another=one",
    }
  );

  Assert.deepEqual(
    OxidationAdapters.convertBridge({
      transport: "obfs4",
      address: "192.168.1.5:443",
      fingerprint: "010203cafe",
      args: "some=param another=one",
    }),
    {
      transport: "obfs4",
      addr: "192.168.1.5:443",
      id: "010203cafe",
      args: "some=param another=one",
    }
  );

  Assert.deepEqual(
    OxidationAdapters.convertBridge({
      transport: "obfs4",
      address: "192.168.1.5:443",
      fingerprint: "010203cafe",
    }),
    {
      transport: "obfs4",
      addr: "192.168.1.5:443",
      id: "010203cafe",
    }
  );
});

add_task(function test_adapter_pluggable_transports() {
  Assert.deepEqual(OxidationAdapters.convertPluggableTransports([]), []);

  Assert.deepEqual(
    OxidationAdapters.convertPluggableTransports([
      {
        transports: ["obfs4", "meek_lite"],
        type: "exec",
        pathToBinary: "TorBrowser/Tor/PluggableTransports/lyrebird",
      },
      {
        transports: ["snowflake"],
        type: "exec",
        pathToBinary: "TorBrowser/Tor/PluggableTransports/lyrebird",
        options: "some=option",
        ip: null,
        port: null,
      },
      {
        transports: ["conjure"],
        type: "exec",
        pathToBinary: "TorBrowser/Tor/PluggableTransports/conjure-client",
        // Ignored even when not null.
        ip: "10.0.2.2",
        port: 80,
      },
      {
        transports: ["proxy1"],
        type: "socks4",
        pathToBinary: null,
        ip: "10.0.2.2",
        port: 80,
      },
      {
        transports: ["proxy2"],
        type: "socks5",
        pathToBinary: "an/ignored/binary",
        ip: "[::1]",
        port: 80,
      },
    ]),
    [
      {
        transports: ["obfs4", "meek_lite"],
        type: "exec",
        pathToBinary: "TorBrowser/Tor/PluggableTransports/lyrebird",
        options: "",
      },
      {
        transports: ["snowflake"],
        type: "exec",
        pathToBinary: "TorBrowser/Tor/PluggableTransports/lyrebird",
        options: "some=option",
      },
      {
        transports: ["conjure"],
        type: "exec",
        pathToBinary: "TorBrowser/Tor/PluggableTransports/conjure-client",
        options: "",
      },
      {
        transports: ["proxy1"],
        type: "socks4",
        ip: "10.0.2.2",
        port: 80,
      },
      {
        transports: ["proxy2"],
        type: "socks5",
        ip: "[::1]",
        port: 80,
      },
    ]
  );
});

add_task(function test_adapter_onion_keys() {
  const address = "PG6MMJIYJMCRSSLVYKFWNNTLARU7P5SVN6Y2YMMJU6NUBXNDF4PSCRYD";
  const keyType = "x25519";
  const keyBlob = "abase64key";

  Assert.deepEqual(OxidationAdapters.convertOnionKeys([]), []);

  Assert.deepEqual(
    OxidationAdapters.convertOnionKeys([
      {
        address,
        key_type: keyType,
        key_blob: keyBlob,
        is_permanent: false,
      },
      {
        address,
        key_type: keyType,
        key_blob: keyBlob,
        is_permanent: true,
      },
    ]),
    [
      {
        address,
        keyType,
        keyBlob,
        flags: [],
      },
      {
        address,
        keyType,
        keyBlob,
        flags: ["Permanent"],
      },
    ]
  );
});
