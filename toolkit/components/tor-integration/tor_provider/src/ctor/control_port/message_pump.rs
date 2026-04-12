// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::Bytes;
use std::{cell::Cell, rc::Rc};

use super::control_socket::{ControlSocket, ControlSocketError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReadAction {
    Continue,
    Stop,
}

pub(super) struct MessagePump {
    socket: Rc<dyn ControlSocket>,
    data_handler: Box<dyn Fn(Bytes) -> ReadAction>,
    error_handler: Box<dyn Fn()>,
    started: Cell<bool>,
}

impl MessagePump {
    /// Create a new MessagePump instance.
    /// The handlers will not be used until the start method is called.
    pub fn new(
        socket: Rc<dyn ControlSocket>,
        data_handler: Box<dyn Fn(Bytes) -> ReadAction>,
        error_handler: Box<dyn Fn()>,
    ) -> Rc<Self> {
        Rc::new(Self {
            socket,
            data_handler,
            error_handler,
            started: Cell::new(false),
        })
    }

    pub fn start(self: &Rc<Self>) -> Result<(), ControlSocketError> {
        if self.started.replace(true) {
            debug_assert!(false, "MessagePump started twice!");
            return Ok(());
        }
        self.queue_read()
    }

    fn queue_read(self: &Rc<Self>) -> Result<(), ControlSocketError> {
        // We use a Rc because as a matter of fact we need to be accessed also
        // by the socket, but it does not make sense that this makes us outlive
        // our actual (logically single) owner, therefore use a weak ref.
        let self_weak = Rc::downgrade(self);
        self.socket.queue_read(Box::new(move || {
            if let Some(mp) = self_weak.upgrade() {
                if mp.read().is_err() {
                    (mp.error_handler)();
                }
            }
        }))
    }

    fn read(self: &Rc<Self>) -> Result<(), ControlSocketError> {
        let available = self.socket.available()?;
        if available == 0 {
            return Err(ControlSocketError::ConnectionClosed);
        }
        // Notice this might read less than what available returned.
        // We prefer queuing a separate read, in case we cannot really rely on
        // available (but we hope we can and we will usually read all).
        // In addition to that, we rely on the other layers to do the proper
        // buffering and partial read handling (especially ReplyDispatcher).
        let buf = self.socket.read(available)?;
        if buf.is_empty() {
            // As described in the trait, an empty response is intended as an
            // EOL/connection closed.
            return Err(ControlSocketError::ConnectionClosed);
        }

        let action = (self.data_handler)(buf);
        if action == ReadAction::Continue {
            self.queue_read()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ReadError {
        ExplicitError,
        EmptyBuffer,
        None,
    }

    struct MockSocket {
        next_read: RefCell<Option<Box<dyn FnOnce()>>>,
        to_read: RefCell<Bytes>,

        read_called: Cell<bool>,
        done: Cell<bool>,
        close_called: Cell<bool>,

        should_queue_fail: Cell<bool>,
        should_read_fail: Cell<ReadError>,
        panic_on_queue: Cell<bool>,
        pump_was_dropped: Cell<bool>,
    }

    impl MockSocket {
        fn new() -> Self {
            Self {
                next_read: RefCell::new(None),
                to_read: RefCell::default(),
                read_called: Cell::new(false),
                done: Cell::new(false),
                close_called: Cell::new(false),
                should_queue_fail: Cell::new(false),
                should_read_fail: Cell::new(ReadError::None),
                panic_on_queue: Cell::new(false),
                pump_was_dropped: Cell::new(false),
            }
        }

        fn has_pending(&self) -> bool {
            self.next_read.borrow().is_some()
        }

        fn set_to_read(&self, data: Bytes) {
            *self.to_read.borrow_mut() = data;
        }

        fn call_next(&self) {
            assert!(!self.read_called.get(), "");
            let next = self.next_read.borrow_mut().take().unwrap();
            (next)();
            assert!(
                self.read_called.get() || self.close_called.get() || self.pump_was_dropped.get(),
                "After a queue dispatch, read was called or the socket was closed."
            );
            self.read_called.set(false);
        }

        fn set_done(&self) {
            self.done.set(true);
        }

        fn server_close(&self) {
            // Set this immediately to make other operations fail.
            // The message pump should not call to read, as instead it should
            // catch the EOF by calling available.
            self.close_called.set(true);

            // Call anything left in the queue, so that it receives 0 as
            // available data length (EOF).
            self.set_to_read(Bytes::new());
            if self.has_pending() {
                self.call_next();
            }
        }

        fn is_closed(&self) -> bool {
            self.close_called.get()
        }

        fn enable_queue_failures(&self) {
            self.should_queue_fail.set(true);
        }

        fn set_read_failures(&self, value: ReadError) {
            self.should_read_fail.set(value);
        }

        fn should_not_queue_anymore(&self) {
            self.panic_on_queue.set(true);
        }

        fn pump_dropped(&self) {
            self.pump_was_dropped.set(true);
        }
    }

    impl ControlSocket for MockSocket {
        fn queue_read(&self, f: Box<dyn FnOnce()>) -> Result<(), ControlSocketError> {
            assert!(
                !self.close_called.get(),
                "Should not queue a read if the socked was closed."
            );
            assert!(
                !self.panic_on_queue.get(),
                "Should queue only when queuing is enabled."
            );
            assert!(
                self.next_read.borrow().is_none(),
                "We expect to queue a read only after the previous one finished."
            );

            if self.should_queue_fail.get() {
                return Err(ControlSocketError::ImplementationError(0xDEADBEEF));
            }

            *self.next_read.borrow_mut() = Some(f);
            Ok(())
        }

        fn available(&self) -> Result<u32, ControlSocketError> {
            Ok(self.to_read.borrow().len() as u32)
        }

        fn read(&self, max_len: u32) -> Result<Bytes, ControlSocketError> {
            assert!(
                !self.close_called.get(),
                "Should not read afater the socket was closed."
            );
            self.read_called.set(true);

            match self.should_read_fail.get() {
                ReadError::ExplicitError => {
                    return Err(ControlSocketError::ImplementationError(0xCAFECAFE));
                }
                ReadError::EmptyBuffer => {
                    return Ok(Bytes::default());
                }
                ReadError::None => {}
            }

            let max_len = max_len as usize;
            assert!(
                max_len <= self.to_read.borrow().len(),
                "Should read at most the number of bytes we report as available."
            );
            Ok(self.to_read.borrow().slice(0..max_len))
        }

        fn queue_write(&self, _f: Box<dyn FnOnce(Result<(), ControlSocketError>)>, _len: u32) {
            unreachable!("Read tests are not supposed to write.");
        }

        fn write(&self, _buffer: &Bytes) -> Result<u32, ControlSocketError> {
            unreachable!("Read tests are not supposed to write.");
        }

        fn close(&self) -> Result<(), ControlSocketError> {
            if !self.should_queue_fail.get()
                && self.should_read_fail.get() != ReadError::None
                && !self.done.get()
            {
                panic!("close called before we were done!");
            }
            self.server_close();
            Ok(())
        }
    }

    #[test]
    fn test_happy() {
        let test_data = b"TEST\r\n";
        let data_called = Rc::new(Cell::new(false));
        let dc = data_called.clone();
        let socket = Rc::new(MockSocket::new());
        let mp = MessagePump::new(
            socket.clone(),
            Box::new(move |data| {
                assert_eq!(data, Bytes::from_static(test_data));
                dc.set(true);
                ReadAction::Continue
            }),
            Box::new(|| {
                unreachable!("We should not need the async error handler in this test.");
            }),
        );
        assert!(!socket.has_pending());
        assert!(mp.start().is_ok());
        assert!(socket.has_pending());
        socket.set_to_read(Bytes::from_static(test_data));
        socket.call_next();
        assert!(socket.has_pending());
        socket.set_done();
    }

    #[test]
    fn server_closed() {
        let eh_called = Rc::new(Cell::new(false));
        let ehc = eh_called.clone();
        let socket = Rc::new(MockSocket::new());
        let mp = MessagePump::new(
            socket.clone(),
            Box::new(|_data| {
                unreachable!("In this test we close immediately the socket, we should not call the data callback!");
            }),
            Box::new(move || {
                ehc.set(true);
            }),
        );
        assert!(mp.start().is_ok());
        socket.server_close();
        assert!(eh_called.get());
        assert!(socket.is_closed());
        socket.set_done();
    }

    #[test]
    fn queue_failure() {
        {
            let socket = Rc::new(MockSocket::new());
            let mp = MessagePump::new(
                socket.clone(),
                Box::new(|_data| {
                    unreachable!(
                        "In this test we fail immediately, we should not call the data callback!"
                    );
                }),
                Box::new(|| {
                    panic!("The failure is immediately, it should not use this function.");
                }),
            );
            socket.enable_queue_failures();
            assert!(mp.start().is_err());
            socket.set_done();
        }

        {
            let eh_called = Rc::new(Cell::new(false));
            let ehc = eh_called.clone();
            let socket = Rc::new(MockSocket::new());
            let mp = MessagePump::new(
                socket.clone(),
                Box::new(|_data| ReadAction::Continue),
                Box::new(move || {
                    ehc.set(true);
                }),
            );
            assert!(mp.start().is_ok());
            socket.enable_queue_failures();
            socket.set_to_read(Bytes::from_static(b"250 OK\r\n"));
            socket.call_next();
            assert!(eh_called.get());
            socket.set_done();
        }
    }

    #[test]
    fn read_error() {
        let test = |read_error| {
            let eh_called = Rc::new(Cell::new(false));
            let ehc = eh_called.clone();
            let socket = Rc::new(MockSocket::new());
            let mp = MessagePump::new(
                socket.clone(),
                Box::new(|_data| {
                    unreachable!(
                        "In this test we fail to read, we should not call the data callback!"
                    );
                }),
                Box::new(move || {
                    ehc.set(true);
                }),
            );
            assert!(mp.start().is_ok());
            socket.set_read_failures(read_error);
            // This is so that available does not return 0.
            socket.set_to_read(Bytes::from_static(b"250 OK\r\n"));
            socket.call_next();
            assert!(eh_called.get());
            socket.set_done();
        };
        test(ReadError::ExplicitError);
        test(ReadError::EmptyBuffer);
    }

    #[test]
    fn simulate_parse_error() {
        // Fail on the first read
        {
            let socket = Rc::new(MockSocket::new());
            let s = socket.clone();
            let mp = MessagePump::new(
                socket.clone(),
                Box::new(move |_data| {
                    s.should_not_queue_anymore();
                    ReadAction::Stop
                }),
                Box::new(move || {
                    unreachable!(
                    "In this test we do not have async errors, this function should not be called!"
                );
                }),
            );
            assert!(mp.start().is_ok());
            socket.set_to_read(Bytes::from_static(b"Malformed data\r\n"));
            socket.call_next();
            socket.set_done();
        }

        // Fail mid-stream
        {
            let should_continue = Rc::new(Cell::new(true));
            let sc = should_continue.clone();
            let last_read = Rc::new(RefCell::new(Bytes::default()));
            let lr = last_read.clone();
            let socket = Rc::new(MockSocket::new());
            let s = socket.clone();
            let mp = MessagePump::new(
                socket.clone(),
                Box::new(move |data| {
                    *lr.borrow_mut() = data;
                    if sc.get() {
                        ReadAction::Continue
                    } else {
                        s.should_not_queue_anymore();
                        ReadAction::Stop
                    }
                }),
                Box::new(move || {
                    unreachable!(
                    "In this test we do not have async errors, this function should not be called!"
                );
                }),
            );
            assert!(mp.start().is_ok());
            let msg = Bytes::from_static(b"Successful 1\r\n");
            socket.set_to_read(msg.clone());
            socket.call_next();
            assert_eq!(*last_read.borrow(), msg);
            let msg = Bytes::from_static(b"Successful 2\r\n");
            socket.set_to_read(msg.clone());
            socket.call_next();
            assert_eq!(*last_read.borrow(), msg);
            should_continue.set(false);
            let msg = Bytes::from_static(b"Will stop\r\n");
            socket.set_to_read(msg.clone());
            socket.call_next();
            assert_eq!(*last_read.borrow(), msg);
            socket.set_done();
        }
    }

    #[test]
    fn ready_after_drop() {
        let test_data = b"TEST\r\n";
        let socket = Rc::new(MockSocket::new());
        let mp = MessagePump::new(
            socket.clone(),
            Box::new(move |_data| {
                unreachable!("We should not be called, as we were dropped!");
            }),
            Box::new(|| {
                unreachable!("We should not need the async error handler in this test.");
            }),
        );
        assert!(mp.start().is_ok());
        socket.set_to_read(Bytes::from_static(test_data));
        drop(mp);
        socket.pump_dropped();
        socket.call_next();
        socket.set_done();
    }
}
