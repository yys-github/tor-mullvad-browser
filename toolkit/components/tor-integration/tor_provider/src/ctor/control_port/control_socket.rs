// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::Bytes;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ControlSocketError {
    /// A wrapper for XPCOM error codes (nsresult), that are 32-bit unsigned
    /// integers. We cannot use nsresult directly as this crate does not link to
    /// XPCOM.
    #[error("implementation error: {0:08X}")]
    ImplementationError(u32),
    #[error("the connection has been closed")]
    ConnectionClosed,
}

/// The I/O layer for the control port.
/// The implementation should manage thread safety, to make sure everything
/// happens in the thread of the caller.
pub trait ControlSocket {
    /// Queue a read.
    fn queue_read(&self, f: Box<dyn FnOnce()>) -> Result<(), ControlSocketError>;

    /// Tell how many bytes can be read from the socket.
    /// Returns 0 if the peer closed the connection.
    fn available(&self) -> Result<u32, ControlSocketError>;

    /// Read at most max_len from the socket.
    /// The caller is expected to support partial reads.
    /// However, an empty result will be treated as an EOF/connection closed.
    /// Also, the caller is expected to call read only after a calback
    /// registered with queue_read was called.
    fn read(&self, max_len: u32) -> Result<Bytes, ControlSocketError>;

    /// Queue a write.
    /// For better ergonomics, implementations should report also synchronous
    /// errors through the callback, as callers will have to implement an async
    /// error strategy anyway.
    fn queue_write(&self, f: Box<dyn FnOnce(Result<(), ControlSocketError>)>, len: u32);

    /// Write to the socket.
    /// The caller is expected to handle partial writes (after calling
    /// queue_write again).
    /// Also, the caller is expected to call write only after a callback
    /// registered with queue_write was called.
    /// The caller is expected not to pass an empty buffer. Implementations are
    /// allowed to return errors in that case.
    /// Notice that 0 bytes written is an allowed value, but the caller might
    /// queue another write immediately. The socket implementation should call
    /// the new callback only when there is actually some room for writing, to
    /// avoid potential infinite loops.
    fn write(&self, buffer: &Bytes) -> Result<u32, ControlSocketError>;

    /// Close the socket.
    fn close(&self) -> Result<(), ControlSocketError>;
}
