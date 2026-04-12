// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::Command;
use crate::ctor::controller::{parsers::parse_ack, ControllerError};

pub fn reset_owning_controller_process() -> Result<Command<u16>, ControllerError> {
    Ok(Command {
        command: String::from("RESETCONF __OwningControllerProcess\r\n"),
        handler: Box::new(parse_ack),
    })
}
