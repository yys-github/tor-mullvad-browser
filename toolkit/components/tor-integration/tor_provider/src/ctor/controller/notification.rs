// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::{Bytes, BytesMut};

use super::{
    parsers::{parse_bootstrap_status, parse_circuit_built, parse_stream_sentconnect},
    types::{BootstrapStatus, Circuit, CircuitID, Stream},
};
use crate::ctor::reply_parser::{DetailReplyLine, Reply};

pub trait NotificationListener {
    fn log_message(&self, severity: String, message: Bytes);
    fn bootstrap_status(&self, status: BootstrapStatus);
    fn circuit_built(&self, circuit: Circuit);
    fn circuit_closed(&self, id: CircuitID);
    fn stream_sent_connect(&self, stream: Stream);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsyncNotification {
    LogMessage { severity: String, message: Bytes },
    BootstrapStatus(BootstrapStatus),
    CircuitBuilt(Circuit),
    CircuitClosed(CircuitID),
    StreamSentConnect(Stream),
}

impl AsyncNotification {
    /// Try to parse a reply as an async notification.
    /// Async notifications do not have an error handler, so any parsing error
    /// will cause the reply to be just dropped silently.
    pub fn parse(reply: Reply) -> Option<Self> {
        if !reply.is_async() {
            return None;
        }
        let first_line = match reply.details().first() {
            Some(DetailReplyLine::MidReplyLine { line, .. }) => line,
            Some(DetailReplyLine::DataReplyLine { lines, .. }) => {
                if lines.is_empty() {
                    debug_assert!(false, "DataReplyLine must always have at least one line.");
                    return None;
                }
                &lines[0]
            }
            None => &reply.end_line().line,
        };
        let (keyword, data) = get_type(first_line)?;
        match keyword.as_str() {
            "STATUS_CLIENT" => Self::parse_bootstrap_status(data),
            "CIRC" => Self::parse_circuit_event(data),
            "STREAM" => Self::parse_stream_event(data),
            "DEBUG" | "INFO" | "NOTICE" | "WARN" | "ERR" => Self::parse_log_message(keyword, reply),
            _ => {
                let mut buf = Vec::new();
                let _ = reply.write_to(&mut buf);
                log::debug!("Unknown notification: {}", String::from_utf8_lossy(&buf));
                None
            }
        }
    }

    /// Try to parse a `STATUS_CLIENT ... BOOTSTRAP` event.
    /// We will silently drop any other kind of STATUS_CLIENT event.
    fn parse_bootstrap_status(data: &[u8]) -> Option<Self> {
        parse_bootstrap_status(data)
            .map(|status| AsyncNotification::BootstrapStatus(status))
            .ok()
    }

    /// Try to parse CIRC BUILT/CLOSED events.
    fn parse_circuit_event(data: &[u8]) -> Option<Self> {
        let (status, id) = circuit_stream_status(data)?;
        match status.as_str() {
            "BUILT" => parse_circuit_built(data)
                .ok()
                .and_then(|c| c)
                .map(AsyncNotification::CircuitBuilt),
            "CLOSED" => Some(AsyncNotification::CircuitClosed(id)),
            _ => None,
        }
    }

    /// Try to parse a STREAM SENTCONNECT event.
    fn parse_stream_event(data: &[u8]) -> Option<Self> {
        let (status, _) = circuit_stream_status(data)?;
        match status.as_str() {
            "SENTCONNECT" => parse_stream_sentconnect(data)
                .ok()
                .and_then(|stream| stream.map(|s| AsyncNotification::StreamSentConnect(s))),
            _ => None,
        }
    }

    /// Try to parse a log message.
    fn parse_log_message(severity: String, reply: Reply) -> Option<Self> {
        match reply.details().first() {
            Some(DetailReplyLine::DataReplyLine { lines, .. }) => {
                if lines.len() == 1 {
                    None
                } else {
                    // No need to join the last line, as it is supposed to be a
                    // `650 OK`.
                    Some(AsyncNotification::LogMessage {
                        severity,
                        message: join_lines(&lines[1..]),
                    })
                }
            }
            None if !reply.has_details() => {
                if reply.end_line().line.len() <= (severity.len() + 1) {
                    None
                } else {
                    let message = reply.end_line().line.slice(severity.len() + 1..);
                    Some(AsyncNotification::LogMessage { severity, message })
                }
            }
            _ => {
                // The logs use data lines, as per the specs.
                None
            }
        }
    }

    /// Consume self by sending it to a NotificationListener.
    pub fn dispatch(self, listener: &dyn NotificationListener) {
        match self {
            AsyncNotification::LogMessage { severity, message } => {
                listener.log_message(severity, message)
            }
            AsyncNotification::BootstrapStatus(status) => listener.bootstrap_status(status),
            AsyncNotification::CircuitBuilt(circuit) => listener.circuit_built(circuit),
            AsyncNotification::CircuitClosed(id) => listener.circuit_closed(id),
            AsyncNotification::StreamSentConnect(stream) => listener.stream_sent_connect(stream),
        }
    }
}

/// Split a reply line into keyword and data.
fn get_type<'a>(data: &'a [u8]) -> Option<(String, &'a [u8])> {
    if data.is_empty() {
        return None;
    }
    let mut parts = data.splitn(2, |b| *b == b' ');
    let keyword = make_keyword(parts.next())?;
    let data = parts.next().unwrap_or(&[]);
    Some((keyword, data))
}

/// Validate a CIRC/STREAM event.
/// We handle them together as they have the same structure:
/// `<id> <event-type> ...` (with compatible formats).
///
/// Notice that we have both a CircuitID and a StreamID type, but for stream we
/// drop the ID, so as a matter of fact we only use the CircuitID variant.
/// Therefore, we thought it was not worth to make this a generic function.
fn circuit_stream_status(data: &[u8]) -> Option<(String, CircuitID)> {
    let mut parts = data.split(|b| *b == b' ');
    let id = parts.next().and_then(|t| CircuitID::new(t));
    make_keyword(parts.next()).zip(id)
}

/// Make a keyword (uppercase string).
/// We use only ASCII keywords for now, but this function accepts anything as
/// long as it is valid UTF-8 (including ASCII).
///
/// Notice that we need to return an owned string as we are doing the case
/// conversion.
fn make_keyword(keyword: Option<&[u8]>) -> Option<String> {
    // Our keywords are ASCII, so ignore this error silently, we are not
    // interested in those anyway...
    keyword.and_then(|kw| String::from_utf8(kw.to_ascii_uppercase()).ok())
}

/// Join log message lines.
fn join_lines(lines: &[Bytes]) -> Bytes {
    let mut result = BytesMut::new();
    for (i, line) in lines.iter().enumerate() {
        result.extend_from_slice(line);
        if i < lines.len() - 1 {
            result.extend_from_slice(b"\r\n");
        }
    }
    result.freeze()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctor::reply_parser::make_reply;

    #[test]
    fn test_make_keyword() {
        assert_eq!(make_keyword(Some(&b"TEST"[..])).unwrap(), "TEST");
        assert_eq!(make_keyword(Some(&b"test"[..])).unwrap(), "TEST");
        assert_eq!(make_keyword(Some(&b"TesT"[..])).unwrap(), "TEST");
        assert_eq!(make_keyword(Some(&b"Te_sT"[..])).unwrap(), "TE_ST");

        assert!(make_keyword(None).is_none());

        let invalid_utf8 = [0xFF, 0xFE];
        assert!(make_keyword(Some(&invalid_utf8)).is_none());
    }

    #[test]
    fn test_get_type() {
        {
            let (keyword, data) = get_type(b"STATUS_CLIENT BOOTSTRAP PROGRESS=50").unwrap();
            assert_eq!(keyword, "STATUS_CLIENT");
            assert_eq!(data, b"BOOTSTRAP PROGRESS=50");
        }
        {
            let (keyword, data) = get_type(b"CIRC").unwrap();
            assert_eq!(keyword, "CIRC");
            assert_eq!(data, &b""[..]);
        }
        {
            let (keyword, _) = get_type(b"debug message").unwrap();
            assert_eq!(keyword, "DEBUG");
        }
        {
            let (keyword, _) = get_type(b"StAtUs_ClIeNt data").unwrap();
            assert_eq!(keyword, "STATUS_CLIENT");
        }
        {
            let (keyword, data) = get_type(b"CIRC   id BUILT").unwrap();
            assert_eq!(keyword, "CIRC");
            assert_eq!(data, b"  id BUILT"); // Splits on first space only
        }
        assert!(get_type(b"").is_none());
    }

    #[test]
    fn test_circuit_stream_status() {
        {
            let (status, id) = circuit_stream_status(b"123 BUILT").unwrap();
            assert_eq!(status, "BUILT");
            assert_eq!(id, CircuitID::new(b"123").unwrap());
        }
        {
            let (status, id) = circuit_stream_status(b"456 CLOSED").unwrap();
            assert_eq!(status, "CLOSED");
            assert_eq!(id, CircuitID::new(b"456").unwrap());
        }
        {
            let (status, _) = circuit_stream_status(b"1 built").unwrap();
            assert_eq!(status, "BUILT");
        }
        {
            let (status, _) = circuit_stream_status(b"1 bUIlt").unwrap();
            assert_eq!(status, "BUILT");
        }

        // Missing ID
        assert!(circuit_stream_status(b"BUILT").is_none());
        // Missing status
        assert!(circuit_stream_status(b"123").is_none());
        // Invalid ID
        assert!(circuit_stream_status(b"ab+c BUILT").is_none());
        // Empty
        assert!(circuit_stream_status(b"").is_none());
    }

    #[test]
    fn test_join_lines() {
        {
            let lines = vec![Bytes::from_static(b"single line")];
            let result = join_lines(&lines);
            assert_eq!(result, "single line");
        }
        {
            let lines = vec![
                Bytes::from_static(b"aaaaaaaaaaa 1234"),
                Bytes::from_static(b"b b"),
                Bytes::from_static(b"cccc cc"),
            ];
            let result = join_lines(&lines);
            assert_eq!(result, "aaaaaaaaaaa 1234\r\nb b\r\ncccc cc");
        }
        {
            let lines: Vec<Bytes> = vec![];
            let result = join_lines(&lines);
            assert_eq!(result, "");
        }
        {
            let lines = vec![
                Bytes::from_static(b"second line is"),
                Bytes::from_static(b""),
                Bytes::from_static(b"empty"),
            ];
            let result = join_lines(&lines);
            assert_eq!(result, "second line is\r\n\r\nempty");
        }
    }

    #[test]
    fn debug_log() {
        {
            let reply = make_reply(b"650 DEBUG Some debug message\r\n");
            let notification = AsyncNotification::parse(reply).unwrap();
            match notification {
                AsyncNotification::LogMessage { severity, message } => {
                    assert_eq!(severity, "DEBUG");
                    assert_eq!(message, "Some debug message");
                }
                _ => unreachable!("Expected LogMessage"),
            }
        }
        {
            let reply = make_reply(b"650+DEBUG\r\nSome debug message\r\n.\r\n650 OK\r\n");
            let notification = AsyncNotification::parse(reply).unwrap();
            match notification {
                AsyncNotification::LogMessage { severity, message } => {
                    assert_eq!(severity, "DEBUG");
                    assert_eq!(message, "Some debug message");
                }
                _ => unreachable!("Expected LogMessage"),
            }
        }
        {
            let reply = make_reply(b"650-DEBUG Some debug message\r\n650 OK\r\n");
            assert!(AsyncNotification::parse(reply).is_none());
        }
    }

    #[test]
    fn info_log() {
        let reply = make_reply(b"650 INFO Connection established\r\n");
        let notification = AsyncNotification::parse(reply).unwrap();
        match notification {
            AsyncNotification::LogMessage { severity, message } => {
                assert_eq!(severity, "INFO");
                assert_eq!(message, "Connection established");
            }
            _ => unreachable!("Expected LogMessage"),
        }
    }

    #[test]
    fn warn_log() {
        let reply = make_reply(b"650 WARN Something suspicious happened\r\n");
        let notification = AsyncNotification::parse(reply).unwrap();
        match notification {
            AsyncNotification::LogMessage { severity, message } => {
                assert_eq!(severity, "WARN");
                assert_eq!(message, "Something suspicious happened");
            }
            _ => unreachable!("Expected LogMessage"),
        }
    }

    #[test]
    fn err_log() {
        let reply = make_reply(b"650 ERR Critical error occurred\r\n");
        let notification = AsyncNotification::parse(reply).unwrap();
        match notification {
            AsyncNotification::LogMessage { severity, message } => {
                assert_eq!(severity, "ERR");
                assert_eq!(message, "Critical error occurred")
            }
            _ => unreachable!("Expected LogMessage"),
        }
    }

    #[test]
    fn notice_log() {
        let reply = make_reply(b"650 NOTICE Important notice\r\n");
        let notification = AsyncNotification::parse(reply).unwrap();
        match notification {
            AsyncNotification::LogMessage { severity, message } => {
                assert_eq!(severity, "NOTICE");
                assert_eq!(message, "Important notice");
            }
            _ => unreachable!("Expected LogMessage"),
        }
    }

    #[test]
    fn empty_log() {
        assert!(AsyncNotification::parse(make_reply(b"650 NOTICE\r\n")).is_none());
        assert!(AsyncNotification::parse(make_reply(b"650 NOTICE \r\n")).is_none());
        // At the moment, we do not trim spaces.
        assert!(AsyncNotification::parse(make_reply(b"650 NOTICE    \r\n")).is_some());
        assert!(AsyncNotification::parse(make_reply(b"650+NOTICE\r\n.\r\n650 OK\r\n")).is_none());
        assert_eq!(
            AsyncNotification::parse(make_reply(b"650+NOTICE\r\n\r\n.\r\n650 OK\r\n")).unwrap(),
            AsyncNotification::LogMessage {
                severity: String::from("NOTICE"),
                message: Bytes::new(),
            }
        );
    }

    #[test]
    fn bootstrap_status() {
        let reply = make_reply(b"650 STATUS_CLIENT NOTICE BOOTSTRAP PROGRESS=10 TAG=conn_done SUMMARY=\"Connected to a relay\"\r\n");
        let notification = AsyncNotification::parse(reply).unwrap();
        match notification {
            AsyncNotification::BootstrapStatus(_) => {
                // Actual parsing delegated to parse_bootstrap_status
            }
            _ => unreachable!("Expected BootstrapStatus"),
        }
    }

    #[test]
    fn circuit_built() {
        let reply = make_reply(b"650 CIRC 42 BUILT $1111111111111111111111111111111111111111\r\n");
        let notification = AsyncNotification::parse(reply).unwrap();
        match notification {
            AsyncNotification::CircuitBuilt(_) => {}
            _ => unreachable!("Expected CircuitBuilt"),
        }
    }

    #[test]
    fn circuit_closed() {
        {
            let reply = make_reply(b"650 CIRC 42 CLOSED\r\n");
            let notification = AsyncNotification::parse(reply).unwrap();
            match notification {
                AsyncNotification::CircuitClosed(id) => {
                    assert_eq!(id, CircuitID::new(b"42").unwrap());
                }
                _ => unreachable!("Expected CircuitClosed"),
            }
        }
        {
            let reply = make_reply(b"650-CIRC 42 CLOSED\r\n650 Extended data\r\n");
            let notification = AsyncNotification::parse(reply).unwrap();
            match notification {
                AsyncNotification::CircuitClosed(id) => {
                    assert_eq!(id, CircuitID::new(b"42").unwrap());
                }
                _ => unreachable!("Expected CircuitClosed"),
            }
        }
    }

    #[test]
    fn stream_sentconnect() {
        let reply = make_reply(b"650 STREAM 1 SENTCONNECT 1 1.2.3.4:443\r\n");
        let notification = AsyncNotification::parse(reply).unwrap();
        match notification {
            AsyncNotification::StreamSentConnect(_) => {}
            _ => unreachable!("Expected StreamSentConnect"),
        }
    }

    #[test]
    fn parse_errors() {
        {
            let reply = make_reply(b"650 UNKNOWN_EVENT data\r\n");
            assert!(AsyncNotification::parse(reply).is_none());
        }
        {
            // Empty keyword
            let reply = make_reply(b"650 \r\n");
            assert!(AsyncNotification::parse(reply).is_none());
        }
        {
            let reply = make_reply(b"250 Not an async reply\r\n");
            assert!(AsyncNotification::parse(reply).is_none());
        }
        // Unkown statuses
        {
            let reply = make_reply(b"650 CIRC 42 EXTENDED\r\n");
            assert!(AsyncNotification::parse(reply).is_none());
        }
        {
            let reply = make_reply(b"650 STREAM 1 ATTACHED\r\n");
            assert!(AsyncNotification::parse(reply).is_none());
        }
        // Invalid IDs
        {
            let reply = make_reply(b"650 CIRC a_b_c BUILT\r\n");
            assert!(AsyncNotification::parse(reply).is_none());
        }
        {
            let reply = make_reply(b"650 STREAM 123 SENTCONNECT a_b_c 1.2.3.4:443\r\n");
            assert!(AsyncNotification::parse(reply).is_none());
        }
        {
            let reply = make_reply(b"650 STREAM 12_3 SENTCONNECT abc 1.2.3.4:443\r\n");
            assert!(AsyncNotification::parse(reply).is_none());
        }
    }
}
