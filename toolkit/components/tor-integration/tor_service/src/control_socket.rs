/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use bytes::{Bytes, BytesMut};
use cstr::cstr;
use log::warn;
use nserror::nsresult;
use nserror::{
    NS_ERROR_FAILURE, NS_ERROR_INVALID_ARG, NS_ERROR_NOT_AVAILABLE, NS_ERROR_NOT_IMPLEMENTED, NS_OK,
};
use nsstring::nsACString;
use std::{cell::Cell, ffi::c_char, ptr::null};
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
        let mut available: u64 = 0;
        // Safety: call to an XPCOM method exposed to Rust.
        // Also, we pass a pointer, but it is to our stack, so it is valid.
        Self::map_err(unsafe { self.input_stream.Available(&mut available) })?;
        Ok(available.min(u32::MAX as u64) as u32)
    }

    fn read(&self, max_len: u32) -> Result<Bytes, ControlSocketError> {
        if max_len == 0 {
            Self::map_err(NS_ERROR_INVALID_ARG)?;
        }
        let mut buffer = BytesMut::with_capacity(max_len as usize);
        let dest = buffer.as_mut_ptr() as *mut c_char;
        let mut read = 0;
        // Safety: we call an XPCOM method available to Rust.
        // We pass a valid pointer, and we constructed max_read so that it does
        // not go beyond the buffer's capacity.
        Self::map_err(unsafe { self.input_stream.Read(dest, max_len, &mut read) })?;
        if read > 0 {
            let new_len = buffer.len() + read as usize;
            debug_assert!(new_len <= buffer.capacity());
            // Safety: we constructed our read limit to read at most a number of
            // bytes that filled the buffer.
            unsafe { buffer.set_len(new_len) };
        }
        Ok(buffer.freeze())
    }

    fn queue_write(&self, f: Box<dyn FnOnce(Result<(), ControlSocketError>)>, len: u32) {}

    fn write(&self, buffer: &Bytes) -> Result<u32, ControlSocketError> {
        // Why would a caller try to write an empty buffer?
        if buffer.is_empty() {
            Self::map_err(NS_ERROR_INVALID_ARG)?;
        }

        let src = buffer.as_ptr() as *const c_char;
        let to_write = buffer.len().min(u32::MAX as usize) as u32;
        let mut wrote = 0;
        // Safety: we call an XPCOM method available to Rust.
        // The pointers we pass are valid, as they are guaranteed not to be null
        // in the case of the bytes object, and we pass a variable from our
        // stack.
        Self::map_err(unsafe { self.output_stream.Write(src, to_write, &mut wrote) })?;
        Ok(wrote)
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
