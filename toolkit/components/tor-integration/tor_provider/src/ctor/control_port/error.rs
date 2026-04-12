// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use thiserror::Error;

use super::super::{ControlSocketError, ReplyError};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ControlPortError {
    #[error("connection error")]
    ConnectionError(#[from] ControlSocketError),
    #[error("protocol violation")]
    ProtocolError(#[from] ReplyError),
    #[error("unsuccessful command ({code}): {message}")]
    TorError { code: u16, message: String },
}
