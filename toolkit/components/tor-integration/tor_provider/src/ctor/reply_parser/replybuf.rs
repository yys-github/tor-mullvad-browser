// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::Bytes;

use super::{
    error::ReplyError,
    line::{CmdData, DetailReplyLine, EndReplyLine, ASYNC_START},
    reply::Reply,
};

#[derive(Debug)]
pub(super) enum ReplyBufState {
    Complete(Reply),
    Incomplete(ReplyBuf),
}

#[derive(Default, Debug)]
pub(super) struct ReplyBuf {
    pending_dataline: Option<(u16, CmdData)>,
    detail_lines: Vec<DetailReplyLine>,
    is_async: Option<bool>,
}

impl ReplyBuf {
    /// Parse a complete line (without the trailing \r\n).
    /// Returns ReplyBufState::Incomplete(self) if more lines are needed, or
    /// ReplyBufState::Complete (consuming the buffer entirely) when the reply
    /// is finished.
    /// This makes invalid state transitions impossible at compile-time.
    pub fn add_line(mut self, data: Bytes) -> Result<ReplyBufState, ReplyError> {
        debug_assert!(
            !data.ends_with(b"\r\n"),
            "Lines should be passed without the final CRLF."
        );

        if let Some((code, mut lines)) = self.pending_dataline.take() {
            if data.len() == 1 && data[0] == b'.' {
                self.detail_lines
                    .push(DetailReplyLine::DataReplyLine { code, lines });
            } else {
                // Periods are escaped with periods, so they must be ignored.
                let line_data = if data.first() == Some(&b'.') {
                    data.slice(1..)
                } else {
                    data
                };
                lines.push(line_data);
                self.pending_dataline = Some((code, lines));
            }
            return Ok(ReplyBufState::Incomplete(self));
        }

        if data.len() < 4 {
            return Err(ReplyError::LineTooShort);
        }

        // Notice: the specs explicitly mention various codes that are used.
        // https://spec.torproject.org/control-spec/replies.html#replies
        // However, they also mention "currently". While it is unlikely that it
        // will be extended at this point, we decided not to enforce the code
        // range.
        let code: u16 = str::from_utf8(&data[0..3])
            .map_err(|_| ReplyError::BadStatusCode)?
            .parse()
            .map_err(|_| ReplyError::BadStatusCode)?;
        let async_line = code >= ASYNC_START;
        match self.is_async {
            Some(is_async) => {
                if is_async != async_line {
                    return Err(ReplyError::SyncAsyncMixed);
                }
            }
            None => {
                self.is_async = Some(async_line);
            }
        }
        let separator = data[3];
        let line = data.slice(4..);

        match separator {
            b'+' => {
                self.pending_dataline = Some((code, CmdData::new(line)));
                Ok(ReplyBufState::Incomplete(self))
            }
            b'-' => {
                self.detail_lines
                    .push(DetailReplyLine::MidReplyLine { code, line });
                Ok(ReplyBufState::Incomplete(self))
            }
            b' ' => Ok(ReplyBufState::Complete(Reply::new(
                self.detail_lines,
                EndReplyLine { code, line },
            )?)),
            b => Err(ReplyError::BadSeparator(b)),
        }
    }

    #[cfg(test)]
    pub fn has_pending_data(&self) -> bool {
        !self.detail_lines.is_empty() || self.pending_dataline.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::check_250_ok;
    use super::*;

    fn add_detail_line(buf: ReplyBuf, line: &'static [u8]) -> ReplyBuf {
        if let ReplyBufState::Incomplete(b) = buf.add_line(Bytes::from_static(line)).unwrap() {
            b
        } else {
            unreachable!("We are adding details line, we should not get a reply at this point!");
        }
    }

    fn make_happy_reply(details: &[&'static [u8]], end_line: &'static [u8]) -> Reply {
        let mut buf = ReplyBuf::default();
        let mut raw = Vec::new();
        for l in details {
            buf = add_detail_line(buf, *l);
            raw.extend_from_slice(*l);
            raw.extend_from_slice(b"\r\n");
        }

        let reply = match buf.add_line(Bytes::from_static(end_line)).unwrap() {
            ReplyBufState::Complete(r) => r,
            ReplyBufState::Incomplete(_) => {
                unreachable!("We added the end line, we expected to get a reply at this point!")
            }
        };
        raw.extend_from_slice(end_line);
        raw.extend_from_slice(b"\r\n");

        assert_eq!(reply.has_details(), !details.is_empty());
        let mut buf = Vec::new();
        reply.write_to(&mut buf).unwrap();
        assert_eq!(buf, raw);

        for l in reply.details() {
            assert_eq!(l.is_async(), reply.is_async())
        }
        assert_eq!(reply.end_line().is_async(), reply.is_async());

        reply
    }

    #[test]
    fn single_line_reply() {
        let reply = make_happy_reply(&[], b"250 OK");
        check_250_ok(&reply, false);
        assert!(!reply.is_async());
    }

    #[test]
    fn single_line_async() {
        let reply = make_happy_reply(&[], b"650 INFO Test");
        assert_eq!(
            *reply.end_line(),
            EndReplyLine {
                code: 650,
                line: Bytes::from_static(b"INFO Test")
            }
        );
        assert!(reply.is_async());
    }

    #[test]
    fn multi_line_reply() {
        let reply = make_happy_reply(
            &[
                b"250+circuit-status=",
                b"21 BUILT $abc123~nick1,$999999~nick2,$aaabbb~nick3",
                b"22 EXTENDED $abc123~nick1,$1111111~nick4",
                b".",
                b"250-stream-status=",
            ],
            b"250 OK",
        );
        assert_eq!(
            reply.details(),
            &[
                DetailReplyLine::DataReplyLine {
                    code: 250,
                    lines: CmdData::try_from_vec(vec![
                        Bytes::from_static(b"circuit-status="),
                        Bytes::from_static(b"21 BUILT $abc123~nick1,$999999~nick2,$aaabbb~nick3"),
                        Bytes::from_static(b"22 EXTENDED $abc123~nick1,$1111111~nick4"),
                    ])
                    .unwrap(),
                },
                DetailReplyLine::MidReplyLine {
                    code: 250,
                    line: Bytes::from_static(b"stream-status="),
                },
            ],
        );
        check_250_ok(&reply, true);
    }

    #[test]
    fn multi_line_async() {
        let reply = make_happy_reply(&[b"650-WARN warning"], b"650 INFO info");
        assert!(reply.is_async());
        assert_eq!(
            reply.details(),
            &[DetailReplyLine::MidReplyLine {
                code: 650,
                line: Bytes::from_static(b"WARN warning"),
            },],
        );
        assert_eq!(
            *reply.end_line(),
            EndReplyLine {
                code: 650,
                line: Bytes::from_static(b"INFO info")
            }
        );
    }

    #[test]
    fn multi_line_escaped_dot() {
        let reply = make_happy_reply(&[b"250+data=", b"..", b"."], b"250 OK");
        assert_eq!(
            *reply.details(),
            vec![DetailReplyLine::DataReplyLine {
                code: 250,
                lines: CmdData::try_from_vec(vec![
                    Bytes::from_static(b"data="),
                    Bytes::from_static(b".")
                ])
                .unwrap()
            },]
        );
        check_250_ok(&reply, true);
    }

    #[test]
    fn mixed_codes() {
        let reply = make_happy_reply(
            &[
                b"250+section1=",
                b"data1",
                b".",
                b"251+section2=",
                b"data2",
                b".",
                b"400-section3=data3",
            ],
            b"550 Error",
        );
        assert_eq!(
            *reply.details(),
            vec![
                DetailReplyLine::DataReplyLine {
                    code: 250,
                    lines: CmdData::try_from_vec(vec![
                        Bytes::from_static(b"section1="),
                        Bytes::from_static(b"data1"),
                    ])
                    .unwrap(),
                },
                DetailReplyLine::DataReplyLine {
                    code: 251,
                    lines: CmdData::try_from_vec(vec![
                        Bytes::from_static(b"section2="),
                        Bytes::from_static(b"data2"),
                    ])
                    .unwrap(),
                },
                DetailReplyLine::MidReplyLine {
                    code: 400,
                    line: Bytes::from_static(b"section3=data3")
                },
            ],
        );
        assert_eq!(
            *reply.end_line(),
            EndReplyLine {
                code: 550,
                line: Bytes::from_static(b"Error")
            }
        );
    }

    #[test]
    fn multi_line_actually_single() {
        let reply = make_happy_reply(&[b"250+data=", b"."], b"250 OK");
        assert_eq!(
            *reply.details(),
            vec![DetailReplyLine::DataReplyLine {
                code: 250,
                lines: CmdData::new(Bytes::from_static(b"data=")),
            },]
        );
        check_250_ok(&reply, true);
    }

    #[test]
    fn whitespace_only_data_line() {
        let reply = make_happy_reply(&[b"250+test=", b"  ", b"a line", b"."], b"250 OK");
        assert_eq!(
            *reply.details(),
            vec![DetailReplyLine::DataReplyLine {
                code: 250,
                lines: CmdData::try_from_vec(vec![
                    Bytes::from_static(b"test="),
                    Bytes::from_static(b"  "),
                    Bytes::from_static(b"a line"),
                ])
                .unwrap()
            }],
        );
        check_250_ok(&reply, true);
    }

    #[test]
    fn invalid_utf8_ignored() {
        let reply = make_happy_reply(&[], b"250 OK\xFF");
        assert_eq!(
            *reply.end_line(),
            EndReplyLine {
                code: 250,
                line: Bytes::from_static(b"OK\xFF")
            }
        );
    }

    #[test]
    fn line_too_short() {
        let buf = ReplyBuf::default();
        assert_eq!(
            buf.add_line(Bytes::from_static(b"25")).unwrap_err(),
            ReplyError::LineTooShort,
        );
    }

    #[test]
    fn bad_status_code() {
        {
            let buf = ReplyBuf::default();
            assert_eq!(
                buf.add_line(Bytes::from_static(b"12 Data")).unwrap_err(),
                ReplyError::BadStatusCode,
            );
        }

        {
            let buf = ReplyBuf::default();
            assert_eq!(
                buf.add_line(Bytes::from_static(b"aaa+")).unwrap_err(),
                ReplyError::BadStatusCode,
            );
        }

        {
            static INVALID_UNICODE: [u8; 4] = [b'2', 0xF0, b'0', b'+'];
            let buf = ReplyBuf::default();
            assert_eq!(
                buf.add_line(Bytes::from_static(&INVALID_UNICODE))
                    .unwrap_err(),
                ReplyError::BadStatusCode,
            );
        }
    }

    #[test]
    fn sync_async_mixed() {
        {
            let mut buf = ReplyBuf::default();
            buf = add_detail_line(buf, b"250-Something");
            assert_eq!(
                buf.add_line(Bytes::from_static(b"650 Async")).unwrap_err(),
                ReplyError::SyncAsyncMixed,
            );
        }

        {
            let mut buf = ReplyBuf::default();
            buf = add_detail_line(buf, b"650-Async");
            assert_eq!(
                buf.add_line(Bytes::from_static(b"250-Something"))
                    .unwrap_err(),
                ReplyError::SyncAsyncMixed,
            );
        }
    }

    #[test]
    fn bad_separator() {
        let buf = ReplyBuf::default();
        assert_eq!(
            buf.add_line(Bytes::from_static(b"250|Something"))
                .unwrap_err(),
            ReplyError::BadSeparator(b'|'),
        );
    }
}
