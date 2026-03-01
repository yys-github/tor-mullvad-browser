// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use lazy_static::lazy_static;
use regex::bytes::{Captures, Regex};

use super::tor_unescape;
use crate::ctor::controller::{ClientTransportPlugin, ControllerError, PluggableTransport};

lazy_static! {
    // man 1 tor: ClientTransportPlugin transport socks4|socks5 IP:PORT
    static ref SOCKS_REGEX: Regex =
        Regex::new(r"^(?<transport>[A-Za-z_][A-Za-z0-9_,]*) (?<protocol>socks[45]) (?<address>(?:[\d\.]{7,15}|\[[\da-fA-F:]+\]):\d{1,5})$")
            .expect("The regex is hardcoded, it should be good.");
    // man 1 tor: transport exec path-to-binary [options]
    static ref EXEC_REGEX: Regex = Regex::new(r#"^(?<transport>[A-Za-z_][A-Za-z0-9_,]*) exec (?<path>"(?:[^"\\]|\\.)*"|[^ ]+)(?: (?<options>.*?) *)?$"#)
        .expect("The regex is hardcoded, it should be good.");
}

pub fn parse_client_transport_plugin(line: &[u8]) -> Result<PluggableTransport, ControllerError> {
    if let Some(socks_line) = SOCKS_REGEX.captures(line) {
        parse_socks(&socks_line)
    } else if let Some(exec_line) = EXEC_REGEX.captures(line) {
        parse_exec(&exec_line)
    } else {
        Err(ControllerError::WrongFormat(
            String::from_utf8_lossy(line).into_owned(),
        ))
    }
}

fn get_transports(c: &Captures) -> Result<Vec<String>, ControllerError> {
    let transports = c
        .name("transport")
        .ok_or(ControllerError::KeyNotFound(String::from("transport")))?
        .as_bytes()
        .split(|b| *b == b',')
        .map(|t| String::from_utf8_lossy(t).into_owned())
        .collect::<Vec<_>>();
    if transports.is_empty() || transports.iter().any(|t| t.is_empty()) {
        return Err(ControllerError::MalformedReply(String::from(
            "empty transports",
        )));
    }
    Ok(transports)
}

fn parse_socks(socks_line: &Captures) -> Result<PluggableTransport, ControllerError> {
    let address = str::from_utf8(
        socks_line
            .name("address")
            .ok_or(ControllerError::KeyNotFound(String::from("address")))?
            .as_bytes(),
    )
    .expect("The regex matches only ASCII characters (hence valid UTF-8)")
    .parse()
    .map_err(|e| ControllerError::MalformedReply(format!("{}", e)))?;
    let plugin = match socks_line
        .name("protocol")
        .ok_or(ControllerError::KeyNotFound(String::from("protocol")))?
        .as_bytes()
    {
        b"socks4" => ClientTransportPlugin::Socks4(address),
        b"socks5" => ClientTransportPlugin::Socks5(address),
        _ => unreachable!("We validated this with the regex"),
    };
    Ok(PluggableTransport {
        transports: get_transports(&socks_line)?,
        plugin,
    })
}

fn parse_exec(exec_line: &Captures) -> Result<PluggableTransport, ControllerError> {
    let options = exec_line
        .name("options")
        .map(|o| o.as_bytes())
        .and_then(|s| if s.is_empty() { None } else { Some(s.into()) });
    let path = tor_unescape(
        exec_line
            .name("path")
            .ok_or(ControllerError::KeyNotFound(String::from("path")))?
            .as_bytes(),
    )
    .map_err(|e| ControllerError::MalformedReply(e.to_string()))?
    .into_owned();
    if path.is_empty() {
        return Err(ControllerError::MalformedReply(String::from("empty path")));
    }
    Ok(PluggableTransport {
        transports: get_transports(&exec_line)?,
        plugin: ClientTransportPlugin::Executable { path, options },
    })
}

#[cfg(test)]
mod tests {
    use std::{
        assert_matches,
        net::{Ipv4Addr, SocketAddrV4},
    };

    use super::*;

    #[test]
    fn socks() {
        {
            let pt =
                parse_client_transport_plugin(b"mytransport,mypt2 socks4 127.0.0.1:9052").unwrap();
            assert_eq!(pt.transports, vec!["mytransport", "mypt2"]);
            assert_eq!(
                pt.plugin,
                ClientTransportPlugin::Socks4(
                    SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9052).into()
                )
            );
        }
        {
            let pt = parse_client_transport_plugin(b"pt3,pt_4 socks5 10.0.1.2:9052").unwrap();
            assert_eq!(pt.transports, vec!["pt3", "pt_4"]);
            assert_eq!(
                pt.plugin,
                ClientTransportPlugin::Socks5(
                    SocketAddrV4::new(Ipv4Addr::new(10, 0, 1, 2), 9052).into()
                )
            );
        }
        {
            let pt = parse_client_transport_plugin(
                b"mytransport,mypt2 socks4 [38e5:09fc:2080:673d:9ab8:4545:08db:36c2]:1234",
            )
            .unwrap();
            assert_eq!(pt.transports, vec!["mytransport", "mypt2"]);
            assert_eq!(
                pt.plugin,
                ClientTransportPlugin::Socks4(
                    "[38e5:09fc:2080:673d:9ab8:4545:08db:36c2]:1234"
                        .parse()
                        .unwrap()
                )
            );
        }
        {
            let pt = parse_client_transport_plugin(
                b"s5pt socks5 [dad7:ddf4:7fc4:ef92:4bb2:ceb9:05b9:9161]:5555",
            )
            .unwrap();
            assert_eq!(pt.transports, vec!["s5pt"]);
            assert_eq!(
                pt.plugin,
                ClientTransportPlugin::Socks5(
                    "[dad7:ddf4:7fc4:ef92:4bb2:ceb9:05b9:9161]:5555"
                        .parse()
                        .unwrap()
                )
            );
        }
    }

    #[test]
    fn exec_multiple_transports() {
        let pt = parse_client_transport_plugin(b"obfs4,meek exec lyrebird").unwrap();
        assert_eq!(
            pt.transports,
            vec![String::from("obfs4"), String::from("meek")]
        );
        assert_eq!(
            pt.plugin,
            ClientTransportPlugin::Executable {
                path: Vec::from(b"lyrebird"),
                options: None,
            }
        );
    }

    #[test]
    fn exec_with_spaces() {
        let pt =
            parse_client_transport_plugin(b"snowflake exec \"pluggable transports/snowflake\"")
                .unwrap();
        assert_eq!(pt.transports, vec![String::from("snowflake")]);
        assert_eq!(
            pt.plugin,
            ClientTransportPlugin::Executable {
                path: Vec::from("pluggable transports/snowflake"),
                options: None,
            }
        )
    }

    #[test]
    fn exec_windows_path() {
        let pt = parse_client_transport_plugin(
            b"snowflake exec \"C:\\\\Program Files\\\\Tor Project \\\\snowflake.exe\"",
        )
        .unwrap();
        assert_eq!(pt.transports, vec![String::from("snowflake")]);
        assert_eq!(
            pt.plugin,
            ClientTransportPlugin::Executable {
                path: Vec::from("C:\\Program Files\\Tor Project \\snowflake.exe"),
                options: None,
            }
        )
    }

    #[test]
    fn exec_with_options() {
        let pt = parse_client_transport_plugin(
            b"conjure,coupdetat exec conjure-client -registerURL https://registration.refraction.network/api     ",
        )
        .unwrap();
        assert_eq!(
            pt.transports,
            vec![String::from("conjure"), String::from("coupdetat")]
        );
        assert_eq!(
            pt.plugin,
            ClientTransportPlugin::Executable {
                path: Vec::from("conjure-client"),
                options: Some(b"-registerURL https://registration.refraction.network/api".into())
            }
        )
    }

    #[test]
    fn invalid() {
        assert_matches!(
            parse_client_transport_plugin(b"transport socks2 127.0.0.1:9052").unwrap_err(),
            ControllerError::WrongFormat(_)
        );
    }

    #[test]
    fn socks_errors() {
        assert_matches!(
            parse_client_transport_plugin(b"transport socks4").unwrap_err(),
            ControllerError::WrongFormat(_)
        );

        assert_matches!(
            parse_client_transport_plugin(b"transport socks4 123.45.6.7.8:123456").unwrap_err(),
            ControllerError::WrongFormat(_)
        );

        assert_matches!(
            parse_client_transport_plugin(b", socks4 123.45.6.7.8:1234").unwrap_err(),
            ControllerError::WrongFormat(_)
        );

        assert_matches!(
            parse_client_transport_plugin(b"transport socks4 1234.5.6.7.8:123").unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            parse_client_transport_plugin(b"transport socks4 273.12.34.56:789").unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            parse_client_transport_plugin(b"transport socks4 123.12.34.56:78999").unwrap_err(),
            ControllerError::MalformedReply(_)
        );
    }

    #[test]
    fn exec_errors() {
        assert_matches!(
            parse_client_transport_plugin(b"transport exec").unwrap_err(),
            ControllerError::WrongFormat(_)
        );

        assert_matches!(
            parse_client_transport_plugin(b"transport exec \"\"").unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            parse_client_transport_plugin(b"transport exec \"unterminated").unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            parse_client_transport_plugin(b"transport exec \"invalid hex \\xAz\"").unwrap_err(),
            ControllerError::MalformedReply(_)
        );

        assert_matches!(
            parse_client_transport_plugin(b", exec mypt.exe").unwrap_err(),
            ControllerError::WrongFormat(_)
        );
        assert_matches!(
            parse_client_transport_plugin(b"garbage trans,port exec mypt.exe").unwrap_err(),
            ControllerError::WrongFormat(_)
        );
    }
}
