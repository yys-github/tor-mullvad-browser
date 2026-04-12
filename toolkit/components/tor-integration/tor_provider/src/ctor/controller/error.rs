// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use thiserror::Error;

use crate::ctor::{
    control_port::ControlPortError,
    reply_parser::{Reply, ReplyError},
};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ControllerError {
    #[error("connection error: {0:#x}")]
    ConnectionError(u32),
    #[error("protocol violation: {0}")]
    ProtocolError(#[from] ReplyError),
    #[error("unsuccessful command ({code}): {message}")]
    TorError {
        code: u16,
        // Notice: tor does not dive any guarantee about charsets.
        // However, we expect errors to be ASCII strings, and they are used only
        // for logs, they will never be directly user-facing.
        message: String,
    },
    #[error("the reply contains lines with mixed codes")]
    MixedCodes,
    #[error("the reply does not match the expected format: {0}")]
    WrongFormat(String),
    #[error("the requested key {0} was not found")]
    KeyNotFound(String),
    #[error("malformed reply: {0}")]
    MalformedReply(String),
}

impl ControllerError {
    pub(super) fn from_reply(reply: &Reply) -> Option<Self> {
        if reply.end_line().is_error() {
            Some(Self::TorError {
                code: reply.end_line().code,
                message: String::from_utf8_lossy(&reply.end_line().line).into_owned(),
            })
        } else {
            None
        }
    }
}

impl From<ControlPortError> for ControllerError {
    fn from(value: ControlPortError) -> Self {
        match value {
            ControlPortError::ConnectionError(rv) => ControllerError::ConnectionError(rv),
            ControlPortError::ProtocolError(err) => ControllerError::ProtocolError(err),
        }
    }
}
