/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use nsstring::nsCString;
use serde::Serialize;
use std::net::SocketAddr;
use tor_provider::ctor::*;
use xpcom::{interfaces::torITorReplyCallback, RefPtr};

fn call_xpcom_callback<T>(res: Result<T, ControllerError>, callback: RefPtr<torITorReplyCallback>)
where
    T: Serialize,
{
    // Safety: methods from our own interface that we created so that it was
    // exposed to Rust.
    match res {
        Ok(value) => match serde_json::to_vec(&value) {
            Ok(vec) => unsafe {
                callback.OnReply(&*nsCString::from(vec));
            },
            Err(e) => unsafe {
                callback.OnGenericError(&*nsCString::from(e.to_string()));
            },
        },
        Err(ControllerError::TorError { code, message }) => unsafe {
            callback.OnTorError(code, &*nsCString::from(message));
        },
        Err(e) => unsafe {
            callback.OnGenericError(&*nsCString::from(e.to_string()));
        },
    }
}

pub fn make_xpcom_handler<T>(
    callback: *const torITorReplyCallback,
) -> Box<dyn FnOnce(Result<T, ControllerError>)>
where
    T: Serialize,
{
    // Safety: from_raw will handle the null pointer case for us.
    // It will also increment the reference count, which is what we need, since
    // the XPCOM convention is that pointers are borrowed.
    let callback = unsafe { RefPtr::from_raw(callback) };
    match callback {
        Some(callback) => Box::new(move |r| {
            call_xpcom_callback(r, callback);
        }),
        None => Box::new(move |_| {}),
    }
}

pub fn make_xpcom_handler_mapped<F, T, R>(
    callback: *const torITorReplyCallback,
    map: F,
) -> Box<dyn FnOnce(Result<T, ControllerError>)>
where
    F: FnOnce(T) -> R + 'static,
    R: Serialize,
{
    // Safety: from_raw will handle the null pointer case for us.
    // It will also increment the reference count, which is what we need, since
    // the XPCOM convention is that pointers are borrowed.
    let callback = unsafe { RefPtr::from_raw(callback) };
    match callback {
        Some(callback) => Box::new(move |r| {
            call_xpcom_callback(r.map(map), callback);
        }),
        None => Box::new(move |_| {}),
    }
}

/// A struct that looks like Stream, but uses strings for username and password,
/// to avoid converting between bytes and strings in JavaScript.
#[derive(Serialize)]
pub(crate) struct StreamWithStrings {
    stream_id: StreamID,
    circuit_id: CircuitID,
    socks_username: Option<String>,
    socks_password: Option<String>,
}

impl From<Stream> for StreamWithStrings {
    fn from(value: Stream) -> Self {
        // The browser uses a limited set of ASCII characters, so even if we end
        // up with the replacement character, those streams should be discarded
        // for our purposes anyway.
        let socks_username = value
            .socks_username
            .map(|s| String::from_utf8_lossy(&s).into_owned());
        let socks_password = value
            .socks_password
            .map(|s| String::from_utf8_lossy(&s).into_owned());
        Self {
            stream_id: value.stream_id,
            circuit_id: value.circuit_id,
            socks_username,
            socks_password,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct BridgeWithStrings {
    pub transport: Option<String>,
    pub address: String,
    pub fingerprint: Option<String>,
    pub args: Option<String>,
}

impl From<&Bridge> for BridgeWithStrings {
    fn from(value: &Bridge) -> Self {
        Self {
            transport: value.transport.clone(),
            address: value.address.to_string(),
            fingerprint: value.fingerprint.map(|fp| hex::encode(fp)),
            args: value
                .args
                .as_ref()
                .map(|a| String::from_utf8_lossy(a).into_owned()),
        }
    }
}

/// A flattened version PluggableTransport that is friendlier to serialization.
/// When pt_type is exec, path_to_binary and options are set and ip/port are
/// None, opposite when it is another type.
#[derive(Serialize)]
pub(crate) struct ExpandedPluggableTransport {
    transports: Vec<String>,
    pt_type: String,
    path_to_binary: Option<String>,
    options: Option<String>,
    ip: Option<String>,
    port: Option<u16>,
}

impl ExpandedPluggableTransport {
    fn new_exec(transports: &Vec<String>, path: &Vec<u8>, options: &Option<Vec<u8>>) -> Self {
        Self {
            transports: transports.clone(),
            pt_type: String::from("exec"),
            path_to_binary: Some(String::from_utf8_lossy(path).into_owned()),
            options: options
                .as_ref()
                .map(|o| String::from_utf8_lossy(o).into_owned()),
            ip: None,
            port: None,
        }
    }

    fn new_socks(transports: &Vec<String>, socks_type: &str, addr: &SocketAddr) -> Self {
        let ip = if addr.is_ipv6() {
            format!("[{}]", addr.ip())
        } else {
            addr.ip().to_string()
        };
        Self {
            transports: transports.clone(),
            pt_type: socks_type.to_string(),
            path_to_binary: None,
            options: None,
            ip: Some(ip),
            port: Some(addr.port()),
        }
    }
}

impl From<&PluggableTransport> for ExpandedPluggableTransport {
    fn from(value: &PluggableTransport) -> Self {
        match &value.plugin {
            ClientTransportPlugin::Executable { path, options } => {
                ExpandedPluggableTransport::new_exec(&value.transports, path, options)
            }
            ClientTransportPlugin::Socks4(addr) => {
                ExpandedPluggableTransport::new_socks(&value.transports, "socks4", addr)
            }
            ClientTransportPlugin::Socks5(addr) => {
                ExpandedPluggableTransport::new_socks(&value.transports, "socks5", addr)
            }
        }
    }
}
