// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use data_encoding::{DecodeError, BASE32_NOPAD};
use serde::{Serialize, Serializer};
use sha3::{Digest, Sha3_256};
use smallvec::SmallVec;
use std::{
    fmt,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    path::PathBuf,
};
use thiserror::Error;
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

/// Onion service address (in the canonical base32 representation, without the
/// .onion suffix).
/// The address is validated on construction, to make sure it adheres to the
/// spec: https://spec.torproject.org/rend-spec/encoding-onion-addresses.html.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HSAddress([u8; 56]);

impl HSAddress {
    pub fn new(base32_address: &[u8; 56]) -> Result<Self, OnionValidationError> {
        validate_onion_address(base32_address).map(Self)
    }

    pub fn from_slice(base32_address: &[u8]) -> Result<Self, OnionValidationError> {
        Self::new(
            base32_address
                .try_into()
                .map_err(|_| OnionValidationError::InvalidLength)?,
        )
    }

    pub fn from_str(base32_address: &str) -> Result<Self, OnionValidationError> {
        Self::from_slice(base32_address.as_bytes())
    }
}

impl AsRef<str> for HSAddress {
    fn as_ref(&self) -> &str {
        str::from_utf8(&self.0).expect(
            "We validated the address on the constructor, therefore it must be valid UTF-8.",
        )
    }
}

impl Serialize for HSAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}

/// Possible errors that can happen when validating an onion address.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum OnionValidationError {
    #[error("Invalid length")]
    InvalidLength,
    #[error("cannot decode the address")]
    CannotDecode(#[from] DecodeError),
    #[error("unsupported version: {0}")]
    UnsupportedVersion(u8),
    #[error("bad checksum")]
    BadChecksum,
}

/// Check the validity of an onion address.
///
/// https://spec.torproject.org/rend-spec/encoding-onion-addresses.html
fn validate_onion_address(base32_address: &[u8; 56]) -> Result<[u8; 56], OnionValidationError> {
    let mut uppercased = *base32_address;
    uppercased.make_ascii_uppercase();
    let decoded = BASE32_NOPAD
        .decode(&uppercased)
        .map_err(|e| OnionValidationError::from(e))?;
    debug_assert_eq!(
        decoded.len(),
        35,
        "We are using fixed size, how can the length be wrong?!"
    );

    let pubkey = &decoded[0..32];
    let stored_checksum = &decoded[32..34];
    let version = decoded[34];

    if version != 3 {
        return Err(OnionValidationError::UnsupportedVersion(version));
    }

    let hash = {
        let mut hasher = Sha3_256::new();
        hasher.update(b".onion checksum");
        hasher.update(pubkey);
        hasher.update(&[version]);
        hasher.finalize()
    };

    let calculated_checksum = &hash[0..2];
    if stored_checksum != calculated_checksum {
        return Err(OnionValidationError::BadChecksum);
    }

    Ok(uppercased)
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

/// A private key for onion authentication.
/// The key_blob is a base64 string, rather than a binary blob.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct OnionClientAuth {
    pub address: HSAddress,
    pub key_type: String,
    pub key_blob: String,
    pub is_permanent: bool,
}

impl fmt::Debug for OnionClientAuth {
    /// Explicit implementation of fmt::Debug for OnionClientAuth that redacts
    /// the private key.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnionClientAuth")
            .field("address", &self.address)
            .field("key_type", &self.key_type)
            .field("key_blob", &"[redacted]")
            .field("is_permanent", &self.is_permanent)
            .finish()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onion_validation() {
        assert!(validate_onion_address(
            b"PG6MMJIYJMCRSSLVYKFWNNTLARU7P5SVN6Y2YMMJU6NUBXNDF4PSCRYD"
        )
        .is_ok());
        assert!(validate_onion_address(
            b"pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd"
        )
        .is_ok());
        assert!(validate_onion_address(
            b"pG6mMjIyJmCrSsLvYkFwNnTlArU7p5SvN6y2YmMjU6nuBxNdF4pScRyD"
        )
        .is_ok());

        assert!(matches!(
            validate_onion_address(b"+-6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd")
                .unwrap_err(),
            OnionValidationError::CannotDecode(_)
        ));

        assert_eq!(
            validate_onion_address(b"pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscrye")
                .unwrap_err(),
            OnionValidationError::UnsupportedVersion(4)
        );

        assert_eq!(
            validate_onion_address(b"pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscsyd")
                .unwrap_err(),
            OnionValidationError::BadChecksum
        );
        assert_eq!(
            validate_onion_address(b"aaammjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscsyd")
                .unwrap_err(),
            OnionValidationError::BadChecksum
        );
    }

    #[test]
    fn onion_private_key_redacted() {
        let auth = OnionClientAuth {
            address: HSAddress::from_str(
                "PG6MMJIYJMCRSSLVYKFWNNTLARU7P5SVN6Y2YMMJU6NUBXNDF4PSCRYD",
            )
            .unwrap(),
            key_type: String::from("x25519"),
            key_blob: String::from("abase64key"),
            is_permanent: false,
        };
        let auth_fmt = format!("{:?}", auth);
        assert!(!auth_fmt.contains(&auth.key_blob));
        assert!(auth_fmt.contains("[redacted]"));
    }
}
