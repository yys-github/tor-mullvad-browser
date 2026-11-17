/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use bytes::Bytes;
use nserror::nsresult;
use nserror::{NS_ERROR_INVALID_ARG, NS_ERROR_NOT_CONNECTED, NS_ERROR_UNEXPECTED, NS_OK};
use nsstring::{nsACString, nsCString};
use serde_json::{json, Value as JsonValue};
use std::rc::Rc;
use thin_vec::ThinVec;
use tor_provider::ctor::*;
use xpcom::interfaces::{nsIFile, torITorNotificationListener, torITorReplyCallback};
use xpcom::RefPtr;

// Actually used, but the compiler does not detect it.
#[allow(unused)]
use xpcom::interfaces::torITorControlPort;

use super::{control_socket::ControlSocketXpcom, plumbing::*};

#[xpcom(implement(torITorControlPort), atomic)]
pub struct ControlPortXpcom {
    control_port: TorController<ControlPort>,
}

impl ControlPortXpcom {
    pub fn new_ipc(socket: &nsIFile) -> Result<RefPtr<Self>, nsresult> {
        Self::new(Box::new(ControlSocketXpcom::new_ipc(socket)?))
    }

    pub fn new_tcp(host: &nsACString, port: i32) -> Result<RefPtr<Self>, nsresult> {
        Self::new(Box::new(ControlSocketXpcom::new_tcp(host, port)?))
    }

    fn new(socket: Box<ControlSocketXpcom>) -> Result<RefPtr<Self>, nsresult> {
        let control_port = TorController::new(ControlPort::new(socket).map_err(Self::map_err)?);
        Ok(Self::allocate(InitControlPortXpcom { control_port }))
    }

    xpcom_method!(close => Close());
    pub fn close(&self) -> Result<(), nsresult> {
        self.control_port.close().map_err(|e| match e {
            ControllerError::ConnectionError(rv) => nsresult(rv),
            _ => NS_ERROR_UNEXPECTED,
        })
    }

    fn map_err(e: ControlSocketError) -> nsresult {
        match e {
            ControlSocketError::ImplementationError(rv) => nsresult(rv),
            ControlSocketError::ConnectionClosed => NS_ERROR_NOT_CONNECTED,
        }
    }

    xpcom_method!(authenticate => Authenticate(password: *const ThinVec<u8>, callback: *const torITorReplyCallback));
    pub fn authenticate(
        &self,
        password: &ThinVec<u8>,
        callback: *const torITorReplyCallback,
    ) -> Result<(), nsresult> {
        self.control_port
            .authenticate(&password, make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(get_bootstrap_phase => GetBootstrapPhase(callback: *const torITorReplyCallback));
    fn get_bootstrap_phase(&self, callback: *const torITorReplyCallback) -> Result<(), nsresult> {
        self.control_port
            .get_bootstrap_phase(make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(get_node_addresses => GetNodeAddresses(node: *const nsACString, callback: *const torITorReplyCallback));
    fn get_node_addresses(
        &self,
        node: &nsACString,
        callback: *const torITorReplyCallback,
    ) -> Result<(), nsresult> {
        let node_str = str::from_utf8(&node).map_err(|_| NS_ERROR_INVALID_ARG)?;
        let mut node = [0; 20];
        hex::decode_to_slice(node_str, &mut node).map_err(|_| NS_ERROR_INVALID_ARG)?;
        self.control_port
            .get_node_addresses(node, make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(get_ip_country => GetIpCountry(ip: *const nsACString, callback: *const torITorReplyCallback));
    fn get_ip_country(
        &self,
        ip: &nsACString,
        callback: *const torITorReplyCallback,
    ) -> Result<(), nsresult> {
        let ip = str::from_utf8(&ip).map_err(|_| NS_ERROR_INVALID_ARG)?;
        let ip = ip.parse().map_err(|_| NS_ERROR_INVALID_ARG)?;
        self.control_port
            .get_ip_country(ip, make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(get_socks_listeners => GetSocksListeners(callback: *const torITorReplyCallback));
    fn get_socks_listeners(&self, callback: *const torITorReplyCallback) -> Result<(), nsresult> {
        let cb = make_xpcom_handler(callback);
        self.control_port.get_socks_listeners(Box::new(|reply| {
            (cb)(reply.map(|listeners| {
                listeners
                    .iter()
                    .map(|l| match l {
                        SocksListener::Tcp(addr) => {
                            json!({ "host": addr.ip().to_string(), "port": addr.port() })
                        }
                        SocksListener::UnixSocket(path) => {
                            json!({ "ipcPath": path })
                        }
                    })
                    .collect::<Vec<_>>()
            }))
        }));
        Ok(())
    }

    xpcom_method!(get_circuits => GetCircuits(callback: *const torITorReplyCallback));
    fn get_circuits(&self, callback: *const torITorReplyCallback) -> Result<(), nsresult> {
        self.control_port.get_circuits(make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(get_bridges => GetBridges(callback: *const torITorReplyCallback));
    pub fn get_bridges(&self, callback: *const torITorReplyCallback) -> Result<(), nsresult> {
        self.control_port.get_bridges(make_xpcom_handler_mapped(
            callback,
            |bridges: Vec<Bridge>| {
                bridges
                    .iter()
                    .map(|b| BridgeWithStrings::from(b))
                    .collect::<Vec<_>>()
            },
        ));
        Ok(())
    }

    xpcom_method!(get_pluggable_transports => GetPluggableTransports(callback: *const torITorReplyCallback));
    pub fn get_pluggable_transports(
        &self,
        callback: *const torITorReplyCallback,
    ) -> Result<(), nsresult> {
        self.control_port
            .get_pluggable_transports(make_xpcom_handler_mapped(
                callback,
                |pts: Vec<PluggableTransport>| {
                    pts.iter()
                        .map(|pt| ExpandedPluggableTransport::from(pt))
                        .collect::<Vec<_>>()
                },
            ));
        Ok(())
    }

    xpcom_method!(set_conf => SetConf(values: *const nsACString, callback: *const torITorReplyCallback));
    fn set_conf(
        &self,
        values: &nsACString,
        callback: *const torITorReplyCallback,
    ) -> Result<(), nsresult> {
        let values: Vec<(String, JsonValue)> =
            serde_json::from_slice(&values).map_err(|_| NS_ERROR_INVALID_ARG)?;
        let mut conf = Vec::with_capacity(values.len());
        for (val, c) in values.iter() {
            let cv = match c {
                JsonValue::Bool(b) => ConfValue::Bool(*b),
                JsonValue::Number(n) => {
                    if let Some(i) = n.as_i64().and_then(|i| i.try_into().ok()) {
                        ConfValue::Int(i)
                    } else {
                        return Err(NS_ERROR_INVALID_ARG);
                    }
                }
                JsonValue::String(s) => ConfValue::String(s.as_str()),
                JsonValue::Array(arr) => {
                    let mut strings = Vec::with_capacity(arr.len());
                    for v in arr.iter() {
                        match v {
                            JsonValue::String(s) => strings.push(s.as_str()),
                            _ => {
                                return Err(NS_ERROR_INVALID_ARG);
                            }
                        }
                    }
                    ConfValue::Array(strings)
                }
                JsonValue::Null => ConfValue::Null,
                _ => return Err(NS_ERROR_INVALID_ARG),
            };
            conf.push((val.as_str(), cv));
        }
        self.control_port
            .set_conf(&conf, make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(set_network_enabled => SetNetworkEnabled(enabled: bool, callback: *const torITorReplyCallback));
    fn set_network_enabled(
        &self,
        enabled: bool,
        callback: *const torITorReplyCallback,
    ) -> Result<(), nsresult> {
        self.control_port.set_conf(
            &[("DisableNetwork", ConfValue::Bool(!enabled))],
            make_xpcom_handler(callback),
        );
        Ok(())
    }

    xpcom_method!(flush_settings => FlushSettings(callback: *const torITorReplyCallback));
    fn flush_settings(&self, callback: *const torITorReplyCallback) -> Result<(), nsresult> {
        self.control_port.save_conf(make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(onion_auth_view_keys => OnionAuthViewKeys(callback: *const torITorReplyCallback));
    fn onion_auth_view_keys(&self, callback: *const torITorReplyCallback) -> Result<(), nsresult> {
        self.control_port
            .onion_auth_view_keys(make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(onion_auth_add => OnionAuthAdd(address: *const nsACString, key: *const nsACString, permanent: bool, callback: *const torITorReplyCallback));
    fn onion_auth_add(
        &self,
        address: &nsACString,
        key: &nsACString,
        permanent: bool,
        callback: *const torITorReplyCallback,
    ) -> Result<(), nsresult> {
        let address = HSAddress::from_slice(&address).map_err(|_| NS_ERROR_INVALID_ARG)?;
        self.control_port
            .onion_auth_add(address, key, permanent, make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(onion_auth_remove => OnionAuthRemove(address: *const nsACString, callback: *const torITorReplyCallback));
    fn onion_auth_remove(
        &self,
        address: &nsACString,
        callback: *const torITorReplyCallback,
    ) -> Result<(), nsresult> {
        let address = HSAddress::from_slice(&address).map_err(|_| NS_ERROR_INVALID_ARG)?;
        self.control_port
            .onion_auth_remove(address, make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(take_ownership => TakeOwnership(callback: *const torITorReplyCallback));
    fn take_ownership(&self, callback: *const torITorReplyCallback) -> Result<(), nsresult> {
        self.control_port
            .take_ownership(make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(reset_owning_controller_process => ResetOwningControllerProcess(callback: *const torITorReplyCallback));
    fn reset_owning_controller_process(
        &self,
        callback: *const torITorReplyCallback,
    ) -> Result<(), nsresult> {
        self.control_port
            .reset_owning_controller_process(make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(newnym => Newnym(callback: *const torITorReplyCallback));
    fn newnym(&self, callback: *const torITorReplyCallback) -> Result<(), nsresult> {
        self.control_port
            .signal_newnym(make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(set_events => SetEvents(events: *const ThinVec<nsCString>, callback: *const torITorReplyCallback));
    fn set_events(
        &self,
        events: &ThinVec<nsCString>,
        callback: *const torITorReplyCallback,
    ) -> Result<(), nsresult> {
        let events = events
            .iter()
            .map(|e| str::from_utf8(&e))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| NS_ERROR_INVALID_ARG)?;
        self.control_port
            .set_events(&events, make_xpcom_handler(callback));
        Ok(())
    }

    xpcom_method!(set_notification_listener => SetNotificationListener(listener: *const torITorNotificationListener));
    fn set_notification_listener(
        &self,
        listener: &torITorNotificationListener,
    ) -> Result<(), nsresult> {
        self.control_port
            .set_notification_listener(Some(Rc::new(XpcomNotificationListener::new(listener))));
        let close_listener = RefPtr::new(listener);
        // For consistency with the JavaScript interface, we still keep the
        // close handler with the notifications.
        self.control_port.set_close_handler(Box::new(move || {
            // Safety: method from our own interface that we created so that it
            // was exposed to Rust.
            unsafe { close_listener.ConnectionClosed() };
        }));
        Ok(())
    }
}

struct XpcomNotificationListener(RefPtr<torITorNotificationListener>);

impl XpcomNotificationListener {
    fn new(listener: &torITorNotificationListener) -> Self {
        Self(RefPtr::new(listener))
    }
}

impl NotificationListener for XpcomNotificationListener {
    fn log_message(&self, severity: String, message: Bytes) {
        // Safety: method from our own interface that we created so that it was
        // exposed to Rust.
        unsafe {
            self.0.LogMessage(
                &*nsCString::from(severity),
                &*nsCString::from(message.as_ref()),
            )
        };
    }

    fn bootstrap_status(&self, status: BootstrapStatus) {
        if let Ok(status) = serde_json::to_vec(&status) {
            // Safety: method from our own interface that we created so that it
            // was exposed to Rust.
            unsafe { self.0.BootstrapStatus(&*nsCString::from(status)) };
        }
    }

    fn circuit_built(&self, circuit: Circuit) {
        // Safety: method from our own interface that we created so that it was
        // exposed to Rust.
        if let Ok(circuit) = serde_json::to_vec(&circuit) {
            unsafe { self.0.CircuitBuilt(&*nsCString::from(circuit)) };
        }
    }

    fn circuit_closed(&self, id: CircuitID) {
        // Safety: method from our own interface that we created so that it was
        // exposed to Rust.
        unsafe { self.0.CircuitClosed(&*nsCString::from(id.as_str())) };
    }

    fn stream_sent_connect(&self, stream: Stream) {
        let stream = StreamWithStrings::from(stream);
        if let Ok(stream) = serde_json::to_vec(&stream) {
            // Safety: method from our own interface that we created so that it
            // was exposed to Rust.
            unsafe { self.0.StreamSentConnect(&*nsCString::from(stream)) };
        }
    }
}
