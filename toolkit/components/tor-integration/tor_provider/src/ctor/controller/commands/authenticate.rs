// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::Command;
use crate::ctor::controller::{parsers::parse_ack, ControllerError};

pub fn authenticate(password: &[u8]) -> Result<Command<u16>, ControllerError> {
    const COMMAND: &str = "AUTHENTICATE";
    let mut command = String::new();
    command.reserve(COMMAND.len() + 1 + password.len() * 2 + 2);
    command.push_str(COMMAND);
    if !password.is_empty() {
        command.push(' ');
        command.push_str(hex::encode(password).as_str());
    }
    command.push_str("\r\n");
    Ok(Command {
        command,
        handler: Box::new(parse_ack),
    })
}
