// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::Bytes;

use super::error::ControllerError;
use crate::ctor::{
    control_port::{ControlPortInterface, ControlSocketError},
    reply_parser::Reply,
};

/// A controller for a tor daemon.
///
/// TorController wraps an implementation of a ControlPortInterface (the lower-level transport that
/// exposes a command-based interface) and exposes a set of async, callback-based methods.
/// Every method correspond to a control port command.
///
/// Callers should authenticate before issuing any other command, even when authentication is
/// disabled on the server side, and close the connection when done with it.
/// The tor daemon can be configured so that this controller is its owner, so that when it closes
/// the connection, the remote daemon shuts down.
pub struct TorController<CP: ControlPortInterface>(CP);

impl<CP: ControlPortInterface> TorController<CP> {
    #[inline]
    pub fn new(control_port: CP) -> Self {
        Self(control_port)
    }

    #[inline]
    pub fn close(&self) -> Result<(), ControllerError> {
        match self.0.close() {
            Ok(()) => Ok(()),
            Err(ControlSocketError::ConnectionClosed) => Ok(()),
            Err(ControlSocketError::ImplementationError(rv)) => {
                Err(ControllerError::ConnectionError(rv))
            }
        }
    }
}

// TODO: Remove once we merge all parts of tor-browser#44930.
impl<CP: ControlPortInterface> TorController<CP> {
    #[inline]
    pub fn send_raw_command(
        &self,
        command: Bytes,
        handler: Box<dyn FnOnce(Result<Reply, ControllerError>)>,
    ) {
        self.0.send_command(
            command,
            Box::new(|res| handler(res.map_err(|e| ControllerError::from(e)))),
        );
    }

    #[inline]
    pub fn set_async_handler(&self, cb: Option<Box<dyn Fn(Reply)>>) {
        self.0.set_async_handler(cb);
    }

    #[inline]
    pub fn set_close_handler(&self, cb: Box<dyn FnOnce()>) {
        self.0.set_close_handler(cb);
    }
}
