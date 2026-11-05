// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::fmt;

use crate::ctor::{controller::ControllerError, reply_parser::Reply};

pub struct Command<T> {
    pub command: String,
    pub handler: Box<dyn Fn(Reply) -> Result<T, ControllerError>>,
}

impl<T> fmt::Debug for Command<T> {
    /// Custom formatter that redacts the arguments, as they might contain
    /// sensitive information, such as passwords or keys.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut split = self.command.splitn(2, ' ');
        let mut command = split.next().unwrap_or_default().to_string();
        if split.next().is_some() {
            command.push_str(" ...");
        }
        if command.is_empty() {
            command = String::from("(empty)");
        }

        f.debug_struct("Command")
            .field("command", &command)
            .field("handler", &"<handler>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_fmt_is_redacted() {
        {
            let cmd = Command {
                command: String::from("QUIT"),
                handler: Box::new(|_| Ok(())),
            };
            let cmd_fmt = format!("{:?}", cmd);
            assert!(cmd_fmt.contains("command: \"QUIT\""));
        }
        {
            let password = "p4ssw0rd";
            let cmd = Command {
                command: format!("SETCONF Socks5ProxyPassword={}", password),
                handler: Box::new(|_| Ok(())),
            };
            let cmd_fmt = format!("{:?}", cmd);
            assert!(cmd_fmt.contains("command: \"SETCONF ...\""));
            assert!(!cmd_fmt.contains(password));
        }
        {
            let cmd = Command {
                command: String::from(""),
                handler: Box::new(|_| Ok(())),
            };
            let cmd_fmt = format!("{:?}", cmd);
            assert!(cmd_fmt.contains("command: \"(empty)\""));
        }
    }
}
