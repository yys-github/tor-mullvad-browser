/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use nserror::{nsresult, NS_ERROR_NULL_POINTER, NS_OK};
use xpcom::{interfaces::ITorService, RefPtr};

#[xpcom(implement(ITorService), atomic)]
struct TorService {}

impl TorService {
    fn new() -> RefPtr<TorService> {
        TorService::allocate(InitTorService {})
    }
}

// See toolkit/components/extensions/storage/webext_storage_bridge/src/lib.rs.
#[no_mangle]
pub unsafe extern "C" fn NewTorServiceImpl(result: *mut *const ITorService) -> nsresult {
    if result.is_null() {
        return NS_ERROR_NULL_POINTER;
    }
    let service = TorService::new();
    RefPtr::new(service.coerce::<ITorService>()).forget(&mut *result);
    NS_OK
}
