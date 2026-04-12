// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::{BufMut, Bytes, BytesMut};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

use super::{
    command_writer::CommandWriter,
    control_socket::*,
    error::ControlPortError,
    message_pump::{MessagePump, ReadAction},
};
use crate::ctor::reply_parser::{Reply, ReplyDispatcher, ReplyError};

/// The lower-level part of the control port implementation.
/// It contains the logic for actually sending the command, and it hides its
/// reference-counted nature from actual consumers.
struct ControlPortInner {
    reply_dispatcher: ReplyDispatcher,
    socket: Rc<dyn ControlSocket>,
    writer: Rc<CommandWriter>,
    message_pump: Rc<MessagePump>,
    closed: Cell<bool>,
}

impl ControlPortInner {
    fn new(socket: Rc<dyn ControlSocket>) -> Result<Rc<Self>, ControlSocketError> {
        // We need to make sure the callbacks do not create cyclic references,
        // so use new_cyclic rather than new.
        let cp = Rc::new_cyclic(|weak_self| Self {
            reply_dispatcher: ReplyDispatcher::new(),
            socket: socket.clone(),
            writer: CommandWriter::new(socket.clone()),
            message_pump: MessagePump::new(
                socket.clone(),
                Self::make_data_cb(weak_self.clone()),
                Self::make_async_failure_cb(weak_self.clone()),
            ),
            closed: Cell::new(false),
        });
        cp.message_pump.start().inspect_err(|_| {
            let _ = cp.close();
        })?;
        Ok(cp)
    }

    fn make_data_cb(weak_self: Weak<Self>) -> Box<dyn Fn(Bytes) -> ReadAction> {
        Box::new(move |data| {
            let Some(cp) = weak_self.upgrade() else {
                return ReadAction::Stop;
            };
            if cp.reply_dispatcher.feed(&data).is_err() {
                // This will call fail_all again on the dispactcher,
                // but it is not a problem.
                cp.async_failure();
                return ReadAction::Stop;
            }
            ReadAction::Continue
        })
    }

    fn make_async_failure_cb(weak_self: Weak<Self>) -> Box<dyn Fn()> {
        Box::new(move || {
            if let Some(cp) = weak_self.upgrade() {
                cp.async_failure();
            }
        })
    }

    fn async_failure(&self) {
        self.writer.clear_queue();
        if let Err(e) = self.close() {
            log::error!("Failed to close the control port: {}.", e.to_string());
        }
    }

    fn close(&self) -> Result<(), ControlSocketError> {
        if self.closed.replace(true) {
            return Ok(());
        }
        self.reply_dispatcher.fail_all(ReplyError::ConnectionClosed);
        self.socket.close()
    }

    fn set_async_handler(&self, cb: Box<dyn Fn(Reply)>) {
        self.reply_dispatcher.set_async_handler(cb);
    }

    fn send_command(
        self: &Rc<Self>,
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

        // The callback is going to be called before other commands are sent,
        // therefore it is safe to queue the callback at this point, as next
        // command are still in the writer's queue.
        let weak_self = Rc::downgrade(self);
        self.writer.write(
            command,
            Box::new(move |res| {
                let this = match weak_self.upgrade() {
                    Some(t) => t,
                    None => {
                        return;
                    }
                };
                match res {
                    Ok(()) => {
                        this.reply_dispatcher.push_callback(Box::new(move |r| {
                            handler(r.map_err(|e| ControlPortError::ProtocolError(e)));
                        }));
                    }
                    Err(e) => {
                        handler(Err(ControlPortError::from(e)));
                        this.async_failure();
                    }
                }
            }),
        );
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

    #[inline]
    pub fn set_async_handler(&self, cb: Box<dyn Fn(Reply)>) {
        self.0.set_async_handler(cb);
    }
}
