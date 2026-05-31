// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use lazy_static::lazy_static;
use regex::bytes::Regex;

use super::Command;
use crate::ctor::controller::{parsers::parse_ack, types::*, ControllerError};

lazy_static! {
    // This is not a strict base64 regex, but we are intrested only in making
    // sure user input does not allow command injection.
    static ref BASE64ISH_REGEX: Regex =
        Regex::new(r"^[A-Za-z0-9+/]*={0,3}$").expect("The regex is hardcoded, it should be good.");
}

pub fn onion_auth_add(
    address: HSAddress,
    key: &[u8],
    permanent: bool,
) -> Result<Command<u16>, ControllerError> {
    if !BASE64ISH_REGEX.is_match(key) {
        return Err(ControllerError::InvalidArgument(String::from(
            "the key is not in base64",
        )));
    }
    let key = str::from_utf8(key).map_err(|e| ControllerError::InvalidArgument(e.to_string()))?;
    let flags = if permanent { " Flags=Permanent" } else { "" };
    let command = format!(
        "ONION_CLIENT_AUTH_ADD {} x25519:{}{}\r\n",
        address.as_ref(),
        key,
        flags
    );
    Ok(Command {
        command,
        handler: Box::new(parse_ack),
    })
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;

    #[test]
    fn permanent() {
        let command = onion_auth_add(
            HSAddress::from_str("pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd")
                .unwrap(),
            b"theKEYinbase64",
            true,
        )
        .unwrap();
        assert!(command.command.to_lowercase().contains("flags=permanent"));
    }

    #[test]
    fn not_permanent() {
        let command = onion_auth_add(
            HSAddress::from_str("pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd")
                .unwrap(),
            b"theKEYinbase64",
            false,
        )
        .unwrap();
        assert!(!command.command.to_lowercase().contains("permanent"));
    }

    #[test]
    fn pad_and_unpadded() {
        // Tor is liberal about the necessity of padding, so make sure that
        // padded strings are accepted by us even when not padded.
        assert!(onion_auth_add(
            HSAddress::from_str("pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd")
                .unwrap(),
            b"SGVsbG8sIHdvcmxkIQo=",
            false,
        )
        .is_ok());
            assert!(onion_auth_add(
            HSAddress::from_str("pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd")
                .unwrap(),
            b"SGVsbG8sIHdvcmxkIQo",
            false,
        )
        .is_ok());
    }

    #[test]
    fn reject_invalid_base64() {
        assert_matches!(
            onion_auth_add(
                HSAddress::from_str("pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd")
                    .unwrap(),
                b"NotBase64!!!",
                true,
            )
            .unwrap_err(),
            ControllerError::InvalidArgument(_)
        );
    }
}
