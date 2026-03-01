// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::iter::once;

use super::Command;
use crate::ctor::{
    controller::{parsers::*, types::*, ControllerError},
    reply_parser::{DetailReplyLine, Reply},
};

pub fn bridges() -> Result<Command<Vec<Bridge>>, ControllerError> {
    get_conf("Bridge", |line| {
        parse_bridge_line(line).map_err(|e| ControllerError::MalformedReply(e.to_string()))
    })
}

pub fn client_transport_plugins() -> Result<Command<Vec<PluggableTransport>>, ControllerError> {
    get_conf("ClientTransportPlugin", parse_client_transport_plugin)
}

fn get_conf<F, T>(key: &'static str, converter: F) -> Result<Command<Vec<T>>, ControllerError>
where
    F: Fn(&[u8]) -> Result<T, ControllerError> + 'static,
{
    Ok(Command {
        command: format!("GETCONF {}\r\n", key),
        handler: Box::new(move |reply| parse_get_conf(key.as_bytes(), &converter, reply)),
    })
}

/// Parse and convert the output of the GETCONF command for a single keyword.
///
/// We currently query and parse only one key at the time, but this parser
/// tolerates keywords we did not ask (it silently ignores them).
fn parse_get_conf<F, T>(key: &[u8], converter: &F, reply: Reply) -> Result<Vec<T>, ControllerError>
where
    F: Fn(&[u8]) -> Result<T, ControllerError> + 'static,
{
    if let Some(e) = ControllerError::from_reply(&reply) {
        return Err(e);
    }

    let check_code = |code| {
        if code != 250 {
            Err(ControllerError::MalformedReply(format!(
                "Expected the 250 status code, got '{}'",
                code
            )))
        } else {
            Ok(())
        }
    };

    check_code(reply.end_line().code)?;

    // Special case: "default value semantically different from an empty string"
    // is just "250 KEYWORD CRLF" without '=' (see the specs).
    if reply.details().is_empty() && reply.end_line().line.eq_ignore_ascii_case(key) {
        return Ok(Vec::new());
    }

    // Key followed by '='.
    let prefix_len = key.len() + 1;

    let details = reply.details().iter().map(|l| match l {
        DetailReplyLine::MidReplyLine { code, line } => {
            check_code(*code)?;
            Ok(line.as_ref())
        }
        DetailReplyLine::DataReplyLine { .. } => Err(ControllerError::MalformedReply(
            String::from("GETCONF does not allow data lines"),
        )),
    });

    let all_lines = details.chain(once(Ok(reply.end_line().line.as_ref())));

    all_lines
        .filter_map(|line| match line {
            Ok(l) => {
                if l.len() >= prefix_len
                    && l[0..key.len()].eq_ignore_ascii_case(key)
                    && l[key.len()] == b'='
                {
                    Some(converter(&l[prefix_len..]))
                } else {
                    None
                }
            }
            Err(e) => Some(Err(e)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;
    use crate::ctor::reply_parser::make_reply;

    fn converter(v: &[u8]) -> Result<String, ControllerError> {
        Ok(String::from_utf8_lossy(v).into_owned())
    }

    #[test]
    fn simple() {
        assert_eq!(
            parse_get_conf(b"test", &converter, make_reply(b"250 test=value\r\n")).unwrap(),
            vec!["value"]
        );
    }

    #[test]
    fn case_insensitivity() {
        assert_eq!(
            parse_get_conf(b"test", &converter, make_reply(b"250 TEST=value\r\n")).unwrap(),
            vec!["value"]
        );
        assert_eq!(
            parse_get_conf(b"Test", &converter, make_reply(b"250 TEST=value\r\n")).unwrap(),
            vec!["value"]
        );
        assert_eq!(
            parse_get_conf(b"TeSt", &converter, make_reply(b"250 tESt=value\r\n")).unwrap(),
            vec!["value"]
        );
    }

    #[test]
    fn array() {
        assert_eq!(
            parse_get_conf(
                b"test",
                &converter,
                make_reply(b"250-test=value 1\r\n250 test=value 2\r\n")
            )
            .unwrap(),
            vec!["value 1", "value 2"]
        );

        assert_eq!(
            parse_get_conf(
                b"Test",
                &converter,
                make_reply(b"250-test=VALUE 1\r\n250 TEST=value 2\r\n")
            )
            .unwrap(),
            vec!["VALUE 1", "value 2"]
        );
    }

    #[test]
    fn empty() {
        assert!(
            parse_get_conf(b"test", &converter, make_reply(b"250 test\r\n"))
                .unwrap()
                .is_empty(),
        );
        assert!(
            parse_get_conf(b"Test", &converter, make_reply(b"250 TEST\r\n"))
                .unwrap()
                .is_empty(),
        );
    }

    #[test]
    fn other_keys() {
        assert_eq!(
            parse_get_conf(
                b"test",
                &converter,
                make_reply(b"250-test 1=a\r\n250-test=b\r\n250-test=c\r\n250 test 2=d\r\n")
            )
            .unwrap(),
            vec!["b", "c"]
        );

        assert!(parse_get_conf(
            b"C",
            &converter,
            make_reply(b"250-A=Apple\r\n250 B=Banana\r\n"),
        )
        .unwrap()
        .is_empty(),);
    }

    #[test]
    fn tor_error() {
        assert_eq!(
            parse_get_conf(
                b"test",
                &converter,
                make_reply(b"552 Unrecognized configuration key \"test\"\r\n")
            )
            .unwrap_err(),
            ControllerError::TorError {
                code: 552,
                message: String::from("Unrecognized configuration key \"test\"")
            }
        );
    }

    #[test]
    fn no_250() {
        assert_matches!(
            parse_get_conf(
                b"test",
                &converter,
                make_reply(b"251 test=value\r\n")
            )
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );
        assert_matches!(
            parse_get_conf(
                b"test",
                &converter,
                make_reply(b"251-test1=value\r\n250 test2=value2\r\n")
            )
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );
        assert_matches!(
            parse_get_conf(
                b"test",
                &converter,
                make_reply(b"250-test1=value\r\n251 test1=value2\r\n")
            )
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );
    }

    #[test]
    fn no_multiline_getconf() {
        assert_matches!(
            parse_get_conf(
                b"test",
                &converter,
                make_reply(b"250+test=\r\nsome value\r\n.\r\n250 OK\r\n")
            )
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );
    }

    #[test]
    fn converter_error() {
        assert_eq!(
            parse_get_conf::<_, String>(
                b"test",
                &(|_| Err(ControllerError::WrongFormat(String::from(
                    "Something is fishy"
                )))),
                make_reply(b"250 test=value\r\n")
            )
            .unwrap_err(),
            ControllerError::WrongFormat(String::from("Something is fishy"))
        );
    }
}
