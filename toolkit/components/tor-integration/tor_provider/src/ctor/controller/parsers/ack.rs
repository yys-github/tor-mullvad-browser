// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::ctor::{reply_parser::Reply, ControllerError};

/// Check whether a reply contains a successful reply code and return it.
/// To be used with commands we only want to check whether they were successful.
/// It can be used also for notifications (and it will return their code), but
/// it does not make much sense to do it.
pub fn parse_ack(reply: Reply) -> Result<u16, ControllerError> {
    // Ignore any detail line on purpose to allow compatbility with future
    // protocol/command versions.
    match ControllerError::from_reply(&reply) {
        Some(e) => Err(e),
        None => Ok(reply.end_line().code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctor::reply_parser::make_reply;

    #[test]
    fn successful() {
        assert_eq!(parse_ack(make_reply(b"250 OK\r\n")).unwrap(), 250);
        assert_eq!(
            parse_ack(make_reply(b"251 Also successful\r\n")).unwrap(),
            251,
        );
        assert_eq!(
            parse_ack(make_reply(b"399 Yet another one\r\n")).unwrap(),
            399,
        );
        assert_eq!(parse_ack(make_reply(b"650 Notification\r\n")).unwrap(), 650);
    }

    #[test]
    fn error_code() {
        assert_eq!(
            parse_ack(make_reply(b"400 An error\r\n")).unwrap_err(),
            ControllerError::TorError {
                code: 400,
                message: String::from("An error"),
            },
        );
        assert_eq!(
            parse_ack(make_reply(b"599 Last error\r\n")).unwrap_err(),
            ControllerError::TorError {
                code: 599,
                message: String::from("Last error"),
            },
        );
    }

    #[test]
    fn invalid_unicode() {
        assert_eq!(parse_ack(make_reply(b"250 \xFD\r\n")).unwrap(), 250);
        assert_eq!(
            parse_ack(make_reply(b"252-Line\r\n252 \xFD\r\n")).unwrap(),
            252
        );
        assert_eq!(
            parse_ack(make_reply(b"500 Invalid \xFD codepoint\r\n")).unwrap_err(),
            ControllerError::TorError {
                code: 500,
                message: String::from("Invalid \u{FFFD} codepoint"),
            },
        );
    }

    #[test]
    fn details_ignored() {
        assert_eq!(
            parse_ack(make_reply(b"250-Details\r\n250 OK\r\n")).unwrap(),
            250
        );
        assert_eq!(
            parse_ack(make_reply(
                b"255-Changing status code\r\n260 Does not impact\r\n"
            ))
            .unwrap(),
            260
        );
        assert_eq!(
            parse_ack(make_reply(b"550-Details with error code\r\n250 OK\r\n")).unwrap(),
            250
        );
        assert_eq!(
            parse_ack(make_reply(b"250-Succesful details\r\n450 Error status\r\n")).unwrap_err(),
            ControllerError::TorError {
                code: 450,
                message: String::from("Error status"),
            },
        );
    }
}
