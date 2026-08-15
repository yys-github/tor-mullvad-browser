// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use thiserror::Error;

use crate::ctor::ReplyError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ControlPortError {
    #[error("connection error: {0:#x}")]
    ConnectionError(u32),
    #[error("protocol violation: {0}")]
    ProtocolError(#[from] ReplyError),
}
