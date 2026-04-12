// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::ctor::{controller::ControllerError, reply_parser::Reply};

pub struct Command<T> {
    pub command: String,
    pub handler: Box<dyn Fn(Reply) -> Result<T, ControllerError>>,
}
