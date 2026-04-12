// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::Bytes;
use std::{
    io::{self, Write},
    ops::Deref,
};

pub const ERROR_START: u16 = 400;
pub const ASYNC_START: u16 = 600;

/// An enum to represent MidReplyLine and DataReplyLine form the control port
/// spec (both for synchronous and asynchronous replies).
/// EndReplyLine is not in the enum because must have exactly one of them.
/// Therefore, we have a specific struct for it, as this allows to guarantee
/// some invariants by construction.
///
/// We decided to use bytes rather than strings as the spec explicitly allows
/// all 8-bit characters. For those cases where we want strings, we perform the
/// conversion only after unescaping.
///
/// The convention is that lines should not contain CRLF (not even the trailing
/// one).
/// This is followed especially when building a reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailReplyLine {
    MidReplyLine { code: u16, line: Bytes },
    // The Vec must contain at least one element; individual Bytes entries may
    // be empty.
    DataReplyLine { code: u16, lines: CmdData },
}

impl DetailReplyLine {
    /// Returns the status code associated to this line.
    pub fn code(&self) -> u16 {
        match self {
            Self::MidReplyLine { code, .. } | Self::DataReplyLine { code, .. } => *code,
        }
    }

    /// Tells whether the line is part of an async reply.
    pub fn is_async(&self) -> bool {
        self.code() >= ASYNC_START
    }

    /// Writes the raw line, including the ending CRLF.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        match self {
            Self::MidReplyLine { code, line } => {
                write!(w, "{}-", code)?;
                w.write_all(line.as_ref())?;
                w.write_all(b"\r\n")?;
            }

            Self::DataReplyLine { code, lines } => {
                write!(w, "{}+", code)?;
                for line in lines.iter() {
                    if line.starts_with(b".") {
                        w.write_all(b".")?;
                    }
                    w.write_all(line.as_ref())?;
                    w.write_all(b"\r\n")?;
                }
                w.write_all(b".\r\n")?;
            }
        }
        Ok(())
    }
}

/// A container for the data of the DataReplyLines.
/// It guarantees that there will always be at least one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmdData {
    data: Vec<Bytes>,
}

impl CmdData {
    pub(super) fn new(b: Bytes) -> Self {
        Self { data: vec![b] }
    }

    #[cfg(test)]
    pub(super) fn try_from_vec(data: Vec<Bytes>) -> Option<Self> {
        if !data.is_empty() {
            Some(Self { data })
        } else {
            None
        }
    }

    pub(super) fn push(&mut self, value: Bytes) {
        self.data.push(value);
    }
}

impl Deref for CmdData {
    type Target = [Bytes];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndReplyLine {
    pub code: u16,
    pub line: Bytes,
}

impl EndReplyLine {
    /// Tells whether the line is an error.
    pub fn is_error(&self) -> bool {
        self.code >= ERROR_START && self.code < ASYNC_START
    }

    /// Tells whether the line is part of an async reply.
    pub fn is_async(&self) -> bool {
        self.code >= ASYNC_START
    }

    /// Writes the raw line, including the ending CRLF.
    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write!(w, "{} ", self.code)?;
        w.write_all(self.line.as_ref())?;
        w.write_all(b"\r\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_code() {
        {
            let r = DetailReplyLine::MidReplyLine {
                code: 650,
                line: Bytes::from_static(b"test="),
            };
            assert_eq!(r.code(), 650);
        }
        {
            let r = DetailReplyLine::DataReplyLine {
                code: 250,
                lines: CmdData::try_from_vec(vec![Bytes::from_static(b"line1")]).unwrap(),
            };
            assert_eq!(r.code(), 250);
        }
    }

    #[test]
    fn check_type() {
        {
            let r = EndReplyLine {
                code: 250,
                line: Bytes::from_static(b"OK"),
            };
            assert!(!r.is_error());
            assert!(!r.is_async());
        }
        {
            let r = EndReplyLine {
                code: 399,
                line: Bytes::from_static(b"Success"),
            };
            assert!(!r.is_error());
            assert!(!r.is_async());
        }
        {
            let r = EndReplyLine {
                code: 400,
                line: Bytes::from_static(b"Temporary error"),
            };
            assert!(r.is_error());
            assert!(!r.is_async());
        }
        {
            let r = EndReplyLine {
                code: 500,
                line: Bytes::from_static(b"Permanent error"),
            };
            assert!(r.is_error());
            assert!(!r.is_async());
        }
        {
            let r = EndReplyLine {
                code: 599,
                line: Bytes::from_static(b"Yet another error"),
            };
            assert!(r.is_error());
            assert!(!r.is_async());
        }
        {
            let r = EndReplyLine {
                code: 600,
                line: Bytes::from_static(b"Notification"),
            };
            assert!(!r.is_error());
            assert!(r.is_async());
        }
        {
            let r = EndReplyLine {
                code: 650,
                line: Bytes::from_static(b"CIRC BUILT"),
            };
            assert!(!r.is_error());
            assert!(r.is_async());
        }

        {
            let r = DetailReplyLine::MidReplyLine {
                code: 250,
                line: Bytes::from_static(b"OK"),
            };
            assert!(!r.is_async());
        }
        {
            let r = DetailReplyLine::MidReplyLine {
                code: 399,
                line: Bytes::from_static(b"Success"),
            };
            assert!(!r.is_async());
        }
        {
            let r = DetailReplyLine::MidReplyLine {
                code: 400,
                line: Bytes::from_static(b"Temporary error"),
            };
            assert!(!r.is_async());
        }
        {
            let r = DetailReplyLine::MidReplyLine {
                code: 500,
                line: Bytes::from_static(b"Permanent error"),
            };
            assert!(!r.is_async());
        }
        {
            let r = DetailReplyLine::MidReplyLine {
                code: 599,
                line: Bytes::from_static(b"Yet another error"),
            };
            assert!(!r.is_async());
        }
        {
            let r = DetailReplyLine::MidReplyLine {
                code: 600,
                line: Bytes::from_static(b"Notification"),
            };
            assert!(r.is_async());
        }
        {
            let r = DetailReplyLine::MidReplyLine {
                code: 650,
                line: Bytes::from_static(b"CIRC BUILT"),
            };
            assert!(r.is_async());
        }
    }

    #[test]
    fn write_end_line() {
        let r = EndReplyLine {
            code: 450,
            line: Bytes::from_static(b"Error"),
        };
        let mut buf = Vec::new();
        r.write_to(&mut buf).unwrap();
        assert_eq!(&buf, b"450 Error\r\n");
    }

    #[test]
    fn write_mid_line() {
        let r = DetailReplyLine::MidReplyLine {
            code: 650,
            line: Bytes::from_static(b"test="),
        };
        let mut buf = Vec::new();
        r.write_to(&mut buf).unwrap();
        assert_eq!(&buf, b"650-test=\r\n");
    }

    #[test]
    fn write_data_line() {
        let r = DetailReplyLine::DataReplyLine {
            code: 250,
            lines: CmdData::try_from_vec(vec![
                Bytes::from_static(b"line1="),
                Bytes::from_static(b"line2"),
                Bytes::from_static(b".line3"),
            ])
            .unwrap(),
        };
        let mut buf = Vec::new();
        r.write_to(&mut buf).unwrap();
        assert_eq!(&buf, b"250+line1=\r\nline2\r\n..line3\r\n.\r\n");
    }
}
