// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use lazy_static::lazy_static;
use regex::Regex;

use super::Command;
use crate::ctor::controller::{parsers::*, ConfValue, ControllerError};

lazy_static! {
    // The SETCONF command's documentation lists `keyword` for keys, and
    // keyword is defined as `1*ALPHA`, so it should not include numbers.
    // However, as a matter of fact, several settings (including the ones we
    // use) have numbers, therefore we also accept them.
    static ref KEY_REGEX: Regex =
        Regex::new("^[a-zA-Z0-9_]+$").expect("The regex is hardcoded, it should be good.");
}

pub fn set_conf(values: &[(&str, ConfValue)]) -> Result<Command<u16>, ControllerError> {
    Ok(Command {
        command: make_setconf_command(values)?,
        handler: Box::new(parse_ack),
    })
}

fn make_setconf_command(values: &[(&str, ConfValue)]) -> Result<String, ControllerError> {
    let mut res = String::from("SETCONF");
    res.reserve(256);
    for (key, value) in values.iter() {
        if !KEY_REGEX.is_match(key) {
            return Err(ControllerError::InvalidArgument(format!(
                "invalid key: '{}'",
                key
            )));
        }
        match value {
            ConfValue::Bool(v) => push_kv(key, if *v { "1" } else { "0" }, false, &mut res),
            ConfValue::Int(i) => {
                push_k(key, &mut res);
                res.push('=');
                let mut buf = itoa::Buffer::new();
                res.push_str(buf.format(*i));
            }
            ConfValue::String(s) => push_kv(key, s, true, &mut res),
            ConfValue::Array(a) => {
                if a.is_empty() {
                    push_k(key, &mut res);
                } else {
                    for v in a.iter() {
                        push_kv(key, v, true, &mut res);
                    }
                }
            }
            ConfValue::Null => push_k(key, &mut res),
        }
    }
    res.push_str("\r\n");
    Ok(res)
}

fn push_k(key: &str, dest: &mut String) {
    dest.push(' ');
    dest.push_str(key);
}

fn push_kv(key: &str, val: &str, escape: bool, dest: &mut String) {
    push_k(key, dest);
    dest.push('=');
    if escape {
        tor_escape_into(val, dest);
    } else {
        dest.push_str(val);
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;

    #[test]
    fn test_bool() {
        let vals = &[
            ("test", ConfValue::Bool(true)),
            ("AA", ConfValue::Bool(false)),
        ];
        assert_eq!(
            make_setconf_command(vals).unwrap(),
            "SETCONF test=1 AA=0\r\n"
        );
    }

    #[test]
    fn test_int() {
        let vals = &[
            ("aaa", ConfValue::Int(42)),
            ("bBbB", ConfValue::Int(0)),
            ("cc", ConfValue::Int(-3)),
        ];
        assert_eq!(
            make_setconf_command(vals).unwrap(),
            "SETCONF aaa=42 bBbB=0 cc=-3\r\n"
        );
    }

    #[test]
    fn test_string() {
        let vals = &[("Test123", ConfValue::String("a b\\c"))];
        assert_eq!(
            make_setconf_command(vals).unwrap(),
            "SETCONF Test123=\"a b\\\\c\"\r\n"
        );
    }

    #[test]
    fn test_array() {
        let vals = &[
            ("empty", ConfValue::Array(vec![])),
            ("list", ConfValue::Array(vec!["one", "two two"])),
        ];
        assert_eq!(
            make_setconf_command(vals).unwrap(),
            "SETCONF empty list=\"one\" list=\"two two\"\r\n"
        );
    }

    #[test]
    fn test_null() {
        let vals = &[("test", ConfValue::Null)];
        assert_eq!(make_setconf_command(vals).unwrap(), "SETCONF test\r\n");
    }

    #[test]
    fn mixed_values() {
        let vals = &[
            ("b", ConfValue::Bool(false)),
            ("a", ConfValue::Int(-7)),
            ("c", ConfValue::String("x y")),
            ("d", ConfValue::Null),
        ];
        assert_eq!(
            make_setconf_command(vals).unwrap(),
            "SETCONF b=0 a=-7 c=\"x y\" d\r\n"
        );
    }

    #[test]
    fn repeated_key() {
        let vals = &[
            ("bridge", ConfValue::String("obfs4")),
            ("bridge", ConfValue::String("snowflake")),
            ("bridge", ConfValue::String("meek")),
        ];
        assert_eq!(
            make_setconf_command(vals).unwrap(),
            "SETCONF bridge=\"obfs4\" bridge=\"snowflake\" bridge=\"meek\"\r\n"
        );
    }

    #[test]
    fn realistic_configs() {
        // No bridges
        assert_eq!(
            make_setconf_command(&[
                ("UseBridges", ConfValue::Bool(false)),
                ("Bridge", ConfValue::Null),
            ])
            .unwrap(),
            "SETCONF UseBridges=0 Bridge\r\n"
        );

        // One bridge
        assert_eq!(
            make_setconf_command(&[
                ("UseBridges", ConfValue::Bool(true)),
                ("Bridge", ConfValue::Array(vec!["1.2.3.4:443"]))
            ])
            .unwrap(),
            "SETCONF UseBridges=1 Bridge=\"1.2.3.4:443\"\r\n"
        );

        // Some bridges
        assert_eq!(
            make_setconf_command(&[
                ("UseBridges", ConfValue::Bool(true)),
                (
                    "Bridge",
                    ConfValue::Array(vec![
                        "1.2.3.4:443",
                        "obfs4 5.6.7.8:9999 0123456789012345678901234567890123456789 iat-mode=0",
                    ]),
                ),
            ])
            .unwrap(),
            concat!(
                "SETCONF ",
                "UseBridges=1 ",
                "Bridge=\"1.2.3.4:443\" ",
                "Bridge=\"obfs4 5.6.7.8:9999 0123456789012345678901234567890123456789 iat-mode=0\"",
                "\r\n"
            )
        );

        // No firewall
        assert_eq!(
            make_setconf_command(&[("ReachableAddresses", ConfValue::Null)]).unwrap(),
            "SETCONF ReachableAddresses\r\n"
        );

        // Firewall that only allows 80 and 443
        assert_eq!(
            make_setconf_command(&[("ReachableAddresses", ConfValue::String("*:80,*:443"))])
                .unwrap(),
            "SETCONF ReachableAddresses=\"*:80,*:443\"\r\n"
        );

        // No proxies
        assert_eq!(
            make_setconf_command(&[
                ("Socks4Proxy", ConfValue::Null),
                ("Socks5Proxy", ConfValue::Null),
                ("Socks5ProxyUsername", ConfValue::Null),
                ("Socks5ProxyPassword", ConfValue::Null),
                ("HTTPSProxy", ConfValue::Null),
                ("HTTPSProxyAuthenticator", ConfValue::Null),
            ])
            .unwrap(),
            "SETCONF Socks4Proxy Socks5Proxy Socks5ProxyUsername Socks5ProxyPassword HTTPSProxy HTTPSProxyAuthenticator\r\n"
        );
    }

    #[test]
    fn invalid_key() {
        let val = &[("invalid\r\nkey", ConfValue::Null)];
        assert_matches!(
            make_setconf_command(val).unwrap_err(),
            ControllerError::InvalidArgument(_)
        );
    }
}
