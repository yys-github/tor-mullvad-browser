// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::io::{self, Write};

use super::{
    error::ReplyError,
    line::{DetailReplyLine, EndReplyLine},
};

/// A control port reply to a command, or an async notification.
/// This struct represents both synchronous replies to commands and asynchronous
/// notifications.
///
/// The reply is guaranteed to be complete (i.e., it must contain and end line).
///
/// The first parsed status code determines the reply type (sync vs async).
/// All subsequent lines must match that type; in other words, mixing sync and
/// async lines is illegal.
#[derive(Debug)]
pub struct Reply {
    detail_lines: Vec<DetailReplyLine>,
    end_reply_line: EndReplyLine,
}

impl Reply {
    pub(crate) fn new(
        detail_lines: Vec<DetailReplyLine>,
        end_reply_line: EndReplyLine,
    ) -> Result<Self, ReplyError> {
        // We expect this check to be handled by the caller too...
        let is_async = end_reply_line.is_async();
        for line in detail_lines.iter() {
            if line.is_async() != is_async {
                return Err(ReplyError::SyncAsyncMixed);
            }
        }

        Ok(Self {
            detail_lines,
            end_reply_line,
        })
    }

    /// Tell whether a certain reply contain detail lines.
    pub fn has_details(&self) -> bool {
        !self.detail_lines.is_empty()
    }

    /// Access the detail lines.
    pub fn details(&self) -> &[DetailReplyLine] {
        &self.detail_lines
    }

    /// Access the end reply line.
    pub fn end_line(&self) -> &EndReplyLine {
        &self.end_reply_line
    }

    /// Write the entire raw reply.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        for line in self.detail_lines.iter() {
            line.write_to(w)?;
        }
        self.end_reply_line.write_to(w)
    }

    pub fn is_async(&self) -> bool {
        self.end_reply_line.is_async()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn sync_async_mixed() {
        {
            assert_eq!(
                Reply::new(
                    vec![DetailReplyLine::MidReplyLine {
                        code: 300,
                        line: Bytes::from_static(b"Error"),
                    }],
                    EndReplyLine {
                        code: 600,
                        line: Bytes::from_static(b"Notification"),
                    },
                )
                .unwrap_err(),
                ReplyError::SyncAsyncMixed
            );
        }

        {
            assert_eq!(
                Reply::new(
                    vec![DetailReplyLine::MidReplyLine {
                        code: 600,
                        line: Bytes::from_static(b"Async"),
                    }],
                    EndReplyLine {
                        code: 450,
                        line: Bytes::from_static(b"Something else"),
                    },
                )
                .unwrap_err(),
                ReplyError::SyncAsyncMixed
            );
        }
    }

    #[test]
    fn test_is_async() {
        {
            let r = Reply::new(
                vec![],
                EndReplyLine {
                    code: 600,
                    line: Bytes::from_static(b"Notification"),
                },
            )
            .unwrap();
            assert!(r.is_async());
        }
        {
            let r = Reply::new(
                vec![],
                EndReplyLine {
                    code: 599,
                    line: Bytes::from_static(b"Error"),
                },
            )
            .unwrap();
            assert!(!r.is_async());
        }
    }
}
