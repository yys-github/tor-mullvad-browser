// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use serde::Serialize;
use std::net::SocketAddr;

/// Information extracted from a bridge line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bridge {
    pub transport: Option<String>,
    pub address: SocketAddr,
    pub fingerprint: Option<[u8; 20]>,
    pub args: Option<Vec<u8>>,
}

/// The definition of a handler for a pluggable transport.
/// The conversion between 8-bit characters and 16-bit characters on Windows is
/// not well defined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ClientTransportPlugin {
    Executable {
        path: Vec<u8>,
        options: Option<Vec<u8>>,
    },
    Socks4(SocketAddr),
    Socks5(SocketAddr),
}

/// A configuration value to be used with the SETCONF command.
/// Boolean will be converted to 1 and 0.
/// Null can be used to reset a certain value. When building the SETCONF
/// command, we will add `<key>`, rather than `<key>=<value>` or `<key>=`.
#[derive(Debug, Clone)]
pub enum ConfValue<'a> {
    Bool(bool),
    Int(i32),
    String(&'a str),
    Array(Vec<&'a str>),
    Null,
}

/// A pluggable transport, with its handler and the transport it can manages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluggableTransport {
    pub transports: Vec<String>,
    pub plugin: ClientTransportPlugin,
}
