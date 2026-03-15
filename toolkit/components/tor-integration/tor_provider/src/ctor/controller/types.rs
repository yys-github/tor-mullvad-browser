// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use serde::Serialize;
use smallvec::SmallVec;
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    path::PathBuf,
};
use tinystr::TinyAsciiStr;

/// The severity of STATUS_CLIENT updates. It is the first token:
///
/// NOTICE BOOTSTRAP PROGRESS=n ...
/// WARN BOOTSTRAP PROGRESS=n ...
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StatusSeverity {
    Notice,
    Warn,
}

impl ToString for StatusSeverity {
    fn to_string(&self) -> String {
        match self {
            StatusSeverity::Notice => String::from("NOTICE"),
            StatusSeverity::Warn => String::from("WARN"),
        }
    }
}

/// Data structure with the information associated to a STATUS_CLIENT event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BootstrapStatus {
    pub severity: StatusSeverity,
    pub progress: u8,
    pub tag: String,
    pub summary: String,
    pub reason: Option<String>,
    pub should_warn: bool,
}

/// Information extracted from a bridge line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bridge {
    pub transport: Option<String>,
    pub address: SocketAddr,
    pub fingerprint: Option<[u8; 20]>,
    pub args: Option<Vec<u8>>,
}

/// A circuit ID. It can be an alphanumeric string up to 16 characters.
/// At the moment, comparison between them is case-sensitive, but tor uses only
/// digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct CircuitID(TinyAsciiStr<16>);

impl CircuitID {
    pub fn new(data: &[u8]) -> Option<Self> {
        if data.is_empty() || data.iter().any(|c| !c.is_ascii_alphanumeric()) {
            return None;
        }
        let s = str::from_utf8(data).ok()?.parse().ok()?;
        Some(Self(s))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// A conflux cookie, used to group conflux circuits.
pub type ConfluxID = [u8; 16];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Circuit {
    pub id: CircuitID,
    pub path: SmallVec<[NodeFingerprint; 3]>,
    pub conflux_id: Option<ConfluxID>,
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

/// The information about the IP addresses of a relay.
/// At the moment, all realys must have an IPv4 address, and tor will use at
/// most one IPv6 address (which is optional).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NodeAddresses {
    pub ipv4: Ipv4Addr,
    pub ipv6: Option<Ipv6Addr>,
}

/// The fingerprint of a node (as bytes, i.e., decoded from the 40 hex digits
/// format).
pub type NodeFingerprint = [u8; 20];

/// A pluggable transport, with its handler and the transport it can manages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluggableTransport {
    pub transports: Vec<String>,
    pub plugin: ClientTransportPlugin,
}

/// A listener for the SOCKS port.
/// On Windows, only TCP listeners are supported.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub enum SocksListener {
    Tcp(SocketAddr),
    UnixSocket(PathBuf),
}

/// The bare minimum information we need about a stream to associate some
/// isolation credentials to a circuit.
///
/// Notice: since tor uses SOCKS passwords as isolation parameters rather than
/// as real passwords, we do not redact them, and we use the default Debug
/// implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Stream {
    pub stream_id: StreamID,
    pub circuit_id: CircuitID,
    pub socks_username: Option<Vec<u8>>,
    pub socks_password: Option<Vec<u8>>,
}

/// A stream ID. It can be an alphanumeric string up to 16 characters.
/// At the moment, comparison between them is case-sensitive, but tor uses only
/// digits.
/// Circuit IDs are defined in the same way, but we do not define them as alias
/// to prevent exchanging them by mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct StreamID(TinyAsciiStr<16>);

impl StreamID {
    pub fn new(data: &[u8]) -> Option<Self> {
        if data.is_empty() || data.iter().any(|c| !c.is_ascii_alphanumeric()) {
            return None;
        }
        let s = str::from_utf8(data).ok()?.parse().ok()?;
        Some(Self(s))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
