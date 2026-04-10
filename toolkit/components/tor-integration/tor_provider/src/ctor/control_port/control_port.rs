// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::{BufMut, Bytes, BytesMut};
use std::{cell::Cell, rc::Rc};

use super::{control_socket::*, error::ControlPortError};
use crate::ctor::reply_parser::{Reply, ReplyDispatcher, ReplyError};

/// The lower-level part of the control port implementation.
/// It contains the logic for actually sending the command, and it hides its
/// reference-counted nature from actual consumers.
struct ControlPortInner {
    reply_dispatcher: ReplyDispatcher,
    socket: Rc<dyn ControlSocket>,
    closed: Cell<bool>,
}

impl ControlPortInner {
    fn new(socket: Rc<dyn ControlSocket>) -> Result<Rc<Self>, ControlSocketError> {
        Ok(Rc::new(Self {
            reply_dispatcher: ReplyDispatcher::new(),
            socket,
            closed: Cell::new(false),
        }))
        // TODO: Start the message pump.
    }

    fn close(&self) -> Result<(), ControlSocketError> {
        if self.closed.replace(true) {
            return Ok(());
        }
        self.reply_dispatcher.fail_all(ReplyError::ConnectionClosed);
        self.socket.close()
    }

    fn send_command(
        &self,
        mut command: Bytes,
        handler: Box<dyn FnOnce(Result<Reply, ControlPortError>)>,
    ) {
        if self.closed.get() {
            handler(Err(ControlPortError::ConnectionError(
                ControlSocketError::ConnectionClosed,
            )));
            return;
        }

        if !command.ends_with(b"\r\n") {
            let mut buf = BytesMut::from(command);
            buf.put(&b"\r\n"[..]);
            command = buf.freeze();
        }

        // TODO: Implement.
    }
}

impl Drop for ControlPortInner {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            log::error!(
                "Failed to close the control socket from drop: {}",
                e.to_string()
            );
        }
    }
}

pub struct ControlPort(Rc<ControlPortInner>);

impl ControlPort {
    #[inline]
    pub fn new(socket: Box<dyn ControlSocket>) -> Result<Self, ControlSocketError> {
        Ok(Self(ControlPortInner::new(Rc::from(socket))?))
    }

    #[inline]
    pub fn close(&self) -> Result<(), ControlSocketError> {
        self.0.close()
    }

    // TODO: Keep only the methods speicifc to commands and remove this one
    // (tor-browser#44930).
    #[inline]
    pub fn send_command(
        &self,
        command: Bytes,
        handler: Box<dyn FnOnce(Result<Reply, ControlPortError>)>,
    ) {
        self.0.send_command(command, handler);
    }
}
