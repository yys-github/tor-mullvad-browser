/* Any copyright is dedicated to the Public Domain.
 * http://creativecommons.org/publicdomain/zero/1.0/ */

"use strict";

async function runTest(func) {
  info(`Running ${func.name} with TCP`);
  await func("tcp");
  if (IPC_SUPPORTED) {
    info(`Running ${func.name} with Unix-domain sockets`);
    await func("ipc");
  }
}

add_task(async function test_simple() {
  await runTest(async function testSimple(protocol) {
    const server = new ControlPortServer(protocol, async transport => {
      const s = await transport.readLine();
      Assert.equal(s, "TEST");
      await transport.write("250 OK\r\n");
      transport.close();
    });

    let cp;
    try {
      cp = new ControlPortClient(server);
      const reply = await cp.sendCommand("TEST");
      Assert.equal(reply, "250 OK");
    } finally {
      cp?.close();
      server.close();
    }
  });
});

add_task(async function test_invalidSyntax() {
  await runTest(async function testInvalidSyntax(protocol) {
    const server = new ControlPortServer(protocol, async transport => {
      await transport.read();
      await transport.write("250|OK\r\n");
      transport.close();
    });

    let failed = true;
    let cp;
    try {
      cp = new ControlPortClient(server);
      await cp.sendCommand("TEST");
    } catch (e) {
      Assert.ok(true, `We got the expected failure: ${e.message}`);
    } finally {
      try {
        cp.close();
      } catch (e) {
        info(`The control port threw when closing: ${e.message}.`);
      }
    }
    Assert.ok(failed, "sendCommand threw when we expected it to.");

    server.close();
  });
});

/**
 * A specialized control port client for testing async messages.
 */
class ControlPortClientAsyncNotification extends ControlPortClient {
  notificationPromise;
  #resolve;

  constructor(server) {
    super(server);
    const { promise, resolve } = Promise.withResolvers();
    this.notificationPromise = promise;
    this.#resolve = resolve;
  }

  onAsyncMessage(message) {
    Assert.equal(message, "650-Test\r\n650 notification");
    this.#resolve();
  }
}

add_task(async function test_asyncNotification() {
  await runTest(async function testAsyncNotification(protocol) {
    const server = new ControlPortServer(protocol, async transport => {
      await transport.write("650-Test\r\n650 notification\r\n");
      transport.close();
    });
    const cp = new ControlPortClientAsyncNotification(server);
    await cp.notificationPromise;
    cp.close();
    server.close();
  });
});

add_task(async function test_failureAfterClosing() {
  await runTest(async function testFailureAfterClosing(protocol) {
    {
      let cp;
      const { promise, resolve, reject } = Promise.withResolvers();
      const server = new ControlPortServer(protocol, async transport => {
        transport.close();
        server.close();

        try {
          await cp.sendCommand("TEST");
          reject("We managed to send a command after closing the connection.");
        } catch (e) {
          Assert.ok(
            true,
            `We failed to send the command as expected (${e.message})`
          );
        } finally {
          try {
            cp?.close();
          } catch {}
        }
        resolve();
      });
      try {
        cp = new ControlPortClient(server);
      } catch {
        server.close();
      }
      await promise;
    }

    {
      const server = new ControlPortServer(protocol, async transport => {
        try {
          const s = await transport.readLine();
          Assert.equal(s, "TEST");
          await transport.write("250 OK\r\n");
        } catch {
          // We do not really care if the client manages to close the socket
          // before we handle it.
        }
        transport.close();
      });

      const cp = new ControlPortClient(server);
      cp.close();
      let sentCommand = false;
      try {
        await cp.sendCommand("TEST");
        sentCommand = true;
      } catch (e) {
        Assert.ok(
          true,
          `We failed to send the command as expected (${e.message})`
        );
      } finally {
        server.close();
      }
      Assert.ok(
        !sentCommand,
        "We did not send the command as we had already closed the connection"
      );
    }
  });
});
