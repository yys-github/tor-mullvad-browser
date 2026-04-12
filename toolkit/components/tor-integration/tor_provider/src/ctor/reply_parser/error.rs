// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use thiserror::Error;

/// Errors linked to replies.
#[derive(Error, Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReplyError {
    /// A line is less than 4 characters
    #[error("The line is too short, therefore it is malformed")]
    LineTooShort,
    /// The beginning of a line cannot be parsed as a status code
    #[error("Invalid status code")]
    BadStatusCode,
    /// Seen the start of a sync/async reply while handling the other type
    #[error("The server sent a sync or async command before the end of the previous reply")]
    SyncAsyncMixed,
    /// The first byte after the status code is not +, - or space
    #[error("{0} is not a valid separator after the status code")]
    BadSeparator(u8),
    /// The I/O user of the dispatcher closed the connection.
    #[error("The connection was closed")]
    ConnectionClosed,
}
