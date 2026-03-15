// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::net::IpAddr;
use tinystr::TinyAsciiStr;

use super::Command;
use crate::ctor::{
    controller::{parsers::*, types::*, ControllerError},
    reply_parser::{DetailReplyLine, Reply},
};

pub fn bootstrap_phase() -> Result<Command<BootstrapStatus>, ControllerError> {
    get_info_single(
        String::from("status/bootstrap-phase"),
        parse_bootstrap_status,
    )
}

pub fn circuits() -> Result<Command<Vec<Circuit>>, ControllerError> {
    get_info(String::from("circuit-status"), |lines| {
        lines
            .iter()
            .filter_map(|line| parse_circuit_built(*line).transpose())
            .collect()
    })
}

pub fn net_listeners_socks() -> Result<Command<Vec<SocksListener>>, ControllerError> {
    get_info_single(String::from("net/listeners/socks"), parse_socks_listeners)
}

pub fn node_addresses(node: NodeFingerprint) -> Result<Command<NodeAddresses>, ControllerError> {
    get_info(
        format!("ns/id/{}", hex::encode(node)),
        addresses_from_status_info,
    )
}

pub fn ip_to_country(ip: IpAddr) -> Result<Command<TinyAsciiStr<2>>, ControllerError> {
    get_info_single(format!("ip-to-country/{}", ip), parse_country_code)
}

fn get_info<F, T>(key: String, converter: F) -> Result<Command<T>, ControllerError>
where
    F: Fn(&[&[u8]]) -> Result<T, ControllerError> + 'static,
{
    Ok(Command {
        command: format!("GETINFO {}\r\n", key),
        handler: Box::new(move |reply| parse_get_info(&key, &converter, reply)),
    })
}

fn get_info_single<F, T>(key: String, converter: F) -> Result<Command<T>, ControllerError>
where
    F: Fn(&[u8]) -> Result<T, ControllerError> + 'static,
{
    get_info(key, move |lines| {
        if lines.len() != 1 {
            Err(ControllerError::MalformedReply(String::from(
                "Expected a single line on the reply.",
            )))
        } else {
            converter(lines[0])
        }
    })
}

fn parse_get_info<S, F, T>(key: S, converter: &F, reply: Reply) -> Result<T, ControllerError>
where
    S: AsRef<str>,
    F: Fn(&[&[u8]]) -> Result<T, ControllerError> + 'static,
{
    if let Some(e) = ControllerError::from_reply(&reply) {
        return Err(e);
    }

    let k = key.as_ref().as_bytes();

    for line in reply.details().iter() {
        match line {
            DetailReplyLine::MidReplyLine { code, line } => {
                if *code != 250 {
                    continue;
                }
                if line.len() < k.len() + 1 {
                    continue;
                }
                if line[0..k.len()].eq_ignore_ascii_case(k) && line[k.len()] == b'=' {
                    return converter(&[&line[(k.len() + 1)..]]);
                }
            }
            DetailReplyLine::DataReplyLine { code, lines } => {
                if *code != 250 {
                    continue;
                }
                let first = &lines[0];
                if first.len() == k.len() + 1
                    && first[0..k.len()].eq_ignore_ascii_case(k)
                    && first[k.len()] == b'='
                {
                    let lines: Vec<&[u8]> = lines[1..].iter().map(|b| &b[..]).collect();
                    return converter(&lines);
                }
            }
        }
    }

    // We should never arrive here. Instead, tor should send 552.
    Err(ControllerError::KeyNotFound(key.as_ref().into()))
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;
    use crate::ctor::reply_parser::make_reply;

    fn identity_converter(v: &[&[u8]]) -> Result<Vec<Vec<u8>>, ControllerError> {
        Ok(v.iter().map(|s| Vec::from(*s)).collect())
    }

    #[test]
    fn mid_line() {
        assert_eq!(
            parse_get_info(
                "version",
                &identity_converter,
                make_reply(b"250-version=0.5.0.0-alpha-dev\r\n250 OK\r\n")
            )
            .unwrap(),
            &[b"0.5.0.0-alpha-dev"]
        );
    }

    #[test]
    fn data_line() {
        assert_eq!(
            parse_get_info(
                "test",
                &identity_converter,
                make_reply(b"250+test=\r\n123\r\n456\r\n.\r\n250 OK\r\n")
            )
            .unwrap(),
            vec![b"123", b"456"]
        );
    }

    #[test]
    fn data_line_empty() {
        assert_eq!(
            parse_get_info(
                "test",
                &identity_converter,
                make_reply(b"250+test=\r\n.\r\n250 OK\r\n")
            )
            .unwrap(),
            Vec::<&[u8]>::new()
        );
    }

    #[test]
    fn mid_line_prefix() {
        assert_matches!(
            parse_get_info(
                "version",
                &identity_converter,
                make_reply(b"250-version__=0.5.0.0-alpha-dev\r\n250 OK\r\n")
            )
            .unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
    }

    #[test]
    fn data_line_prefix() {
        assert_matches!(
            parse_get_info(
                "test",
                &identity_converter,
                make_reply(b"250+test__=\r\n123\r\n456\r\n.\r\n250 OK\r\n")
            )
            .unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
    }

    #[test]
    fn short_line_long_key() {
        assert_matches!(
            parse_get_info(
                "averyverylongkey",
                &identity_converter,
                make_reply(b"250+short=\r\n123\r\n456\r\n.\r\n250 OK\r\n")
            )
            .unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
        assert_matches!(
            parse_get_info(
                "averyverylooooooooooongkey",
                &identity_converter,
                make_reply(b"250-short=\r\n250 OK\r\n")
            )
            .unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
    }

    #[test]
    fn tor_error() {
        assert_eq!(
            parse_get_info(
                "test",
                &identity_converter,
                make_reply(b"552 Unrecognized key \"test\"\r\n")
            )
            .unwrap_err(),
            ControllerError::TorError {
                code: 552,
                message: String::from("Unrecognized key \"test\"")
            }
        );
    }

    #[test]
    fn error_in_success() {
        // This case should not be possible in GETINFO.
        // But we want to be sure we can handle it (as a key not found), just in
        // case.
        assert_matches!(
            parse_get_info(
                "test",
                &identity_converter,
                make_reply(b"552-test=0.5.0.0-alpha-dev\r\n250 OK\r\n")
            )
            .unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
        assert_matches!(
            parse_get_info(
                "test",
                &identity_converter,
                make_reply(b"552-test=\r\n250 OK\r\n")
            )
            .unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
    }

    #[test]
    fn only_end_line() {
        assert_matches!(
            parse_get_info(
                "test",
                &identity_converter,
                make_reply(b"250 OK\r\n")
            )
            .unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
    }
}
