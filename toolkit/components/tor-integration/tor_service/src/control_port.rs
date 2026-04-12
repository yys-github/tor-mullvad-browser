/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use bytes::Bytes;
use nserror::nsresult;
use nserror::{NS_ERROR_NOT_CONNECTED, NS_OK};
use nsstring::{nsACString, nsCString};
use tor_provider::ctor::ControlPort;
use tor_provider::ctor::ControlSocketError;
use xpcom::interfaces::{nsIFile, ITorControlPort, ITorControlPortReceiver, ITorMessageHandler};
use xpcom::RefPtr;

use super::control_socket::ControlSocketXpcom;

#[xpcom(implement(ITorControlPort), atomic)]
pub struct ControlPortXpcom {
    control_port: ControlPort,
}

impl ControlPortXpcom {
    pub fn new_ipc(socket: &nsIFile) -> Result<RefPtr<Self>, nsresult> {
        Self::new(Box::new(ControlSocketXpcom::new_ipc(socket)?))
    }

    pub fn new_tcp(host: &nsACString, port: i32) -> Result<RefPtr<Self>, nsresult> {
        Self::new(Box::new(ControlSocketXpcom::new_tcp(host, port)?))
    }

    fn new(socket: Box<ControlSocketXpcom>) -> Result<RefPtr<Self>, nsresult> {
        let control_port = ControlPort::new(socket).map_err(Self::map_err)?;
        Ok(Self::allocate(InitControlPortXpcom { control_port }))
    }

    xpcom_method!(start => Start(receiver: *const ITorControlPortReceiver));
    pub fn start(&self, receiver: &ITorControlPortReceiver) -> Result<(), nsresult> {
        let receiver = RefPtr::new(receiver);
        self.control_port.set_async_handler(Box::new(move |reply| {
            let mut buf = Vec::new();
            if let Err(e) = reply.write_to(&mut buf) {
                log::error!(
                    "Cannot convert the reply to the raw message: {}.",
                    e.to_string()
                );
                return;
            }
            if buf.ends_with(b"\r\n") {
                buf.truncate(buf.len() - 2);
            }
            // These conversions re-use the buffer, Gecko uses the same
            // allocator for Rust and C++ (see the nsstring crate).
            let as_str = nsCString::from(buf);
            // Safety: call to an XPCOM method that we expect to be exposed on
            // Rust. As per the documentation in nsstring, it is safe to pass
            // nsCStrings created in Rust to C++.
            unsafe { receiver.OnAsyncMessage(&*as_str) };
        }));
        Ok(())
    }

    xpcom_method!(send_command => SendCommand(command: *const nsACString, handler: *const ITorMessageHandler));
    pub fn send_command(
        &self,
        command: &nsACString,
        handler: &ITorMessageHandler,
    ) -> Result<(), nsresult> {
        let command = Bytes::copy_from_slice(&command[..]);
        let handler = RefPtr::new(handler);
        self.control_port.send_command(
            command,
            Box::new(move |reply| {
                let mut buf = Vec::new();
                let reply = match reply {
                    Ok(r) => r,
                    Err(e) => {
                        let as_str = nsCString::from(e.to_string());
                        // Safety: see the async notification handler.
                        unsafe { handler.OnError(&*as_str) };
                        return;
                    }
                };
                if let Err(e) = reply.write_to(&mut buf) {
                    let as_str = nsCString::from(e.to_string());
                    // Safety: see the async notification handler.
                    unsafe { handler.OnError(&*as_str) };
                    return;
                }
                if buf.ends_with(b"\r\n") {
                    buf.truncate(buf.len() - 2);
                }
                let as_str = nsCString::from(buf);
                // Safety: see the async notification handler.
                unsafe { handler.OnMessage(&*as_str) };
            }),
        );
        Ok(())
    }

    xpcom_method!(close => Close());
    pub fn close(&self) -> Result<(), nsresult> {
        self.control_port.close().map_err(Self::map_err)
    }

    fn map_err(e: ControlSocketError) -> nsresult {
        match e {
            ControlSocketError::ImplementationError(rv) => nsresult(rv),
            ControlSocketError::ConnectionClosed => NS_ERROR_NOT_CONNECTED,
        }
    }
}
