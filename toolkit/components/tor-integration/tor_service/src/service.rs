/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use nserror::{nsresult, NS_ERROR_NULL_POINTER, NS_OK};
use nsstring::nsACString;
use xpcom::interfaces::{nsIFile, torITorControlPort};
use xpcom::RefPtr;

use super::control_port::ControlPortXpcom;

// Actually used, but the compiler does not detect it.
#[allow(unused)]
use xpcom::interfaces::torITorService;

#[xpcom(implement(torITorService), atomic)]
struct TorService {}

impl TorService {
    fn new() -> RefPtr<TorService> {
        TorService::allocate(InitTorService {})
    }

    xpcom_method!(create_control_port => CreateControlPort(host: *const nsACString, port: i32, out: *mut *const torITorControlPort));
    fn create_control_port(
        &self,
        host: &nsACString,
        port: i32,
        out: *mut *const torITorControlPort,
    ) -> Result<(), nsresult> {
        if out.is_null() {
            return Err(NS_ERROR_NULL_POINTER);
        }
        let cp = RefPtr::new(ControlPortXpcom::new_tcp(host, port)?.coerce::<torITorControlPort>());
        cp.forget(unsafe { &mut *out });
        Ok(())
    }

    xpcom_method!(create_control_port_ipc => CreateControlPortIPC(socket: *const nsIFile, out: *mut *const torITorControlPort));
    fn create_control_port_ipc(
        &self,
        socket: &nsIFile,
        out: *mut *const torITorControlPort,
    ) -> Result<(), nsresult> {
        if out.is_null() {
            return Err(NS_ERROR_NULL_POINTER);
        }
        let cp = RefPtr::new(ControlPortXpcom::new_ipc(socket)?.coerce::<torITorControlPort>());
        cp.forget(unsafe { &mut *out });
        Ok(())
    }
}

// See toolkit/components/extensions/storage/webext_storage_bridge/src/lib.rs.
#[no_mangle]
pub unsafe extern "C" fn NewTorServiceImpl(result: *mut *const torITorService) -> nsresult {
    if result.is_null() {
        return NS_ERROR_NULL_POINTER;
    }
    let service = TorService::new();
    RefPtr::new(service.coerce::<torITorService>()).forget(&mut *result);
    NS_OK
}
