// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::Bytes;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

use super::control_socket::{ControlSocket, ControlSocketError};

type WriteCallback = Box<dyn FnOnce(Result<(), ControlSocketError>)>;

/// A command writer that will handle partial writes.
///
/// We cannot rely on Write's write_all as it is blocking, whereas our socket
/// implementation is async.
///
/// We have a small struct only for this for easiness of testing.
pub(super) struct CommandWriter {
    socket: Rc<dyn ControlSocket>,
    /// This locks the writing mechanism.
    /// If this is true when write is called, the command will be queued in an
    /// internal queue. Otherwise, it will be scheduled for writing immediately.
    /// This allows us to handle partial writes.
    /// As an alternative, we could have kept the item being worked here, but
    /// moving it around rather than keeping a state seemed easier to reason
    /// about.
    write_locked: Cell<bool>,
    command_queue: RefCell<VecDeque<(Bytes, WriteCallback)>>,
}

impl CommandWriter {
    pub fn new(socket: Rc<dyn ControlSocket>) -> Rc<Self> {
        Rc::new(Self {
            socket,
            write_locked: Cell::new(false),
            command_queue: RefCell::new(VecDeque::new()),
        })
    }

    /// Write a command on the socket asynchronously.
    /// The provided callback will be called before continuing processing the
    /// write queue.
    /// Since the callers will need to handle potential async errors, also sync
    /// errors are signaled through the callback.
    pub fn write(self: &Rc<Self>, data: Bytes, callback: WriteCallback) {
        // This should really never happen: the writer is used only in the
        // control port to send commands, which will always have at least two
        // bytes (the final CRLF).
        // So, use an assertion rather than an error code here.
        debug_assert!(!data.is_empty());
        if self.write_locked.replace(true) {
            self.command_queue.borrow_mut().push_back((data, callback));
        } else {
            self.schedule_write(data, callback);
        }
    }

    fn maybe_schedule_write(self: &Rc<Self>) {
        debug_assert!(
            !self.write_locked.get(),
            "We were called before the write lock was released!"
        );
        let was_locked = self.write_locked.replace(true);
        if was_locked {
            return;
        }
        let (data, callback) = match self.command_queue.borrow_mut().pop_front() {
            Some(pair) => pair,
            None => {
                self.write_locked.set(false);
                return;
            }
        };
        self.schedule_write(data, callback);
    }

    fn schedule_write(self: &Rc<Self>, data: Bytes, callback: WriteCallback) {
        debug_assert!(
            self.write_locked.get(),
            "schedule_write was called without holding the write lock!"
        );
        // We use a Rc because as a matter of fact we need to be accessed also
        // by the socket, but it does not make sense that this makes us outlive
        // our actual (logically single) owner, therefore use a weak ref.
        let self_weak = Rc::downgrade(self);
        let len = data.len().min(u32::MAX as usize) as u32;
        self.socket.queue_write(
            Box::new(move |res| match res {
                Ok(()) => {
                    if let Some(this) = self_weak.upgrade() {
                        this.ready_to_write(data, callback);
                    }
                }
                Err(e) => {
                    if let Some(this) = self_weak.upgrade() {
                        // Unlock for correctness, but queue errors should be
                        // treated as fatal errors.
                        this.write_locked.set(false);
                    }
                    callback(Err(e));
                }
            }),
            len,
        );
    }

    fn ready_to_write(self: &Rc<Self>, mut data: Bytes, callback: WriteCallback) {
        debug_assert!(
            self.write_locked.get(),
            "ready_to_write was called even though the write mechanism was not locked!"
        );

        match self.socket.write(&data) {
            Ok(len) => {
                let len = len as usize;
                // This looks like recursion, but it is not: there is the async
                // waiting in the middle. We rely on the implementation to post
                // the write request to its event queue (or something similar).
                // So, this is the best way we have to implement this.
                if len == data.len() {
                    self.write_locked.set(false);
                    callback(Ok(()));
                    self.maybe_schedule_write();
                } else if len > 0 {
                    debug_assert!(len < data.len());
                    self.schedule_write(data.split_off(len), callback);
                } else {
                    // Notice: we do not treat len == 0 as an error, but as a
                    // no-space situation (and we assume underlying sockets are
                    // well behaved, and will not transform this into an
                    // infinite loop, but the async wait will actually wait for
                    // some memory to be available).
                    self.schedule_write(data, callback);
                }
            }
            Err(e) => {
                // Notice that a write error should be considered a fatal error.
                self.write_locked.set(false);
                callback(Err(e));
            }
        }
    }

    pub fn clear_queue(&self) {
        self.command_queue.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSocket {
        next_write: RefCell<Option<Box<dyn FnOnce(Result<(), ControlSocketError>)>>>,
        to_write: Cell<u32>,

        write_called: Cell<bool>,
        done: Cell<bool>,

        should_queue_fail: Cell<bool>,
        should_write_fail: Cell<bool>,
        expected_bytes: RefCell<Bytes>,
        writer_was_dropped: Cell<bool>,
    }

    impl MockSocket {
        fn new() -> Self {
            Self {
                next_write: RefCell::new(None),
                to_write: Cell::default(),
                write_called: Cell::new(false),
                done: Cell::new(false),
                should_queue_fail: Cell::new(false),
                should_write_fail: Cell::new(false),
                expected_bytes: RefCell::default(),
                writer_was_dropped: Cell::new(false),
            }
        }

        fn call_next(&self) {
            assert!(!self.write_called.get());
            let next = self.next_write.borrow_mut().take().unwrap();
            (next)(Ok(()));
            assert!(self.write_called.get() || self.writer_was_dropped.get());
            self.write_called.set(false);
        }

        fn has_next(&self) -> bool {
            self.next_write.borrow().is_some()
        }

        fn set_done(&self) {
            assert!(self.next_write.borrow().is_none());
            self.done.set(true);
        }

        fn set_to_write(&self, b: u32) {
            self.to_write.set(b);
        }

        fn set_expected_bytes(&self, b: Bytes) {
            *self.expected_bytes.borrow_mut() = b;
        }

        fn enable_queue_failures(&self) {
            self.should_queue_fail.set(true);
        }

        fn enable_write_failures(&self) {
            self.should_write_fail.set(true);
        }

        fn writer_dropped(&self) {
            self.writer_was_dropped.set(true);
        }
    }

    impl ControlSocket for MockSocket {
        fn queue_read(&self, _f: Box<dyn FnOnce()>) -> Result<(), ControlSocketError> {
            unreachable!("Write tests are not supposed to read.");
        }

        fn available(&self) -> Result<u32, ControlSocketError> {
            unreachable!("Write tests should not need to call available.")
        }

        fn read(&self, _max_len: u32) -> Result<Bytes, ControlSocketError> {
            unreachable!("Write tests are not supposed to read.");
        }

        fn queue_write(&self, f: Box<dyn FnOnce(Result<(), ControlSocketError>)>, _len: u32) {
            if self.should_queue_fail.get() {
                f(Err(ControlSocketError::ImplementationError(0xDEADBEEF)));
                return;
            }

            assert!(
                self.next_write.borrow().is_none(),
                "We expect to queue a write only after the previous one finished."
            );
            *self.next_write.borrow_mut() = Some(f);
        }

        fn write(&self, buffer: &Bytes) -> Result<u32, ControlSocketError> {
            self.write_called.set(true);

            if self.should_write_fail.get() {
                return Err(ControlSocketError::ImplementationError(0xCAFECAFE));
            }

            let to_write = self.to_write.get();
            // We do not test for badly behaved socket.
            assert!(to_write as usize <= buffer.len());
            assert_eq!(
                *buffer,
                *self.expected_bytes.borrow(),
                "We received the bytes we expected."
            );
            Ok(to_write)
        }

        fn close(&self) -> Result<(), ControlSocketError> {
            if !self.should_queue_fail.get() && !self.should_write_fail.get() && !self.done.get() {
                panic!("close called before we were done!");
            }
            Ok(())
        }
    }

    fn test_simple_write(
        socket: &Rc<MockSocket>,
        writer: &Rc<CommandWriter>,
        cmd: &'static [u8],
        write: bool,
    ) {
        let cmd = Bytes::from_static(cmd);
        socket.set_expected_bytes(cmd.clone());
        socket.set_to_write(cmd.len() as u32);
        let called = Rc::new(Cell::new(false));
        if write {
            let c = called.clone();
            writer.write(
                cmd,
                Box::new(move |r| {
                    assert!(r.is_ok());
                    c.set(true);
                }),
            );
            assert!(!called.get());
        }
        socket.call_next();
        assert_eq!(called.get(), write);
    }

    #[test]
    fn basic_write() {
        let socket = Rc::new(MockSocket::new());
        let writer = CommandWriter::new(socket.clone());
        test_simple_write(&socket, &writer, b"TEST\r\n", true);
        socket.set_done();
    }

    #[test]
    fn multiple_writes() {
        let socket = Rc::new(MockSocket::new());
        let writer = CommandWriter::new(socket.clone());
        test_simple_write(&socket, &writer, b"CMD 1\r\n", true);
        test_simple_write(&socket, &writer, b"CMD 2\r\n", true);
        socket.set_done();
    }

    #[test]
    fn queue_until_done() {
        let socket = Rc::new(MockSocket::new());
        let writer = CommandWriter::new(socket.clone());

        let cmd1 = b"CMD 1\r\n";
        let cmd2 = b"CMD 2 aaaaaa\r\n";
        let cmd3 = b"CMD 3 bbb\r\n";
        let called1 = Rc::new(Cell::new(false));
        let c1 = called1.clone();
        let called2 = Rc::new(Cell::new(false));
        let c2 = called2.clone();
        let called3 = Rc::new(Cell::new(false));
        let c3 = called3.clone();
        writer.write(
            Bytes::from_static(cmd1),
            Box::new(move |r| {
                assert!(r.is_ok());
                c1.set(true);
            }),
        );
        // The following ones should be queued inside the CommandWriter, rather
        // than in the socket.
        writer.write(
            Bytes::from_static(cmd2),
            Box::new(move |r| {
                assert!(r.is_ok());
                c2.set(true);
            }),
        );
        writer.write(
            Bytes::from_static(cmd3),
            Box::new(move |r| {
                assert!(r.is_ok());
                c3.set(true);
            }),
        );
        test_simple_write(&socket, &writer, cmd1, false);
        test_simple_write(&socket, &writer, cmd2, false);
        test_simple_write(&socket, &writer, cmd3, false);
        assert!(called1.get());
        assert!(called2.get());
        assert!(called3.get());
    }

    #[test]
    fn partial_write() {
        let socket = Rc::new(MockSocket::new());
        let writer = CommandWriter::new(socket.clone());

        let called1 = Rc::new(Cell::new(false));
        let c1 = called1.clone();
        let called2 = Rc::new(Cell::new(false));
        let c2 = called2.clone();
        let called3 = Rc::new(Cell::new(false));
        let c3 = called3.clone();

        // When we do a partial write, we expect the CommandWriter to queue the
        // message until completely transmitted...
        let cmd1 = Bytes::from_static(b"CMD 1 111111111111\r\n");
        socket.set_expected_bytes(cmd1.clone());
        socket.set_to_write(2);
        writer.write(
            cmd1.clone(),
            Box::new(move |r| {
                assert!(r.is_ok());
                c1.set(true);
            }),
        );
        socket.call_next();
        assert!(!called1.get());

        let cmd1 = cmd1.slice(2..);
        socket.set_expected_bytes(cmd1.clone());
        socket.set_to_write(cmd1.len() as u32);
        socket.call_next();
        assert!(called1.get());

        // ... even when we schedule multiple commands.
        let cmd2 = Bytes::from_static(b"CMD 2\r\n");
        let cmd3 = Bytes::from_static(b"CMD 3 test\r\n");
        socket.set_expected_bytes(cmd2.clone());
        socket.set_to_write(3);
        writer.write(
            cmd2.clone(),
            Box::new(move |r| {
                assert!(r.is_ok());
                c2.set(true);
            }),
        );
        writer.write(
            cmd3.clone(),
            Box::new(move |r| {
                assert!(r.is_ok());
                c3.set(true);
            }),
        );
        socket.call_next();
        assert!(!called2.get());
        assert!(!called3.get());

        let cmd2 = cmd2.slice(3..);
        socket.set_expected_bytes(cmd2.clone());
        socket.set_to_write(cmd2.len() as u32);
        socket.call_next();
        assert!(called2.get());
        assert!(!called3.get());

        socket.set_expected_bytes(cmd3.clone());
        socket.set_to_write(cmd3.len() as u32);
        // cmd3 is already scheduled, no need to call write again.
        socket.call_next();
        assert!(called3.get());

        socket.set_done();
    }

    #[test]
    fn zero_write() {
        let socket = Rc::new(MockSocket::new());
        let writer = CommandWriter::new(socket.clone());

        // Test that write returning 0 bytes written is handled, rather than
        // being treated as an error.
        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        let cmd = Bytes::from_static(b"TEST\r\n");
        socket.set_expected_bytes(cmd.clone());
        socket.set_to_write(0);
        writer.write(
            cmd.clone(),
            Box::new(move |r| {
                assert!(r.is_ok());
                c.set(true);
            }),
        );
        socket.call_next();
        assert!(!called.get());
        socket.set_to_write(cmd.len() as u32);
        socket.call_next();
        assert!(called.get());
        socket.set_done();
    }

    #[test]
    fn clear_queue() {
        let socket = Rc::new(MockSocket::new());
        let writer = CommandWriter::new(socket.clone());

        let cmd1 = b"CMD 1\r\n";
        let cmd2 = b"CMD 2\r\n";

        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        writer.write(
            Bytes::from_static(cmd1),
            Box::new(move |r| {
                assert!(r.is_ok());
                c.set(true);
            }),
        );

        writer.write(
            Bytes::from_static(cmd2),
            Box::new(move |_| {
                unreachable!("cmd2 is canceled, we should never be called.");
            }),
        );
        writer.clear_queue();

        test_simple_write(&socket, &writer, cmd1, false);
        assert!(called.get(), "cmd1 was completed");
        assert!(
            !socket.has_next(),
            "The socket does not have more pending calls."
        );
        socket.set_done();
    }

    #[test]
    fn queued_write_cannot_be_cancelled() {
        let socket = Rc::new(MockSocket::new());
        let writer = CommandWriter::new(socket.clone());

        let cmd1 = b"CMD 1 12345\r\n";
        let cmd2 = b"CMD 2 7777777\r\n";

        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        let cmd1_bytes = Bytes::from_static(cmd1);
        socket.set_expected_bytes(cmd1_bytes.clone());
        socket.set_to_write(1);
        writer.write(
            cmd1_bytes.clone(),
            Box::new(move |r| {
                assert!(r.is_ok());
                c.set(true);
            }),
        );
        writer.write(
            Bytes::from_static(cmd2),
            Box::new(move |_| {
                unreachable!("The second command should be canceled!");
            }),
        );
        assert!(socket.has_next(), "We have a pending item in the socket.");
        writer.clear_queue();
        assert!(
            socket.has_next(),
            "Clearing writer's queue does not influence the socket."
        );

        // Process the partial write for cmd1.
        socket.call_next();
        assert!(
            !called.get(),
            "We simulated a partial write, the callback should not have been called yet."
        );
        assert!(
            socket.has_next(),
            "The socket has an item to possibly complete cmd1."
        );

        // Complete the remaining write of cmd1
        let cmd1_remaining = cmd1_bytes.slice(1..);
        socket.set_expected_bytes(cmd1_remaining);
        socket.set_to_write(cmd1.len() as u32 - 1);
        socket.call_next();
        assert!(
            called.get(),
            "The callback for cmd1 should have been called at this point."
        );

        assert!(
            !socket.has_next(),
            "We cleared the queue, the socket should not have an item for cmd2!"
        );

        socket.set_done();
    }

    #[test]
    fn queue_failure() {
        // Fail immediately
        {
            let socket = Rc::new(MockSocket::new());
            let writer = CommandWriter::new(socket.clone());
            socket.enable_queue_failures();
            let called = Rc::new(Cell::new(false));
            let c = called.clone();
            writer.write(
                Bytes::from_static(b"TEST\r\n"),
                Box::new(move |r| {
                    assert!(r.is_err());
                    c.set(true);
                }),
            );
            assert!(called.get());
        }

        // Fail after being queued in the writer.
        {
            let socket = Rc::new(MockSocket::new());
            let writer = CommandWriter::new(socket.clone());
            let first = Rc::new(Cell::new(false));
            let f = first.clone();
            let second = Rc::new(Cell::new(false));
            let s = second.clone();
            static CMD1: &[u8] = b"CMD 1 aaaaaaaaaaaaaa\r\n";
            writer.write(
                Bytes::from_static(CMD1),
                Box::new(move |r| {
                    assert!(r.is_ok());
                    f.set(true);
                }),
            );
            socket.enable_queue_failures();
            writer.write(
                Bytes::from_static(b"CMD 2\r\n"),
                Box::new(move |r| {
                    assert!(r.is_err());
                    s.set(true);
                }),
            );
            test_simple_write(&socket, &writer, CMD1, false);
            assert!(first.get());
            assert!(second.get());
        }
    }

    #[test]
    fn write_failure() {
        let socket = Rc::new(MockSocket::new());
        let writer = CommandWriter::new(socket.clone());
        socket.enable_write_failures();
        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        writer.write(
            Bytes::from_static(b"TEST\r\n"),
            Box::new(move |r| {
                assert!(r.is_err());
                c.set(true);
            }),
        );
        socket.call_next();
        assert!(called.get());
    }

    #[test]
    fn ready_after_drop() {
        let socket = Rc::new(MockSocket::new());
        let writer = CommandWriter::new(socket.clone());
        let cmd = Bytes::from_static(b"TEST\r\n");
        socket.set_expected_bytes(cmd.clone());
        socket.set_to_write(cmd.len() as u32);
        writer.write(
            cmd,
            Box::new(|_| {
                unreachable!("We expected the write callback not to be called.");
            }),
        );
        drop(writer);
        socket.writer_dropped();
        socket.call_next();
    }

    #[test]
    fn dropped_during_partial_write() {
        let socket = Rc::new(MockSocket::new());
        let writer = CommandWriter::new(socket.clone());

        let cmd = Bytes::from_static(b"CMD a very long one\r\n");
        socket.set_expected_bytes(cmd.clone());
        socket.set_to_write(5);

        writer.write(
            cmd.clone(),
            Box::new(move |_| {
                unreachable!("The callback should never be called, as we drop the writer before.");
            }),
        );

        // Trigger the partial write
        socket.call_next();
        assert!(
            socket.has_next(),
            "The socket should have a queued item to complete the writing."
        );

        drop(writer);
        socket.writer_dropped();
        socket.call_next();

        socket.set_done();
    }
}
