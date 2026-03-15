// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use lazy_static::lazy_static;
use regex::bytes::Regex;

use super::{parse_key_value_pairs, tor_unescape};
use crate::ctor::controller::{CircuitID, ControllerError, Stream, StreamID};

const STREAM_EVENT: &[u8] = b"SENTCONNECT";

lazy_static! {
    // https://spec.torproject.org/control-spec/replies.html#STREAM
    static ref STATUS_REGEX: Regex = Regex::new(
        "^(?<StreamID>[a-zA-Z0-9]{1,16}) (?<StreamStatus>[A-Za-z_]+) (?<CircuitID>[a-zA-Z0-9]{1,16}) (?:[^ ]+)(?: (?<Args>.*))?$"
    )
    .expect("The regex is hardcoded, it should be good.");
}

const USERNAME_KEYWORD: &'static str = "SOCKS_USERNAME";
const PASSWORD_KEYWORD: &'static str = "SOCKS_PASSWORD";
const STREAM_KEYWORDS: &[&'static str] = &[USERNAME_KEYWORD, PASSWORD_KEYWORD];

/// Parse a stream update event.
/// Only the SENTCONNECT events will be parsed, the other ones will be ignored.
pub fn parse_stream_sentconnect(line: &[u8]) -> Result<Option<Stream>, ControllerError> {
    let stream = STATUS_REGEX
        .captures(line)
        .ok_or_else(|| ControllerError::WrongFormat(String::from_utf8_lossy(line).into_owned()))?;

    if !stream["StreamStatus"].eq_ignore_ascii_case(STREAM_EVENT) {
        return Ok(None);
    }

    let stream_id = StreamID::new(&stream["StreamID"]).ok_or_else(|| {
        ControllerError::MalformedReply(format!(
            "invalid stream ID '{}'",
            String::from_utf8_lossy(&stream["StreamID"])
        ))
    })?;
    let circuit_id = CircuitID::new(&stream["CircuitID"]).ok_or_else(|| {
        ControllerError::MalformedReply(format!(
            "invalid circuit ID '{}'",
            String::from_utf8_lossy(&stream["CircuitID"])
        ))
    })?;

    let args = stream
        .name("Args")
        .map(|a| parse_key_value_pairs(a.as_bytes(), STREAM_KEYWORDS))
        .transpose()?
        .unwrap_or_default();
    let get_credential = |name| {
        args.get(name)
            .map(|c| tor_unescape(c).map(|c| c.into_owned()))
            .transpose()
            .map_err(|e| ControllerError::MalformedReply(e.to_string()))
    };
    let socks_username = get_credential(USERNAME_KEYWORD)?;
    let socks_password = get_credential(PASSWORD_KEYWORD)?;

    Ok(Some(Stream {
        stream_id,
        circuit_id,
        socks_username,
        socks_password,
    }))
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;

    #[test]
    fn simple() {
        let line = b"1 SENTCONNECT 0 1.2.3.4:443";
        let result = parse_stream_sentconnect(line);
        let stream = result.unwrap().unwrap();
        assert_eq!(stream.stream_id, StreamID::new(b"1").unwrap());
        assert_eq!(stream.circuit_id, CircuitID::new(b"0").unwrap());
        assert_eq!(stream.socks_username, None);
        assert_eq!(stream.socks_password, None);
    }

    #[test]
    fn with_socks_credentials() {
        let line =
            b"42 SENTCONNECT 100 (Tor_internal):80 SOCKS_USERNAME=\"user\" SOCKS_PASSWORD=\"pass\"";
        let result = parse_stream_sentconnect(line);
        let stream = result.unwrap().unwrap();
        assert_eq!(stream.stream_id, StreamID::new(b"42").unwrap());
        assert_eq!(stream.circuit_id, CircuitID::new(b"100").unwrap());
        assert_eq!(stream.socks_username, Some(Vec::from("user")));
        assert_eq!(stream.socks_password, Some(Vec::from("pass")));
    }

    #[test]
    fn only_username() {
        let line = b"5 SENTCONNECT 10 10.20.30.40:443 SOCKS_USERNAME=\"alice\"";
        let result = parse_stream_sentconnect(line);
        let stream = result.unwrap().unwrap();
        assert_eq!(stream.stream_id, StreamID::new(b"5").unwrap());
        assert_eq!(stream.circuit_id, CircuitID::new(b"10").unwrap());
        assert_eq!(stream.socks_username, Some(Vec::from("alice")));
        assert_eq!(stream.socks_password, None);
    }

    #[test]
    fn only_password() {
        let line = b"7 SENTCONNECT 20 192.168.1.2:22 SOCKS_PASSWORD=\"secret123\"";
        let result = parse_stream_sentconnect(line);
        let stream = result.unwrap().unwrap();
        assert_eq!(stream.stream_id, StreamID::new(b"7").unwrap());
        assert_eq!(stream.circuit_id, CircuitID::new(b"20").unwrap());
        assert_eq!(stream.socks_username, None);
        assert_eq!(stream.socks_password, Some(Vec::from("secret123")));
    }

    #[test]
    fn valid_but_other_status() {
        let line = b"1 DETACHED 0 172.16.10.2:8080";
        let result = parse_stream_sentconnect(line);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn status_case_insensitive() {
        let line = b"1 sentconnect 0 (Tor_internal):22 socks_username=USER socks_password=PASS";
        let result = parse_stream_sentconnect(line);
        let stream = result.unwrap().unwrap();
        assert_eq!(stream.stream_id, StreamID::new(b"1").unwrap());
        assert_eq!(stream.circuit_id, CircuitID::new(b"0").unwrap());
        assert_eq!(stream.socks_username.unwrap(), b"USER");
        assert_eq!(stream.socks_password.unwrap(), b"PASS");
    }

    #[test]
    fn status_mixed_case() {
        let line = b"1 SentConnect 0 192.168.4.5:22 SOcKs_USeRNAMe=User soCKS_paSSWoRd=paSs";
        let result = parse_stream_sentconnect(line);
        let stream = result.unwrap().unwrap();
        assert_eq!(stream.stream_id, StreamID::new(b"1").unwrap());
        assert_eq!(stream.circuit_id, CircuitID::new(b"0").unwrap());
        assert_eq!(stream.socks_username.unwrap(), b"User");
        assert_eq!(stream.socks_password.unwrap(), b"paSs");
    }

    #[test]
    fn status_unescape_error() {
        assert!(parse_stream_sentconnect(
            b"5 SENTCONNECT 10 10.20.30.40:443 SOCKS_USERNAME=\"alice\\xyz\""
        )
        .is_err());
    }

    #[test]
    fn large_ids() {
        {
            let line = b"9999999999999999 SENTCONNECT 1 5.6.7.8:80";
            let result = parse_stream_sentconnect(line);
            let stream = result.unwrap().unwrap();
            assert_eq!(
                stream.stream_id,
                StreamID::new(b"9999999999999999").unwrap()
            );
            assert_eq!(stream.circuit_id, CircuitID::new(b"1").unwrap());
        }
        {
            let line = b"2 SENTCONNECT 1234567890123456 5.6.7.8:80";
            let result = parse_stream_sentconnect(line);
            let stream = result.unwrap().unwrap();
            assert_eq!(stream.stream_id, StreamID::new(b"2").unwrap());
            assert_eq!(
                stream.circuit_id,
                CircuitID::new(b"1234567890123456").unwrap()
            );
        }
    }

    #[test]
    fn escaped_credentials() {
        let line =
            b"1 SENTCONNECT 0 (Tor_internal):80 SOCKS_USERNAME=\"user\\x20example\" SOCKS_PASSWORD=\"pass\\167ord\"";
        let result = parse_stream_sentconnect(line);
        let stream = result.unwrap().unwrap();
        assert_eq!(stream.socks_username.unwrap(), b"user example");
        assert_eq!(stream.socks_password.unwrap(), b"password");
    }

    #[test]
    fn alphanumeric_ids() {
        let line = b"abcX1y2Z3 SENTCONNECT ABCxyz 10.10.10.10:80";
        let result = parse_stream_sentconnect(line);
        let stream = result.unwrap().unwrap();
        assert_eq!(stream.stream_id, StreamID::new(b"abcX1y2Z3").unwrap());
        assert_eq!(stream.circuit_id, CircuitID::new(b"ABCxyz").unwrap());
    }

    #[test]
    fn stream_targets() {
        // Valid: IPv4
        assert!(
            parse_stream_sentconnect(b"1234 SENTCONNECT 5678 10.10.10.10:80")
                .unwrap()
                .is_some()
        );
        // Valid: IPv6
        assert!(
            parse_stream_sentconnect(b"1 SENTCONNECT 2 [cafe:dead:beef:0123::3200]:443")
                .unwrap()
                .is_some()
        );
        // Valid: hostname
        assert!(parse_stream_sentconnect(b"1 SENTCONNECT 2 example.org:443")
            .unwrap()
            .is_some());
        // Valid: (Tor_internal) literal
        assert!(
            parse_stream_sentconnect(b"1 SENTCONNECT 2 (Tor_internal):22")
                .unwrap()
                .is_some()
        );

        // Invalid (no port) but accepted anyway for now
        assert!(
            parse_stream_sentconnect(b"1234 SENTCONNECT 5678 12.34.56.78")
                .unwrap()
                .is_some()
        );
        assert!(
            parse_stream_sentconnect(b"1 SENTCONNECT 2 [cafe:dead:beef:0123::3200]")
                .unwrap()
                .is_some()
        );
        assert!(parse_stream_sentconnect(b"1 SENTCONNECT 2 example.net")
            .unwrap()
            .is_some());
        assert!(parse_stream_sentconnect(b"1 SENTCONNECT 2 (Tor_internal)")
            .unwrap()
            .is_some());
    }

    #[test]
    fn empty() {
        let line = b"";
        let result = parse_stream_sentconnect(line);
        assert_matches!(result.unwrap_err(), ControllerError::WrongFormat(_));
    }

    #[test]
    fn invalid() {
        let line = b"invalid input";
        let result = parse_stream_sentconnect(line);
        assert_matches!(result.unwrap_err(), ControllerError::WrongFormat(_));
    }

    #[test]
    fn missing_status() {
        let line = b"1 0 10.0.0.1:80";
        let result = parse_stream_sentconnect(line);
        assert_matches!(result.unwrap_err(), ControllerError::WrongFormat(_));
    }

    #[test]
    fn invalid_ids() {
        {
            let line = b"123+ SENTCONNECT 0 10.0.0.1:80";
            let result = parse_stream_sentconnect(line);
            assert_matches!(result.unwrap_err(), ControllerError::WrongFormat(_));
        }
        {
            let line = b"+++ SENTCONNECT 0 10.0.0.1:80";
            let result = parse_stream_sentconnect(line);
            assert_matches!(result.unwrap_err(), ControllerError::WrongFormat(_));
        }
        {
            let line = b"12345678901234567 SENTCONNECT 0 10.0.0.1:80";
            let result = parse_stream_sentconnect(line);
            assert_matches!(result.unwrap_err(), ControllerError::WrongFormat(_));
        }
        {
            let line = b"1 SENTCONNECT 123+-+ 10.0.0.1:80";
            let result = parse_stream_sentconnect(line);
            assert_matches!(result.unwrap_err(), ControllerError::WrongFormat(_));
        }
        {
            let line = b"1 SENTCONNECT +-+ 10.0.0.1:80";
            let result = parse_stream_sentconnect(line);
            assert_matches!(result.unwrap_err(), ControllerError::WrongFormat(_));
        }
    }
}
