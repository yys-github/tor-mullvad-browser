// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use lazy_static::lazy_static;
use regex::bytes::Regex;
use std::net::{Ipv4Addr, SocketAddrV6};

use crate::ctor::controller::{types::*, ControllerError};

lazy_static! {
    // First line is always:
    // `r nickname identity digest PubDate PubTime IP ORPort DirPort`
    // https://spec.torproject.org/dir-spec/consensus-formats.html#section:router-status
    static ref R_LINE_REGEX: Regex = Regex::new(
        r"^r [A-Za-z0-9]+ [A-Za-z0-9+/]+ [A-Za-z0-9+/=]+ \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} ([0-9\.]{7,15}) \d{1,5} \d{1,5}$",
    )
    .expect("The regex is hardcoded, it should be good.");
}

/// Parse a router status and return its IPv4 (and IPv6, if available).
/// We follow the specs from:
/// https://spec.torproject.org/dir-spec/consensus-formats.html#section:router-status
///
/// We expect the first line to have this format:
/// `r nickname identity digest PubDate PubTime IP ORPort DirPort`
/// We only validate the allowed characters, we do not validate all the fields.
///
/// We ignore any other lines, except the ones for IPv6 addresses, that have
/// this format: `a [<IPv6-address>]:port`.
/// Only the first valid "a" line will be picked up.
///
/// This function parses one router at the time.
pub fn addresses_from_status_info(lines: &[&[u8]]) -> Result<NodeAddresses, ControllerError> {
    if lines.is_empty() {
        return Err(ControllerError::MalformedReply(String::from(
            "empty status info",
        )));
    }

    let Some(r_line) = R_LINE_REGEX.captures(lines[0]) else {
        return Err(ControllerError::MalformedReply(format!(
            "the first line cannot be parsed: {}",
            String::from_utf8_lossy(lines[0])
        )));
    };
    let ipv4: Ipv4Addr = str::from_utf8(&r_line[1])
        .map_err(|e| ControllerError::MalformedReply(e.to_string()))?
        .parse()
        .map_err(|e| ControllerError::MalformedReply(format!("{}", e)))?;

    let ipv6 = lines[1..].iter().find_map(|line| {
        if !line.starts_with(b"a ") {
            return None;
        }
        let s = str::from_utf8(&line[2..]).ok()?;
        let addrv6: SocketAddrV6 = s.parse().ok()?;
        Some(*addrv6.ip())
    });

    Ok(NodeAddresses { ipv4, ipv6 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{assert_matches, net::Ipv6Addr};

    #[test]
    fn happy_both() {
        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname abAB123+/xyzXYZaaaaaaaaaaaa bbbbbbbbabAB123+/xyzXYZbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"a [fdc8:08d6:f860:d6c::3100]:9001",
                b"s Fast Guard HSDir Running Stable V2Dir Valid",
                b"w Bandwidth=5000",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: Some(Ipv6Addr::new(
                    0xfdc8, 0x08d6, 0xf860, 0xd6c, 0, 0, 0, 0x3100
                )),
            }
        );
        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"s Fast Guard HSDir Running Stable V2Dir Valid",
                b"w Bandwidth=5000",
                b"a [fdc8:08d6:f860:d6c::3100]:9001",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: Some(Ipv6Addr::new(
                    0xfdc8, 0x08d6, 0xf860, 0xd6c, 0, 0, 0, 0x3100
                )),
            }
        );
    }

    #[test]
    fn only_v4() {
        assert_eq!(
            addresses_from_status_info(&[b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0"]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: None
            }
        );
    }

    #[test]
    fn invalid_v6_ignored() {
        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"a [zzzz:08d6:f860:d6c::3100]:9001",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: None
            }
        );

        // Missing the port is also rejected for IPv6
        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"a [fdc8:08d6:f860:d6c::3100]",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: None
            }
        );

        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"a",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: None
            }
        );

        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"a ",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: None
            }
        );

        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"abc [fdc8:dead:beef:d6c::3100]:9001",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: None
            }
        );

        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"a 127.0.0.1",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: None
            }
        );
    }

    #[test]
    fn multiple_a() {
        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"a [cafe:dead:beef:0123::3100]:9001",
                b"a [cafe:dead:beef:0123::3200]:9001",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: Some(Ipv6Addr::new(
                    0xcafe, 0xdead, 0xbeef, 0x0123, 0, 0, 0, 0x3100
                ))
            }
        );

        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"a [cafe:dead:beef:0123::3100]:9001",
                b"a [cafe:dead:xxxx:0123::3200]:9001",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: Some(Ipv6Addr::new(
                    0xcafe, 0xdead, 0xbeef, 0x0123, 0, 0, 0, 0x3100
                ))
            }
        );

        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"a [cafe:dead:xxxx:0123::3200]:9001",
                b"a [cafe:dead:beef:0123::3100]:9001",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: Some(Ipv6Addr::new(
                    0xcafe, 0xdead, 0xbeef, 0x0123, 0, 0, 0, 0x3100
                ))
            }
        );

        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"a ",
                b"a [cafe:dead:beef:0123::3100]:9001",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: Some(Ipv6Addr::new(
                    0xcafe, 0xdead, 0xbeef, 0x0123, 0, 0, 0, 0x3100
                ))
            }
        );

        assert_eq!(
            addresses_from_status_info(&[
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
                b"a",
                b"a [cafe:dead:beef:0123::3100]:9001",
            ]).unwrap(),
            NodeAddresses {
                ipv4: Ipv4Addr::new(192, 168, 1, 3),
                ipv6: Some(Ipv6Addr::new(
                    0xcafe, 0xdead, 0xbeef, 0x0123, 0, 0, 0, 0x3100
                ))
            }
        );
    }

    #[test]
    fn invalid_r() {
        assert_matches!(
            addresses_from_status_info(&[]).unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            addresses_from_status_info(&[b"does not start with r"]).unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            addresses_from_status_info(&[
                b"r nick$$name aaaa bbbb 2038-01-01 00:00:00 127.0.0.1 9001 0"
            ])
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            addresses_from_status_info(&[
                b"r nickname aa!@aa bbbb 2038-01-01 00:00:00 127.0.0.1 9001 0"
            ])
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            addresses_from_status_info(&[
                b"r nickname aaaa bbbb 2038-1-01 00:00:00 127.0.0.1 9001 0"
            ])
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            addresses_from_status_info(&[
                b"r nickname aaaa bbbb 2038-01-01 00:00:00 127.0.0.1000 9001 0"
            ])
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            addresses_from_status_info(&[
                b"r nickname aaaa bbbb 2038-01-01 00:00:00 ....123 9001 0"
            ])
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        // Bad IPv4 (an octet is 3 digits but higher than 255)
        assert_matches!(
            addresses_from_status_info(&[
                b"r nickname aaaa bbbb 2038-01-01 00:00:00 127.0.0.300 9001 0"
            ])
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            addresses_from_status_info(&[
                b"r nickname aaaa bbbb 2000-06-08 15:10:00 1.2.3.4 100000 0"
            ])
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            addresses_from_status_info(&[
                b"rrr nickname aaaa bbbb 2000-06-08 15:10:00 1.2.3.4 1000 0"
            ])
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            addresses_from_status_info(&[
                b"garbage r nickname aaaa bbbb 2000-06-08 15:10:00 1.2.3.4 9000 0"
            ])
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            addresses_from_status_info(&[
                b"r nickname aaaa bbbb 2000-06-08 15:10:00 1.2.3.4 443 0 garbage"
            ])
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            addresses_from_status_info(&[
                b"R nickname aaaa bbbb 2000-06-08 15:10:00 1.2.3.4 443 0"
            ])
            .unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        // Only IPv6 is not valid
        assert_matches!(
            addresses_from_status_info(&[b"a [fdc8:08d6:f860:d6c::3100]:9001"]).unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        // r must come first
        assert_matches!(
            addresses_from_status_info(&[
                b"a [fdc8:08d6:f860:d6c::3100]:9001",
                b"r nickname aaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbb 2038-01-01 00:00:00 192.168.1.3 9001 0",
            ]).unwrap_err(),
            ControllerError::MalformedReply(_)
        );
    }

    #[test]
    fn fields_not_validated() {
        // Some fiels are not validated on purposes.
        // We might change this in the future, but for now we accept them, so
        // we create these tests just to remind us about this.

        // Digest is in base64. tor's convention is to use unpadded base64, but
        // the specs are not explicit about it (while they are for identity).
        // Let's check we accept a padded base64 for it.
        assert!(addresses_from_status_info(&[
            b"r nickname aaaa bbbb= 2026-11-09 10:34:58 1.2.3.4 7777 8888"
        ])
        .is_ok());

        // No validation on date and time, except for the format.
        assert!(addresses_from_status_info(&[
            b"r nickname aaaa bbbb 2038-25-50 00:00:00 1.2.3.4 7777 8888"
        ])
        .is_ok());
        assert!(addresses_from_status_info(&[
            b"r nickname aaaa bbbb 2000-04-12 32:80:99 1.2.3.4 7777 8888"
        ])
        .is_ok());

        // Ports are not validated either, except for length.
        assert!(addresses_from_status_info(&[
            b"r nickname aaaa bbbb 2000-10-15 12:34:56 1.2.3.4 99999 67890"
        ])
        .is_ok());
    }
}
