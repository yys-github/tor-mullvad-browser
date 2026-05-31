// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::Command;
use crate::ctor::controller::{parsers::parse_onion_client_auth_view, types::*, ControllerError};

pub fn onion_auth_view_keys() -> Result<Command<Vec<OnionClientAuth>>, ControllerError> {
    Ok(Command {
        command: String::from("ONION_CLIENT_AUTH_VIEW\r\n"),
        handler: Box::new(parse_onion_client_auth_view),
    })
}
