// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use lazy_static::lazy_static;
use regex::Regex;

use super::parse_key_value_pairs;
use crate::ctor::{
    controller::{types::*, ControllerError},
    reply_parser::{DetailReplyLine, Reply},
};

lazy_static! {
    static ref CLIENT_REGEX: Regex = Regex::new(
        r"^(?i)CLIENT (?<HSAddress>[a-zA-Z2-7]{56}) (?<KeyType>[A-Za-z0-9]+):(?<PrivateKeyBlob>[A-Za-z0-9+/]+={0,3})(?: (?<Args>.+))?$"
    )
    .expect("The regex is hardcoded, it should be good.");
}

const AUTH_VIEW_HEADER: &[u8] = b"ONION_CLIENT_AUTH_VIEW";

/// Check the first line of the reply to ONION_CLIENT_AUTH_VIEW.
fn check_auth_view(line: Option<&DetailReplyLine>) -> Result<(), ControllerError> {
    if let Some(DetailReplyLine::MidReplyLine { code, line }) = line {
        if *code != 250 {
            // The first thing we check is that we end with "250 OK", so this
            // error code is okay.
            return Err(ControllerError::MixedCodes);
        }
        if line.eq_ignore_ascii_case(AUTH_VIEW_HEADER) {
            return Ok(());
        }
    }
    Err(ControllerError::MalformedReply(String::from(
        "the first line must be 250-ONION_CLIENT_AUTH_VIEW.",
    )))
}

/// Parse an individual 250-CLIENT line, which follows this format:
/// `250-CLIENT <hs-address> <key-type>:<key-blob> <kw-args>`.
///
/// Note:
/// - hs-address is expected not to have the .onion suffix
/// - we will accept any key type, even though tor only supports x25519
/// - key-blob is expected to be base64 (but we perform a light validation and
///   return the key as we received it)
/// - we only parse the Flags keyword argument, and only the permanent flag
///   (in a case-insensitive manner)
fn parse_client_line(data: &[u8]) -> Result<OnionClientAuth, ControllerError> {
    // The reply for this command should not contain escaped strings, but only
    // ASCII data. So, it must also be valid UTF-8.
    let data = str::from_utf8(data).map_err(|e| ControllerError::MalformedReply(e.to_string()))?;
    let groups = CLIENT_REGEX
        .captures(data)
        .ok_or_else(|| ControllerError::WrongFormat(data.to_string()))?;

    let address =
        HSAddress::from_str(&groups["HSAddress"]).map_err(|e| ControllerError::from(e))?;

    let key_type = groups["KeyType"].into();
    let key_blob = groups["PrivateKeyBlob"].into();

    let is_permanent = groups
        .name("Args")
        .map(|a| parse_key_value_pairs(a.as_str().as_bytes(), &["Flags"]))
        .transpose()?
        .and_then(|args| {
            args.get("Flags").map(|flags| {
                flags
                    .split(|b| *b == b',')
                    .any(|flag| flag.eq_ignore_ascii_case(b"Permanent"))
            })
        })
        .unwrap_or(false);

    Ok(OnionClientAuth {
        address,
        key_type,
        key_blob,
        is_permanent,
    })
}

/// Parse the full reply to a ONION_CLIENT_AUTH_VIEW command.
pub fn parse_onion_client_auth_view(reply: Reply) -> Result<Vec<OnionClientAuth>, ControllerError> {
    if let Some(e) = ControllerError::from_reply(&reply) {
        return Err(e);
    }
    if reply.end_line().code != 250 {
        return Err(ControllerError::MalformedReply(format!(
            "expected the end line to be a '250 OK', found '{}'",
            String::from_utf8_lossy(reply.end_line().line.as_ref())
        )));
    }

    check_auth_view(reply.details().first())?;

    reply.details()[1..]
        .iter()
        .map(|l| match l {
            DetailReplyLine::MidReplyLine { code, line } => {
                if *code == 250 {
                    parse_client_line(line.as_ref())
                } else {
                    Err(ControllerError::MixedCodes)
                }
            }
            DetailReplyLine::DataReplyLine { .. } => Err(ControllerError::MalformedReply(
                String::from("Datalines are not allowed in ONION_CLIENT_AUTH_VIEW."),
            )),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;
    use crate::ctor::reply_parser::make_reply;

    #[test]
    fn single_no_permanent() {
        let raw = concat!(
            "250-ONION_CLIENT_AUTH_VIEW\r\n",
            "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/==\r\n",
            "250 OK\r\n"
        );
        let reply = make_reply(raw.as_bytes());
        let result = parse_onion_client_auth_view(reply).unwrap();
        assert_eq!(
            result,
            vec![OnionClientAuth {
                address: HSAddress::new(
                    b"pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd"
                )
                .unwrap(),
                key_type: String::from("x25519"),
                key_blob: String::from("abcdef1234567890+/=="),
                is_permanent: false,
            }]
        )
    }

    #[test]
    fn single_permanent() {
        let raw = concat!(
            "250-ONION_CLIENT_AUTH_VIEW\r\n",
            "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/== Flags=Permanent\r\n",
            "250 OK\r\n"
        );
        let reply = make_reply(raw.as_bytes());
        let result = parse_onion_client_auth_view(reply).unwrap();
        assert_eq!(
            result,
            vec![OnionClientAuth {
                address: HSAddress::new(
                    b"pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd"
                )
                .unwrap(),
                key_type: String::from("x25519"),
                key_blob: String::from("abcdef1234567890+/=="),
                is_permanent: true,
            }]
        )
    }

    #[test]
    fn empty() {
        let reply = make_reply(b"250-ONION_CLIENT_AUTH_VIEW\r\n250 OK\r\n");
        let result = parse_onion_client_auth_view(reply).unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn multiple() {
        let raw = concat!(
            "250-ONION_CLIENT_AUTH_VIEW\r\n",
            "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/== Flags=Permanent\r\n",
            "250-CLIENT sp3k262uwy4r2k3ycr5awluarykdpag6a7y33jxop4cs2lu5uz5sseqd x25519:thekeyhere\r\n",
            "250 OK\r\n"
        );
        let reply = make_reply(raw.as_bytes());
        let result = parse_onion_client_auth_view(reply).unwrap();
        assert_eq!(
            result,
            vec![
                OnionClientAuth {
                    address: HSAddress::new(
                        b"pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd"
                    )
                    .unwrap(),
                    key_type: String::from("x25519"),
                    key_blob: String::from("abcdef1234567890+/=="),
                    is_permanent: true,
                },
                OnionClientAuth {
                    address: HSAddress::new(
                        b"sp3k262uwy4r2k3ycr5awluarykdpag6a7y33jxop4cs2lu5uz5sseqd"
                    )
                    .unwrap(),
                    key_type: String::from("x25519"),
                    key_blob: String::from("thekeyhere"),
                    is_permanent: false,
                },
            ]
        )
    }

    #[test]
    fn case_insensitive() {
        let raw = concat!(
            "250-onioN_CLIENt_autH_VIEW\r\n",
            "250-CLieNt Pg6MmjiYjMcRssLvyKFWNntLARu7P5SVn6y2YMMJu6NUbxNdf4PscRYD x25519:abcdef1234567890+/== fLaGs=permAnEnt\r\n",
            "250 OK\r\n"
        );
        let reply = make_reply(raw.as_bytes());
        let result = parse_onion_client_auth_view(reply).unwrap();
        assert_eq!(
            result,
            vec![OnionClientAuth {
                address: HSAddress::new(
                    b"pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd"
                )
                .unwrap(),
                key_type: String::from("x25519"),
                key_blob: String::from("abcdef1234567890+/=="),
                is_permanent: true,
            }]
        )
    }

    #[test]
    fn unknown_flags() {
        let raw = concat!(
            "250-ONION_CLIENT_AUTH_VIEW\r\n",
            "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/== Flags=SomeFlag,AnotherFlag\r\n",
            "250 OK\r\n"
        );
        let reply = make_reply(raw.as_bytes());
        let result = parse_onion_client_auth_view(reply).unwrap();
        assert_eq!(
            result,
            vec![OnionClientAuth {
                address: HSAddress::new(
                    b"pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd"
                )
                .unwrap(),
                key_type: String::from("x25519"),
                key_blob: String::from("abcdef1234567890+/=="),
                is_permanent: false,
            }]
        )
    }

    #[test]
    fn multiple_flags() {
        let raw = concat!(
            "250-ONION_CLIENT_AUTH_VIEW\r\n",
            "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/== Flags=SomeFlag,Permanent,AnotherFlag\r\n",
            "250 OK\r\n"
        );
        let reply = make_reply(raw.as_bytes());
        let result = parse_onion_client_auth_view(reply).unwrap();
        assert_eq!(
            result,
            vec![OnionClientAuth {
                address: HSAddress::new(
                    b"pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd"
                )
                .unwrap(),
                key_type: String::from("x25519"),
                key_blob: String::from("abcdef1234567890+/=="),
                is_permanent: true,
            }]
        )
    }

    #[test]
    fn client_name_ignored() {
        {
            let raw = concat!(
                "250-ONION_CLIENT_AUTH_VIEW\r\n",
                "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/== ClientName=test\r\n",
                "250 OK\r\n"
            );
            let reply = make_reply(raw.as_bytes());
            assert!(parse_onion_client_auth_view(reply).is_ok());
        }

        {
            let raw = concat!(
                "250-ONION_CLIENT_AUTH_VIEW\r\n",
                "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/== ClientName=test Flags=Permanent\r\n",
                "250 OK\r\n"
            );
            let reply = make_reply(raw.as_bytes());
            let result = parse_onion_client_auth_view(reply).unwrap();
            assert!(result[0].is_permanent);
        }
        {
            let raw = concat!(
                "250-ONION_CLIENT_AUTH_VIEW\r\n",
                "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/== ClientName=test Flags=\r\n",
                "250 OK\r\n"
            );
            let reply = make_reply(raw.as_bytes());
            let result = parse_onion_client_auth_view(reply).unwrap();
            assert!(!result[0].is_permanent);
        }

        // In the specs, Flags is listed after ClientName, but we accept any
        // order.
        {
            let raw = concat!(
                "250-ONION_CLIENT_AUTH_VIEW\r\n",
                "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/== Flags=Permanent ClientName=test\r\n",
                "250 OK\r\n"
            );
            let reply = make_reply(raw.as_bytes());
            let result = parse_onion_client_auth_view(reply).unwrap();
            assert!(result[0].is_permanent);
        }
        {
            let raw = concat!(
                "250-ONION_CLIENT_AUTH_VIEW\r\n",
                "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/== Flags= ClientName=test\r\n",
                "250 OK\r\n"
            );
            let reply = make_reply(raw.as_bytes());
            let result = parse_onion_client_auth_view(reply).unwrap();
            assert!(!result[0].is_permanent);
        }
    }

    #[test]
    fn unsuccessful_end_code() {
        {
            let reply = make_reply(b"512 Syntax error\r\n");
            let result = parse_onion_client_auth_view(reply);
            assert_matches!(
                result.unwrap_err(),
                ControllerError::TorError { code: 512, .. }
            );
        }
        {
            let reply = make_reply(b"250-ONION_CLIENT_AUTH_VIEW\r\n512 Syntax error\r\n");
            let result = parse_onion_client_auth_view(reply);
            assert_matches!(
                result.unwrap_err(),
                ControllerError::TorError { code: 512, .. }
            );
        }
    }

    #[test]
    fn bad_header() {
        {
            let reply = make_reply(b"250 OK\r\n");
            let result = parse_onion_client_auth_view(reply);
            assert_matches!(result.unwrap_err(), ControllerError::MalformedReply(_));
        }
        {
            let reply = make_reply(b"254-BAD HEADER\r\n250 OK\r\n");
            let result = parse_onion_client_auth_view(reply);
            assert_eq!(result.unwrap_err(), ControllerError::MixedCodes);
        }
        {
            let reply = make_reply(b"250-BAD HEADER\r\n250 OK\r\n");
            let result = parse_onion_client_auth_view(reply);
            assert_matches!(result.unwrap_err(), ControllerError::MalformedReply(_));
        }
        {
            let reply = make_reply(b"250+DataReply Header\r\n.\r\n250 OK\r\n");
            let result = parse_onion_client_auth_view(reply);
            assert_matches!(result.unwrap_err(), ControllerError::MalformedReply(_));
        }
    }

    #[test]
    fn datalines() {
        {
            let raw = concat!(
                "250-ONION_CLIENT_AUTH_VIEW\r\n",
                "250+CLIENT\r\n.\r\n",
                "250 OK\r\n"
            );
            let reply = make_reply(raw.as_bytes());
            let result = parse_onion_client_auth_view(reply);
            assert_matches!(result.unwrap_err(), ControllerError::MalformedReply(_));
        }
        {
            let raw = concat!(
                "250-ONION_CLIENT_AUTH_VIEW\r\n",
                "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/==\r\n",
                "250+CLIENT\r\n.\r\n",
                "250 OK\r\n"
            );
            let reply = make_reply(raw.as_bytes());
            let result = parse_onion_client_auth_view(reply);
            assert_matches!(result.unwrap_err(), ControllerError::MalformedReply(_));
        }
        {
            let raw = concat!(
                "250-ONION_CLIENT_AUTH_VIEW\r\n",
                "250+CLIENT\r\n.\r\n",
                "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/==\r\n",
                "250 OK\r\n"
            );
            let reply = make_reply(raw.as_bytes());
            let result = parse_onion_client_auth_view(reply);
            assert_matches!(result.unwrap_err(), ControllerError::MalformedReply(_));
        }
    }

    #[test]
    fn no_250_codes() {
        {
            let raw = concat!(
                "250-ONION_CLIENT_AUTH_VIEW\r\n",
                "251-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/==\r\n",
                "250 OK\r\n"
            );
            let reply = make_reply(raw.as_bytes());
            let result = parse_onion_client_auth_view(reply);
            assert_eq!(result.unwrap_err(), ControllerError::MixedCodes);
        }
        {
            let raw = concat!(
                "250-ONION_CLIENT_AUTH_VIEW\r\n",
                "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/==\r\n",
                "251-CLIENT sp3k262uwy4r2k3ycr5awluarykdpag6a7y33jxop4cs2lu5uz5sseqd x25519:abcdef1234567890+/==\r\n",
                "250 OK\r\n"
            );
            let reply = make_reply(raw.as_bytes());
            let result = parse_onion_client_auth_view(reply);
            assert_eq!(result.unwrap_err(), ControllerError::MixedCodes);
        }
        {
            let raw = concat!(
                "250-ONION_CLIENT_AUTH_VIEW\r\n",
                "251-CLIENT sp3k262uwy4r2k3ycr5awluarykdpag6a7y33jxop4cs2lu5uz5sseqd x25519:abcdef1234567890+/==\r\n",
                "250-CLIENT pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd x25519:abcdef1234567890+/==\r\n",
                "250 OK\r\n"
            );
            let reply = make_reply(raw.as_bytes());
            let result = parse_onion_client_auth_view(reply);
            assert_eq!(result.unwrap_err(), ControllerError::MixedCodes);
        }
        {
            let raw = concat!("251-ONION_CLIENT_AUTH_VIEW\r\n", "250 OK\r\n");
            let reply = make_reply(raw.as_bytes());
            let result = parse_onion_client_auth_view(reply);
            assert_eq!(result.unwrap_err(), ControllerError::MixedCodes);
        }
        {
            let raw = concat!("251-ONION_CLIENT_AUTH_VIEW\r\n", "251 OK\r\n");
            let reply = make_reply(raw.as_bytes());
            assert!(parse_onion_client_auth_view(reply).is_err());
        }
        {
            let raw = concat!("250-ONION_CLIENT_AUTH_VIEW\r\n", "251 OK\r\n");
            let reply = make_reply(raw.as_bytes());
            assert!(parse_onion_client_auth_view(reply).is_err());
        }
    }

    #[test]
    fn malformed_client_line() {
        let make_error =
            |r: &'static str| parse_onion_client_auth_view(make_reply(r.as_bytes())).unwrap_err();

        assert_matches!(
            make_error("250-ONION_CLIENT_AUTH_VIEW\r\n250-aaaaaaaaa\r\n250 OK\r\n"),
            ControllerError::WrongFormat(_)
        );
        assert_matches!(
            make_error("250-ONION_CLIENT_AUTH_VIEW\r\n250-client aaaaaaaaa\r\n250 OK\r\n"),
            ControllerError::WrongFormat(_)
        );
        assert_matches!(
            make_error(
                "250-ONION_CLIENT_AUTH_VIEW\r\n250-client aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa x25519:aaaa\r\n250 OK\r\n"
            ),
            ControllerError::InvalidOnionAddress(_)
        );
        assert_matches!(
            make_error(
                "250-ONION_CLIENT_AUTH_VIEW\r\n250-CLIENT sp3k262uwy4r2k3ycr5awluarykdpag6a7y33jxop4cs2lu5uz5sseqd x25519:not_base_64!!\r\n250 OK\r\n"
            ),
            ControllerError::WrongFormat(_)
        );
        assert_matches!(
            make_error(
                "250-ONION_CLIENT_AUTH_VIEW\r\n250-CLIENT sp3k262uwy4r2k3ycr5awluarykdpag6a7y33jxop4cs2lu5uz5sseqd x25519:aaaa KeyWithoutEqual\r\n250 OK\r\n"
            ),
            ControllerError::MalformedReply(_)
        );
    }
}
