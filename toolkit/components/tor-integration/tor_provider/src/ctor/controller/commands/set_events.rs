// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::Command;
use crate::ctor::controller::{parsers::parse_ack, ControllerError};

pub fn set_events(events: &[&str]) -> Result<Command<u16>, ControllerError> {
    let mut command = String::from("SETEVENTS");
    for e in events {
        command.push(' ');
        command.push_str(e);
    }
    command.push_str("\r\n");
    Ok(Command {
        command,
        handler: Box::new(parse_ack),
    })
}
