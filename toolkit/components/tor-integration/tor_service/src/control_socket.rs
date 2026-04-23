/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use bytes::{Bytes, BytesMut};
use cstr::cstr;
use log::warn;
use moz_task::get_current_thread;
use nserror::nsresult;
use nserror::{
    NS_ERROR_FAILURE, NS_ERROR_INVALID_ARG, NS_ERROR_NOT_AVAILABLE, NS_ERROR_UNEXPECTED, NS_OK,
};
use nsstring::nsACString;
use std::{cell::{Cell, RefCell}, ffi::c_char, ptr::null};
use thin_vec::ThinVec;
use tor_provider::ctor::{ControlSocket, ControlSocketError};
use xpcom::interfaces::{
    nsIAsyncInputStream, nsIAsyncOutputStream, nsIEventTarget, nsIFile, nsIInputStreamCallback,
    nsIOutputStreamCallback, nsISocketTransport, nsISocketTransportService, nsITransport,
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
        let current_thread =
            get_current_thread().map_err(|rv| ControlSocketError::ImplementationError(rv.0))?;
        // Safety: we call an XPCOM method available to Rust.
        // We pass a couple of parameters as raw pointers, but AsyncWait will
        // increase reference as it needs.
        // Finally, we pass this thread, to do everything on this thread, as the
        // dispatcher is not thread-safe.
        Self::map_err(unsafe {
            self.input_stream.AsyncWait(
                ReadCallback::new(f).coerce::<nsIInputStreamCallback>(),
                0,
                0,
                &*current_thread.coerce::<nsIEventTarget>(),
            )
        })
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

    fn queue_write(&self, f: Box<dyn FnOnce(Result<(), ControlSocketError>)>, len: u32) {
        let current_thread = match get_current_thread() {
            Ok(th) => th,
            Err(rv) => {
                f(Err(ControlSocketError::ImplementationError(rv.0)));
                return;
            }
        };
        let callback = WriteCallback::new(f);
        // Safety: we call an XPCOM method available to Rust.
        // We pass a couple of parameters as raw pointers, but AsyncWait will
        // increase reference as it needs.
        let rv = unsafe {
            self.output_stream.AsyncWait(
                callback.coerce::<nsIOutputStreamCallback>(),
                0,
                len,
                &*current_thread.coerce::<nsIEventTarget>(),
            )
        };
        if rv.failed() {
            let _ = callback.call(Err(ControlSocketError::ImplementationError(rv.0)));
        }
    }

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

#[xpcom(implement(nsIInputStreamCallback), atomic)]
struct ReadCallback {
    // We expect the callback to be called once, but we still need to make the
    // borrow checker happy. At the moment, we run in single-threaded, so we can
    // use a RefCell.
    // WriteCallback is modeled in the same way.
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl ReadCallback {
    pub fn new(callback: Box<dyn FnOnce()>) -> RefPtr<Self> {
        Self::allocate(InitReadCallback {
            callback: RefCell::new(Some(callback)),
        })
    }

    xpcom_method!(on_input_stream_ready => OnInputStreamReady(stream: *const nsIAsyncInputStream));
    fn on_input_stream_ready(&self, _stream: &nsIAsyncInputStream) -> Result<(), nsresult> {
        let callback = self.callback.borrow_mut().take();
        if let Some(f) = callback {
            // Even though we can return a nsresult, it is ignored by our caller.
            (f)();
        }
        Ok(())
    }
}

#[xpcom(implement(nsIOutputStreamCallback), atomic)]
struct WriteCallback {
    // Same as ReadCallback.
    callback: RefCell<Option<Box<dyn FnOnce(Result<(), ControlSocketError>)>>>,
}

impl WriteCallback {
    pub fn new(callback: Box<dyn FnOnce(Result<(), ControlSocketError>)>) -> RefPtr<Self> {
        Self::allocate(InitWriteCallback {
            callback: RefCell::new(Some(callback)),
        })
    }

    xpcom_method!(on_output_stream_ready => OnOutputStreamReady(stream: *const nsIAsyncOutputStream));
    fn on_output_stream_ready(&self, _stream: &nsIAsyncOutputStream) -> Result<(), nsresult> {
        self.call(Ok(()))
    }

    pub fn call(&self, value: Result<(), ControlSocketError>) -> Result<(), nsresult> {
        let callback = self.callback.borrow_mut().take();
        match callback {
            Some(f) => {
                f(value);
                Ok(())
            }
            None => {
                debug_assert!(false, "The callback was called more than once!");
                Err(NS_ERROR_UNEXPECTED)
            }
        }
    }
}
