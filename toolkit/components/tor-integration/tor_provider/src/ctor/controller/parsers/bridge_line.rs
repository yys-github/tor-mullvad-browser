// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use hex;
use lazy_static::lazy_static;
use regex::bytes::Regex;
use std::net::SocketAddr;
use thiserror::Error;

use crate::ctor::controller::types::Bridge;

lazy_static! {
    // In C-tor's parse_bridge_line (src/app/config/config.c), the transport is
    // parsed as a potential C identifier (starts with letter/underscores and
    // can contain any additional number of letters, numbers and underscores,
    // as defined in src/lib/string/util_string.c).
    //
    // The logic to distinguish between the fingerprint and args is the
    // following one (from C-tor's source code):
    //  - if the bridge does not use a transport, it cannot have arguments.
    //    If after the address there is anything, it must be the fingerprint;
    //  - otherwise, if there is a = sign, there is not a fingerprint.
    //
    // We do not use arguments in any way, so we do not enforce their format.
    // However, if we see a valid hex sequence, we still need to validate its
    // length, to make sure it actually is a fingerprint.
    //
    // We are not strict on spaces in case this is used with data provided
    // directly by users.
    static ref BRIDGE_REGEX: Regex = Regex::new(r"^ *(?:(?<transport>[A-Za-z_][A-Za-z0-9_]*) +)?(?<addr>[0-9a-fA-F\.\[\]\:]+:\d{1,5})(?: +(?<fingerprint>[0-9a-fA-F]+))?(?: +(?<args>.+?))? *$")
        .expect("The regex is hardcoded, it should be good.");
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BridgeParseError {
    #[error("the bridge line has a wrong format and could not be parsed")]
    InvalidFormat,
    #[error("invalid address")]
    InvalidAddress,
    #[error("port 0 is not valid for bridges")]
    InvalidPort,
    #[error("invalid length of the bridge fingerprint")]
    InvalidFingerprint,
    #[error("arguments can be supplied only when a transport is in use")]
    ArgsWithoutTransport,
}

pub fn parse_bridge_line(value: &[u8]) -> Result<Bridge, BridgeParseError> {
    let caps = BRIDGE_REGEX
        .captures(value)
        .ok_or(BridgeParseError::InvalidFormat)?;

    let transport = caps
        .name("transport")
        .map(|m| String::from_utf8_lossy(m.as_bytes()).into_owned());

    let address: SocketAddr = caps
        .name("addr")
        .and_then(|a| str::from_utf8(a.as_bytes()).ok())
        // This cannot really happen since the regex matched...
        .ok_or(BridgeParseError::InvalidFormat)?
        .parse()
        // Rust validation is very generic, so it is not even worth to
        // include in our error.
        .map_err(|_| BridgeParseError::InvalidAddress)?;
    if address.port() == 0 {
        return Err(BridgeParseError::InvalidPort);
    }

    let fingerprint = caps
        .name("fingerprint")
        .and_then(|m| {
            let mut fp = [0u8; 20];
            // We match only hex characters in the regex, so this should error
            // only if the size is wrong.
            match hex::decode_to_slice(m.as_bytes(), &mut fp[..]) {
                Ok(_) => Some(Ok(fp)),
                Err(_) => Some(Err(BridgeParseError::InvalidFingerprint)),
            }
        })
        .transpose()?;

    let args = caps.name("args");
    if transport.is_none() && args.is_some() {
        // src/app/config/config.c, parse_bridge_line: "If transports are
        // disabled, next field must be a fingerprint.".
        return Err(BridgeParseError::ArgsWithoutTransport);
    }

    Ok(Bridge {
        transport,
        address,
        fingerprint,
        args: args.map(|m| m.as_bytes().to_vec()),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        assert_eq,
        net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6},
    };

    use super::*;

    #[test]
    fn vanilla() {
        {
            let bridge = parse_bridge_line(b"192.168.1.4:443").unwrap();
            assert_eq!(bridge.transport, None);
            assert_eq!(
                bridge.address,
                SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 4), 443).into()
            );
            assert_eq!(bridge.fingerprint, None);
            assert_eq!(bridge.args, None);
        }
        {
            let bridge = parse_bridge_line(b"[dead:BEEF::1234]:443").unwrap();
            assert_eq!(bridge.transport, None);
            assert_eq!(
                bridge.address,
                SocketAddrV6::new(
                    Ipv6Addr::new(0xdead, 0xbeef, 0, 0, 0, 0, 0, 0x1234),
                    443,
                    0,
                    0
                )
                .into()
            );
            assert_eq!(bridge.fingerprint, None);
            assert_eq!(bridge.args, None);
        }
    }

    #[test]
    fn transport() {
        {
            let bridge = parse_bridge_line(b"test 1.2.3.4:567").unwrap();
            assert_eq!(bridge.transport, Some(String::from("test")));
            assert_eq!(
                bridge.address,
                SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 567).into()
            );
            assert_eq!(bridge.fingerprint, None);
            assert_eq!(bridge.args, None);
        }
        {
            let bridge = parse_bridge_line(b"test [cafe:CAFE::BaBe]:567").unwrap();
            assert_eq!(bridge.transport, Some(String::from("test")));
            assert_eq!(
                bridge.address,
                SocketAddrV6::new(
                    Ipv6Addr::new(0xcafe, 0xcafe, 0, 0, 0, 0, 0, 0xbabe),
                    567,
                    0,
                    0
                )
                .into()
            );
            assert_eq!(bridge.fingerprint, None);
            assert_eq!(bridge.args, None);
        }
    }

    #[test]
    fn transport_fingerprint() {
        {
            let bridge = parse_bridge_line(
                b"obfs4 37.218.245.14:38224 D9A82D2F9C2F65A18407B1D2B764F130847F8B5D",
            )
            .unwrap();
            assert_eq!(bridge.transport, Some(String::from("obfs4")));
            assert_eq!(
                bridge.address,
                SocketAddrV4::new(Ipv4Addr::new(37, 218, 245, 14), 38224).into()
            );
            assert_eq!(
                bridge.fingerprint.as_ref().map(|fp| &fp[..]),
                Some(
                    hex::decode("D9A82D2F9C2F65A18407B1D2B764F130847F8B5D")
                        .unwrap()
                        .as_slice()
                )
            );
            assert_eq!(bridge.args, None);
        }
        {
            let bridge = parse_bridge_line(
                b"obfs4 [1234:56:789::abcd]:38224 D9A82D2F9C2F65A18407B1D2B764F130847F8B5D",
            )
            .unwrap();
            assert_eq!(bridge.transport, Some(String::from("obfs4")));
            assert_eq!(
                bridge.address,
                SocketAddrV6::new(
                    Ipv6Addr::new(0x1234, 0x56, 0x789, 0, 0, 0, 0, 0xabcd),
                    38224,
                    0,
                    0
                )
                .into()
            );
            assert_eq!(
                bridge.fingerprint.as_ref().map(|fp| &fp[..]),
                Some(
                    hex::decode("D9A82D2F9C2F65A18407B1D2B764F130847F8B5D")
                        .unwrap()
                        .as_slice()
                )
            );
            assert_eq!(bridge.args, None);
        }
    }

    #[test]
    fn all() {
        {
            let bridge = parse_bridge_line(
                b"obfs4 37.218.245.14:38224 D9A82D2F9C2F65A18407B1D2B764F130847F8B5D iat-mode=0",
            )
            .unwrap();
            assert_eq!(bridge.transport, Some(String::from("obfs4")));
            assert_eq!(
                bridge.address,
                SocketAddrV4::new(Ipv4Addr::new(37, 218, 245, 14), 38224).into()
            );
            assert_eq!(
                bridge.fingerprint.as_ref().map(|fp| &fp[..]),
                Some(
                    hex::decode("D9A82D2F9C2F65A18407B1D2B764F130847F8B5D")
                        .unwrap()
                        .as_slice()
                )
            );
            assert_eq!(&bridge.args.unwrap(), b"iat-mode=0");
        }
        {
            let bridge = parse_bridge_line(
                b"obfs4 [1234:56:789:0a:00b:000c:d:eeee]:38224 D9A82D2F9C2F65A18407B1D2B764F130847F8B5D iat-mode=0",
            )
            .unwrap();
            assert_eq!(bridge.transport, Some(String::from("obfs4")));
            assert_eq!(
                bridge.address,
                SocketAddrV6::new(
                    Ipv6Addr::new(0x1234, 0x56, 0x789, 0xa, 0xb, 0xc, 0xd, 0xeeee),
                    38224,
                    0,
                    0
                )
                .into()
            );
            assert_eq!(
                bridge.fingerprint.as_ref().map(|fp| &fp[..]),
                Some(
                    hex::decode("D9A82D2F9C2F65A18407B1D2B764F130847F8B5D")
                        .unwrap()
                        .as_slice()
                )
            );
            assert_eq!(&bridge.args.unwrap(), b"iat-mode=0");
        }
    }

    #[test]
    fn no_fingerprint() {
        let bridge = parse_bridge_line(b"test 1.2.3.4:567 iat-mode=0").unwrap();
        assert_eq!(bridge.transport.unwrap(), "test");
        assert_eq!(
            bridge.address,
            SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 567).into()
        );
        assert_eq!(bridge.fingerprint, None);
        assert_eq!(&bridge.args.unwrap(), b"iat-mode=0");
    }

    #[test]
    fn multiple_args_and_spaces() {
        let bridge = parse_bridge_line(
            b"   obfs4    37.218.245.14:38224      D9A82D2F9C2F65A18407B1D2B764F130847F8B5D     iat-mode=0 other-arg spaces-at-the-end  ",
        )
        .unwrap();
        assert_eq!(
            &bridge.args.unwrap(),
            b"iat-mode=0 other-arg spaces-at-the-end"
        );
    }

    #[test]
    fn failures() {
        // Only IP address without a port. Does not even match the regex.
        assert_eq!(
            parse_bridge_line(b"192.168.1.4").unwrap_err(),
            BridgeParseError::InvalidFormat,
        );
        // Invalid port (and again, does not match the regex).
        assert_eq!(
            parse_bridge_line(b"192.168.1.4:100000").unwrap_err(),
            BridgeParseError::InvalidFormat,
        );
        // No address
        assert_eq!(
            parse_bridge_line(b"obfs4 D9A82D2F9C2F65A18407B1D2B764F130847F8B5D").unwrap_err(),
            BridgeParseError::InvalidFormat,
        );
        // No address and bad fingerprint (bad no address prevails).
        assert_eq!(
            parse_bridge_line(b"obfs4 aaa1234").unwrap_err(),
            BridgeParseError::InvalidFormat,
        );

        assert_eq!(
            parse_bridge_line(b"192.168.1.375:1234").unwrap_err(),
            BridgeParseError::InvalidAddress,
        );
        assert_eq!(
            parse_bridge_line(b"192.168.1.4:0").unwrap_err(),
            BridgeParseError::InvalidPort,
        );
        assert_eq!(
            parse_bridge_line(b"192.168.1.4:99999").unwrap_err(),
            BridgeParseError::InvalidAddress,
        );
        assert_eq!(
            parse_bridge_line(b"192.168.1.4:65536").unwrap_err(),
            BridgeParseError::InvalidAddress,
        );

        assert_eq!(
            parse_bridge_line(b"1.2.3.4:443 aaaaaaaaa").unwrap_err(),
            BridgeParseError::InvalidFingerprint
        );
        assert_eq!(
            parse_bridge_line(
                b"1.2.3.4:443 0123456789012345678901234567890123456789aaaaa"
            )
            .unwrap_err(),
            BridgeParseError::InvalidFingerprint
        );

        assert_eq!(
            parse_bridge_line(
                b"1.2.3.4:443 0123456789012345678901234567890123456789 key=value"
            ).unwrap_err(),
            BridgeParseError::ArgsWithoutTransport,
        );
        assert_eq!(
            parse_bridge_line(
                b"1.2.3.4:443 key=value"
            ).unwrap_err(),
            BridgeParseError::ArgsWithoutTransport,
        );
    }
}
