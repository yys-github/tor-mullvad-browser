// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use serde::Serialize;
use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bridge {
    pub transport: Option<String>,
    pub address: SocketAddr,
    pub fingerprint: Option<[u8; 20]>,
    pub args: Option<Vec<u8>>,
}
