// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::{Bytes, BytesMut};
use memchr::memchr_iter;
use std::{collections::VecDeque, mem};

use super::{
    error::ReplyError,
    reply::Reply,
    replybuf::{ReplyBuf, ReplyBufState},
};

#[derive(Default)]
pub(crate) struct ReplyFactory {
    // Internal buffer for partial reads. It must not include a CRLF.
    buffer: BytesMut,
    // The builder, saved internally for partial reads.
    reply_buf: ReplyBuf,
}

impl ReplyFactory {
    /// Parse some data to potentially build multiple replies.
    /// Any data that is not consumed to create replies will be buffered for the
    /// next call to this function.
    ///
    /// Notice that an error in a line will inevitably drop well-formed replies
    /// that are succesfully parsed in a single call.
    /// The reason is that protocol errors are likely not recoverable, and a
    /// single outer result is more ergonomic than multiple inner results.
    pub fn build(&mut self, data: &Bytes) -> Result<VecDeque<Reply>, ReplyError> {
        debug_assert!(
            get_next_line(self.buffer.as_ref(), 0).is_none(),
            "self.buffer does not contain a CRLF sequence."
        );

        let mut lines = VecDeque::new();
        let mut offset = 0;
        while let Some(reply) = self.make_reply(data, &mut offset)? {
            lines.push_back(reply);
        }
        Ok(lines)
    }

    /// Extract entire lines from the passed buffer, until a complete reply is
    /// seen. At that point, offset is updated for future calls.
    fn make_reply(
        &mut self,
        data: &Bytes,
        offset: &mut usize,
    ) -> Result<Option<Reply>, ReplyError> {
        debug_assert!(
            *offset == 0 || self.buffer.is_empty(),
            "Data is consumed contiguously: either starting fresh or buffer is exhausted."
        );

        if self.buffer.last() == Some(&b'\r') && data.first() == Some(&b'\n') {
            *offset = 1;
            self.buffer.truncate(self.buffer.len() - 1);
            let buf = mem::take(&mut self.buffer);
            match mem::take(&mut self.reply_buf).add_line(buf.freeze())? {
                ReplyBufState::Complete(reply) => {
                    return Ok(Some(reply));
                }
                ReplyBufState::Incomplete(reply_buf) => {
                    self.reply_buf = reply_buf;
                }
            }
        }

        while let Some(line_end) = get_next_line(data, *offset) {
            // This loop is structured to reduce the usage of the internal
            // buffer as use the provided one instead, to minimize allocations:
            // Bytes instances share the underlying buffer, so creating
            // subslices is cheap.
            let line = if !self.buffer.is_empty() {
                self.buffer.extend_from_slice(&data[*offset..line_end]);
                let buf = mem::take(&mut self.buffer);
                buf.freeze()
            } else {
                data.slice(*offset..line_end)
            };
            *offset = line_end + 2;
            match mem::take(&mut self.reply_buf).add_line(line)? {
                ReplyBufState::Complete(reply) => {
                    return Ok(Some(reply));
                }
                ReplyBufState::Incomplete(reply_buf) => {
                    self.reply_buf = reply_buf;
                }
            }
        }

        // Notice: if we did not have any line, we are going to copy the entire
        // slice into the buffer, which is the expected behavior.
        if *offset < data.len() {
            self.buffer.extend_from_slice(&data[*offset..]);
        }
        Ok(None)
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    #[cfg(test)]
    pub fn has_pending_data(&self) -> bool {
        !self.buffer.is_empty() || self.reply_buf.has_pending_data()
    }
}

/// Get the position of a \r\n on a line (if present).
fn get_next_line(data: &[u8], offset: usize) -> Option<usize> {
    if data.len() < 2 || offset >= data.len() - 1 {
        return None;
    }
    for i in memchr_iter(b'\r', &data[offset..data.len() - 1]) {
        let pos = offset + i;
        debug_assert!(pos + 1 < data.len());
        if data[pos + 1] == b'\n' {
            return Some(pos);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::super::{
        line::{CmdData, DetailReplyLine},
        test_utils::check_250_ok,
    };
    use super::*;

    #[test]
    fn test_get_next_line() {
        assert_eq!(get_next_line(&[], 0), None);
        assert_eq!(get_next_line(b"Test", 0), None);
        assert_eq!(get_next_line(b"Test\r\n", 0), Some(4));
        assert_eq!(get_next_line(b"\r\nTest\r\n", 0), Some(0));
        assert_eq!(get_next_line(b"\r\nTest\r\n", 1), Some(6));
        assert_eq!(get_next_line(b"\r\nTest\r\n", 2), Some(6));
        assert_eq!(get_next_line(b"\nTest\r\n", 0), Some(5));
        assert_eq!(get_next_line(b"Test\r\nTest", 6), None);
        assert_eq!(get_next_line(b"Test\r\n\r\n", 6), Some(6));
        assert_eq!(get_next_line(b"Test\r\n", 5), None);
        assert_eq!(get_next_line(b"Test\r\n", 10), None);
    }

    #[test]
    fn simple_reply() {
        let mut factory = ReplyFactory::default();
        let replies = factory.build(&Bytes::from_static(b"250 OK\r\n")).unwrap();
        assert_eq!(replies.len(), 1);
        check_250_ok(replies.front().unwrap(), false);
    }

    #[test]
    fn split_crlf() {
        {
            let mut factory = ReplyFactory::default();
            assert!(factory
                .build(&Bytes::from_static(b"250 OK"))
                .unwrap()
                .is_empty());
            let replies = factory.build(&Bytes::from_static(b"\r\n")).unwrap();
            assert_eq!(replies.len(), 1);
            check_250_ok(replies.front().unwrap(), false);
        }

        {
            let mut factory = ReplyFactory::default();
            assert!(factory
                .build(&Bytes::from_static(b"250 OK\r"))
                .unwrap()
                .is_empty());
            let replies = factory.build(&Bytes::from_static(b"\n")).unwrap();
            assert_eq!(replies.len(), 1);
            check_250_ok(replies.front().unwrap(), false);
        }
    }

    #[test]
    fn ignore_stray_lf() {
        let mut factory = ReplyFactory::default();
        assert!(factory
            .build(&Bytes::from_static(b"250 OK\n"))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn multiple_replies_single_buffer() {
        let mut factory = ReplyFactory::default();
        let replies = factory
            .build(&Bytes::from_static(b"250 OK\r\n250 OK\r\n"))
            .unwrap();
        assert_eq!(replies.len(), 2);
    }

    #[test]
    fn multiline_reply_single_buffer() {
        let mut factory = ReplyFactory::default();
        let replies = factory
            .build(&Bytes::from_static(b"250-Details\r\n250 OK\r\n"))
            .unwrap();
        assert_eq!(replies.len(), 1);
        assert!(replies.front().unwrap().has_details());
    }

    #[test]
    fn multiline_reply_split_across_builds() {
        let mut factory = ReplyFactory::default();
        assert!(factory
            .build(&Bytes::from_static(b"250-Details"))
            .unwrap()
            .is_empty());
        let replies = factory
            .build(&Bytes::from_static(b"\r\n250 OK\r\n"))
            .unwrap();
        assert!(replies.len() == 1);
        check_250_ok(replies.front().unwrap(), true);
    }

    #[test]
    fn data_section_split_across_builds() {
        let mut factory = ReplyFactory::default();
        assert!(factory
            .build(&Bytes::from_static(b"250+data=\r\nline1"))
            .unwrap()
            .is_empty());
        let replies = factory
            .build(&Bytes::from_static(b"\r\n.\r\n250 OK\r\n"))
            .unwrap();
        assert!(replies.len() == 1);
        let reply = replies.front().unwrap();
        assert_eq!(
            reply.details(),
            vec![DetailReplyLine::DataReplyLine {
                code: 250,
                lines: CmdData::try_from_vec(vec![
                    Bytes::from_static(b"data="),
                    Bytes::from_static(b"line1")
                ])
                .unwrap(),
            }],
        );
        check_250_ok(reply, true);
    }

    #[test]
    fn multiples_replies_split_across_builds() {
        let mut factory = ReplyFactory::default();
        assert!(factory
            .build(&Bytes::from_static(b"250-Details\r\n250 "))
            .unwrap()
            .is_empty());
        let replies = factory
            .build(&Bytes::from_static(b"OK\r\n250 OK\r\n"))
            .unwrap();
        assert!(replies.len() == 2);
        check_250_ok(replies.front().unwrap(), true);
        check_250_ok(replies.back().unwrap(), false);
    }

    #[test]
    fn stray_cr_before_normal_crlf() {
        let mut factory = ReplyFactory::default();
        let replies = factory.build(&Bytes::from_static(b"250 OK\r\r\n")).unwrap();
        // Should treat the first \r as part of line content, not line ending
        // (since it is not followed by \n).
        assert_eq!(replies.len(), 1);
        let reply = replies.front().unwrap();
        assert_eq!(reply.end_line().line, Bytes::from_static(b"OK\r"));
    }

    #[test]
    fn empty_data() {
        let mut factory = ReplyFactory::default();
        assert!(factory.build(&Bytes::new()).unwrap().is_empty());
    }

    #[test]
    fn empty_data_after_partial() {
        let mut factory = ReplyFactory::default();
        assert!(factory
            .build(&Bytes::from_static(b"250 OK"))
            .unwrap()
            .is_empty());

        // Empty data should not break anything
        assert!(factory.build(&Bytes::new()).unwrap().is_empty());

        // And we can still complete the reply
        let replies = factory.build(&Bytes::from_static(b"\r\n")).unwrap();
        assert_eq!(replies.len(), 1);
        check_250_ok(replies.front().unwrap(), false);
    }

    #[test]
    fn clear_after_partial_reply() {
        let mut factory = ReplyFactory::default();
        // Start a reply but don't complete it
        assert!(factory
            .build(&Bytes::from_static(b"250-Details"))
            .unwrap()
            .is_empty());

        // Clear and start fresh with completely different data
        factory.clear();
        let replies = factory
            .build(&Bytes::from_static(b"350 Continue\r\n"))
            .unwrap();
        assert_eq!(replies.len(), 1);
        let reply = replies.front().unwrap();
        assert_eq!(reply.end_line().code, 350);
        assert_eq!(reply.end_line().line, Bytes::from_static(b"Continue"));
    }

    #[test]
    fn clear_after_complete_reply() {
        let mut factory = ReplyFactory::default();
        let replies = factory.build(&Bytes::from_static(b"250 OK\r\n")).unwrap();
        assert_eq!(replies.len(), 1);
        check_250_ok(replies.front().unwrap(), false);

        // Clear and process new reply
        factory.clear();
        let replies = factory
            .build(&Bytes::from_static(b"350 Continue\r\n"))
            .unwrap();
        assert_eq!(replies.len(), 1);
        let reply = replies.front().unwrap();
        assert_eq!(reply.end_line().code, 350);
        assert_eq!(reply.end_line().line, Bytes::from_static(b"Continue"));
    }

    #[test]
    fn clear_with_buffered_partial_line() {
        let mut factory = ReplyFactory::default();
        // Buffer partial data (no complete line)
        assert!(factory
            .build(&Bytes::from_static(b"250 OK"))
            .unwrap()
            .is_empty());

        // Clear discards the buffered data
        assert!(factory.has_pending_data());
        factory.clear();
        assert!(!factory.has_pending_data());

        // Start fresh with different data
        let replies = factory
            .build(&Bytes::from_static(b"350 Continue\r\n"))
            .unwrap();
        assert_eq!(replies.len(), 1);
        let reply = replies.front().unwrap();
        assert_eq!(reply.end_line().code, 350);
        assert_eq!(reply.end_line().line, Bytes::from_static(b"Continue"));
    }

    #[test]
    fn clear_with_split_crlf_in_buffer() {
        let mut factory = ReplyFactory::default();
        // End with \r, will be buffered
        assert!(factory
            .build(&Bytes::from_static(b"250 OK\r"))
            .unwrap()
            .is_empty());

        // Clear before sending the \n
        assert!(factory.has_pending_data());
        factory.clear();
        assert!(!factory.has_pending_data());

        // New data does not complete the old reply
        let replies = factory
            .build(&Bytes::from_static(b"250 Different\r\n"))
            .unwrap();
        assert_eq!(replies.len(), 1);
        let reply = replies.front().unwrap();
        assert_eq!(reply.end_line().code, 250);
        assert_eq!(reply.end_line().line, Bytes::from_static(b"Different"));
    }

    #[test]
    fn malformed_lines() {
        let mut factory = ReplyFactory::default();

        assert_eq!(
            factory.build(&Bytes::from_static(b"A\r\n")).unwrap_err(),
            ReplyError::LineTooShort,
        );
        assert!(!factory.has_pending_data());

        assert_eq!(
            factory.build(&Bytes::from_static(b"aaa \r\n")).unwrap_err(),
            ReplyError::BadStatusCode,
        );
        assert!(!factory.has_pending_data());

        assert_eq!(
            factory
                .build(&Bytes::from_static(b"650-Notification\r\n250 OK\r\n"))
                .unwrap_err(),
            ReplyError::SyncAsyncMixed,
        );
        assert!(!factory.has_pending_data());

        assert_eq!(
            factory
                .build(&Bytes::from_static(b"250-Data\r\n650 Notification\r\n"))
                .unwrap_err(),
            ReplyError::SyncAsyncMixed,
        );
        assert!(!factory.has_pending_data());

        assert_eq!(
            factory.build(&Bytes::from_static(b"250|\r\n")).unwrap_err(),
            ReplyError::BadSeparator(b'|'),
        );
        assert!(!factory.has_pending_data());

        assert_eq!(
            factory
                .build(&Bytes::from_static(b"250 OK\r\nA\r\n"))
                .unwrap_err(),
            ReplyError::LineTooShort,
        );
        assert!(!factory.has_pending_data());
    }
}
