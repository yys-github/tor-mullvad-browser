/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use bytes::{Bytes, BytesMut};
use cstr::cstr;
use log::warn;
use nserror::nsresult;
use nserror::{NS_ERROR_FAILURE, NS_ERROR_NOT_AVAILABLE, NS_ERROR_NOT_IMPLEMENTED, NS_OK};
use nsstring::nsACString;
use std::{cell::Cell, ptr::null};
use thin_vec::ThinVec;
use tor_provider::ctor::{ControlSocket, ControlSocketError};
use xpcom::interfaces::{
    nsIAsyncInputStream, nsIAsyncOutputStream, nsIFile, nsISocketTransport,
    nsISocketTransportService, nsITransport,
};
use xpcom::{get_service, getter_addrefs, RefPtr, XpCom};

/// The control socket inner implementation.
/// It uses more of an XPCOM-style for the code.
pub struct ControlSocketXpcom {
    transport: RefPtr<nsISocketTransport>,
    input_stream: RefPtr<nsIAsyncInputStream>,
    output_stream: RefPtr<nsIAsyncOutputStream>,
    closed: Cell<bool>,
}

impl ControlSocketXpcom {
    pub fn new_ipc(socket: &nsIFile) -> Result<Self, nsresult> {
        let sts = get_service::<nsISocketTransportService>(cstr!(
            "@mozilla.org/network/socket-transport-service;1"
        ))
        .ok_or(NS_ERROR_NOT_AVAILABLE)?;
        // Safety: call to an XPCOM method available to Rust bindings.
        let transport = getter_addrefs(|p| unsafe { sts.CreateUnixDomainTransport(socket, p) })?;
        Self::new(transport)
    }

    pub fn new_tcp(host: &nsACString, port: i32) -> Result<Self, nsresult> {
        let sts = get_service::<nsISocketTransportService>(cstr!(
            "@mozilla.org/network/socket-transport-service;1"
        ))
        .ok_or(NS_ERROR_NOT_AVAILABLE)?;
        // Empty array: default socket type (TCP).
        // There is no way to be explicit about TCP.
        let types = ThinVec::new();
        // Safety: call to an XPCOM method available to Rust bindings.
        let transport = getter_addrefs(|p| unsafe {
            sts.CreateTransport(&types, host, port, null(), null(), p)
        })?;
        Self::new(transport)
    }

    fn new(transport: RefPtr<nsISocketTransport>) -> Result<Self, nsresult> {
        // TODO: Maybe set the event sink to tell when we connect in an async
        // way.

        // Safety: call to an XPCOM method available to Rust bindings.
        // Notice that when in non-blocking mode, streams must support async
        // interfaces (as per nsITransport's documentation).
        let input_stream = getter_addrefs(|p| unsafe {
            transport.OpenInputStream(nsITransport::OPEN_UNBUFFERED, 0, 0, p)
        })?
        .query_interface::<nsIAsyncInputStream>()
        .ok_or(NS_ERROR_FAILURE)?;

        // Safety: call to an XPCOM method available to Rust bindings.
        let output_stream = getter_addrefs(|p| unsafe {
            transport.OpenOutputStream(nsITransport::OPEN_UNBUFFERED, 0, 0, p)
        })?
        .query_interface::<nsIAsyncOutputStream>()
        .ok_or(NS_ERROR_FAILURE)?;

        Ok(Self {
            transport,
            input_stream,
            output_stream,
            closed: Cell::new(false),
        })
    }

    /// Map nsresults to the bridge type we use to avoid linking to XPCOM.
    fn map_err(rv: nsresult) -> Result<(), ControlSocketError> {
        if rv.failed() {
            Err(ControlSocketError::ImplementationError(rv.0))
        } else {
            Ok(())
        }
    }
}

impl ControlSocket for ControlSocketXpcom {
    fn queue_read(&self, f: Box<dyn FnOnce()>) -> Result<(), ControlSocketError> {
        Self::map_err(NS_ERROR_NOT_IMPLEMENTED)?;
        Ok(())
    }

    fn available(&self) -> Result<u32, ControlSocketError> {
        Self::map_err(NS_ERROR_NOT_IMPLEMENTED)?;
        Ok(0)
    }

    fn read(&self, max_len: u32) -> Result<Bytes, ControlSocketError> {
        Self::map_err(NS_ERROR_NOT_IMPLEMENTED)?;
        Ok(Bytes::default())
    }

    fn queue_write(&self, f: Box<dyn FnOnce(Result<(), ControlSocketError>)>, len: u32) {}

    fn write(&self, buffer: &Bytes) -> Result<u32, ControlSocketError> {
        Self::map_err(NS_ERROR_NOT_IMPLEMENTED)?;
        Ok(0)
    }

    fn close(&self) -> Result<(), ControlSocketError> {
        if self.closed.replace(true) {
            return Ok(());
        }

        // Safety: call to XPCOM methods available to Rust bindings.
        // We ignore any error when closing the streams and relay only errors we
        // got when closing the transport as we want to be sure we call the
        // Close method in all our objects.
        let rv = unsafe { self.input_stream.Close() };
        if rv.failed() {
            warn!("input_stream.Close() failed: {}", rv.error_name());
        }
        let rv = unsafe { self.output_stream.Close() };
        if rv.failed() {
            warn!("output_stream.Close() failed: {}", rv.error_name());
        }
        Self::map_err(unsafe { self.transport.Close(NS_OK) })
    }
}

impl Drop for ControlSocketXpcom {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            warn!("close failed when called by drop: {}", e.to_string());
        }
    }
}
