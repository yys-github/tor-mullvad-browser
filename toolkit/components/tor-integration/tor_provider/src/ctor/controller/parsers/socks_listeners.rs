// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use lazy_static::lazy_static;
use regex::bytes::Regex;
use std::path::PathBuf;

use super::tor_unescape;
use crate::ctor::controller::{types::*, ControllerError};

lazy_static! {
    // This regular expression does not include $ on purpose, as we are looping
    // on all values: we get the first quoted value and consume any spaces at
    // end. Then we feed the rest of the string to the same regex, and if it
    // fails it means there was garbage and we should reject the entire reply.
    static ref LISTENERS_REGEX: Regex = Regex::new(r#"^ *("(?:[^"\\]|\\.)+")( +|$)"#)
        .expect("The regex is hardcoded, it should be good.");
}

/// Parse a Unix-domain socket path on Windows.
/// Since they are not supported on Windows, we return None.
#[cfg(target_os = "windows")]
fn make_path(_buf: &[u8]) -> Option<PathBuf> {
    None
}

/// Parse a Unix-domain socket on a non-Windows platform.
/// This function always returns a Some(<path>), even when the path is empty.
#[cfg(not(target_os = "windows"))]
fn make_path(buf: &[u8]) -> Option<PathBuf> {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
    Some(PathBuf::from(OsStr::from_bytes(buf)))
}

/// Parse a single SOCKS listener.
fn parse_listener(listener: &[u8]) -> Option<Result<SocksListener, ControllerError>> {
    let addr = match tor_unescape(listener) {
        Ok(a) => a,
        Err(e) => {
            return Some(Err(ControllerError::MalformedReply(e.to_string())));
        }
    };
    if addr.starts_with(b"unix:/") {
        make_path(&addr[5..]).map(|p| Ok(SocksListener::UnixSocket(p)))
    } else {
        Some(
            str::from_utf8(&*addr)
                .map_err(|e| ControllerError::MalformedReply(format!("{}", e)))
                .and_then(|s| {
                    s.parse()
                        .map_err(|e| ControllerError::MalformedReply(format!("{}", e)))
                })
                .map(|addr| SocksListener::Tcp(addr)),
        )
    }
}

/// Parse the output of a `GETINFO net/listeners/socks` command.
/// The reply is expected to be a single line which can contain multiple
/// listeners (each one quoted and separated by space).
///
/// A reply without listeners is valid (it will return an empty list) and
/// expected before the bootstrap.
///
/// We support IPv4 and IPv6 listeners on all platforms.
/// On non-Windows platforms, we support also Unix-domain sockets.
/// On Windows we parse them but we do not add them to the list.
pub fn parse_socks_listeners(line: &[u8]) -> Result<Vec<SocksListener>, ControllerError> {
    let mut listeners = Vec::new();

    // We use a manual loop because we want to make sure that items are head to
    // tail and we are not simply ignoring invalid content.
    let mut pos = 0;
    while pos < line.len() {
        let remaining = &line[pos..];
        match LISTENERS_REGEX.captures(remaining) {
            Some(cap) => {
                if let Some(l) = parse_listener(&cap[1]) {
                    listeners.push(l?);
                }
                pos += cap[0].len();
            }
            None => {
                return Err(ControllerError::WrongFormat(
                    String::from_utf8_lossy(line).into_owned(),
                ));
            }
        }
    }

    Ok(listeners)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddrV4, SocketAddrV6};

    #[test]
    fn single_ipv4() {
        assert_eq!(
            parse_socks_listeners(b"\"127.0.0.1:9150\"").unwrap(),
            vec![SocksListener::Tcp(
                SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9150).into()
            )]
        );
    }

    #[test]
    fn single_ipv6() {
        let addr_v6: SocketAddrV6 = "[2001:0db8:7000:0000:0000:dead:beef:1234]:9150"
            .parse()
            .unwrap();
        assert_eq!(
            parse_socks_listeners(b"\"[2001:0db8:7000:0000:0000:dead:beef:1234]:9150\"").unwrap(),
            vec![SocksListener::Tcp(addr_v6.into())]
        );
    }

    #[test]
    fn single_unix() {
        let line = b"\"unix:/tmp/test.sock\"";
        if cfg!(target_os = "windows") {
            assert_eq!(parse_socks_listeners(line).unwrap(), vec![]);
        } else {
            assert_eq!(
                parse_socks_listeners(line).unwrap(),
                vec![SocksListener::UnixSocket(PathBuf::from("/tmp/test.sock"))]
            );
        }
    }

    #[test]
    fn no_listeners() {
        assert_eq!(parse_socks_listeners(b"").unwrap(), vec![]);
    }

    #[test]
    fn multiple_listeners() {
        let mut listeners = vec![
            SocksListener::Tcp(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9050).into()),
            SocksListener::Tcp(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9150).into()),
        ];
        if cfg!(not(target_os = "windows")) {
            listeners.push(SocksListener::UnixSocket(PathBuf::from("/tmp/test.sock")));
        }
        assert_eq!(
            parse_socks_listeners(b"\"127.0.0.1:9050\" \"127.0.0.1:9150\" \"unix:/tmp/test.sock\"")
                .unwrap(),
            listeners
        );
    }

    #[test]
    fn unbalanced_quotes() {
        assert!(parse_socks_listeners(b"\"127.0.0.1:9050").is_err());
        assert!(parse_socks_listeners(b"127.0.0.1:9050\"").is_err());
        assert!(parse_socks_listeners(b"\"127.0.0.1:9050\" \"127.0.0.1:9051").is_err());
        assert!(
            parse_socks_listeners(b"\"127.0.0.1:9050\" \"127.0.0.1:9051 \"127.0.0.1:9052\"")
                .is_err()
        );
    }

    #[test]
    fn noise_in_reply() {
        assert!(parse_socks_listeners(b"garbage \"127.0.0.1:9050\"").is_err());
        assert!(parse_socks_listeners(b"\"127.0.0.1:9050\" garbage").is_err());
        assert!(parse_socks_listeners(b"garbage \"127.0.0.1:9050\" garbage").is_err());
        assert!(parse_socks_listeners(b"\"127.0.0.1:9050\" garbage \"127.0.0.1:9051\"").is_err());
        assert!(
            parse_socks_listeners(b"garbage \"127.0.0.1:9050\" garbage \"127.0.0.1:9051\"")
                .is_err()
        );
        assert!(
            parse_socks_listeners(b"\"127.0.0.1:9050\" garbage \"127.0.0.1:9051\" garbage")
                .is_err()
        );
        assert!(parse_socks_listeners(
            b"garbage \"127.0.0.1:9050\" garbage \"127.0.0.1:9051\" garbage"
        )
        .is_err());
        assert!(parse_socks_listeners(b"\"127.0.0.1:9050\" 127.0.0.1:9150").is_err());
    }

    #[test]
    fn spaces() {
        assert!(parse_socks_listeners(b"\"127.0.0.1:9050\"\"127.0.0.1:9150\"").is_err());
        assert_eq!(
            parse_socks_listeners(b"\"127.0.0.1:9050\"    \"127.0.0.1:9150\"").unwrap(),
            vec![
                SocksListener::Tcp(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9050).into()),
                SocksListener::Tcp(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9150).into())
            ]
        );
        assert_eq!(
            parse_socks_listeners(b"  \"127.0.0.1:9050\"     \"127.0.0.1:9150\"  ").unwrap(),
            vec![
                SocksListener::Tcp(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9050).into()),
                SocksListener::Tcp(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9150).into())
            ]
        );
    }

    #[test]
    fn invalid() {
        assert!(parse_socks_listeners(b"\"127.0.0.1:9050\" \"127.0.0.1:100000\"").is_err());
        assert!(parse_socks_listeners(b"\"127.0.0.1:100000\" \"127.0.0.1:9050\"").is_err());
        assert!(parse_socks_listeners(b"\"127.0.0.1\"").is_err());
        assert!(parse_socks_listeners(b"\"localhost:9050\"").is_err());
        assert!(parse_socks_listeners(b"\"\"").is_err());
    }
}
