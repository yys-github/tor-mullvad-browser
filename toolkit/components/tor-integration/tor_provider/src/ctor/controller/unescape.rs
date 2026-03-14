// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::borrow::Cow;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum UnescapeError {
    #[error("space in an unquoted string")]
    SpaceInUnquotedString,
    #[error("unterminated escape sequence or string")]
    Unterminated,
    #[error("found an unescaped quote")]
    UnescapedQuote,
    #[error("found an invalid escape sequence")]
    InvalidEscape,
    #[error("an octal-escaped value exceeds 255")]
    OctalOverflow,
}

/// Adaptation of tor's unescape_string (src/lib/encoding/cstring.c).
/// The main difference is that we allow unquoted strings.
pub fn tor_unescape<'a>(buf: &'a [u8]) -> Result<Cow<'a, [u8]>, UnescapeError> {
    match buf.first() {
        Some(b'"') => {
            // Quoted string, consume the item and continue with the function.
        }
        _ => {
            if buf.contains(&b' ') {
                return Err(UnescapeError::SpaceInUnquotedString);
            }
            return Ok(Cow::Borrowed(buf));
        }
    }

    if buf.len() == 1 || buf.last() != Some(&b'\"') {
        return Err(UnescapeError::Unterminated);
    }

    // This is used only if we did an actual change.
    let mut out = None;

    // We already consumed the initial quote.
    let mut i = 1;
    while i < buf.len() {
        let c = buf[i];
        i += 1;
        match c {
            b'\"' => {
                if i != buf.len() {
                    return Err(UnescapeError::UnescapedQuote);
                }
                return Ok(out
                    .map(|b| Cow::Owned(b))
                    .unwrap_or_else(|| Cow::Borrowed(&buf[1..buf.len() - 1])));
            }
            0 | b'\n' => return Err(UnescapeError::Unterminated),
            b'\\' => {
                let esc = buf.get(i).ok_or(UnescapeError::Unterminated)?;
                i += 1;
                let out = out.get_or_insert_with(|| {
                    let mut b = Vec::with_capacity(buf.len() - 2);
                    b.extend(&buf[1..i - 2]);
                    b
                });
                out.push(match esc {
                    b'n' => Ok(b'\n'),
                    b'r' => Ok(b'\r'),
                    b't' => Ok(b'\t'),
                    b'x' | b'X' => unescape_hex(buf, &mut i),
                    b'0'..=b'7' => unescape_octal(buf, &mut i),
                    b'\'' | b'"' | b'\\' => Ok(*esc),
                    _ => Err(UnescapeError::InvalidEscape),
                }?);
            }
            _ => {
                if let Some(b) = out.as_mut() {
                    b.push(c);
                }
            }
        }
    }

    // If we fall out of the loop we never saw a closing quote.
    Err(UnescapeError::Unterminated)
}

fn unescape_hex(buf: &[u8], i: &mut usize) -> Result<u8, UnescapeError> {
    // The C code expects exactly two hex digits.
    *i += 2;
    str::from_utf8(&buf[*i - 2..*i])
        .ok()
        .and_then(|s| u8::from_str_radix(s, 16).ok())
        .ok_or(UnescapeError::InvalidEscape)
}

fn unescape_octal(buf: &[u8], i: &mut usize) -> Result<u8, UnescapeError> {
    *i -= 1;
    let mut len = 1;
    while (*i + len) < buf.len() && len < 3 {
        let b = buf[*i + len];
        if b >= b'0' && b <= b'7' {
            len += 1;
        } else {
            break;
        }
    }

    *i += len;
    // This should never fail with InvalidEscape, as we have just validated the
    // octal values.
    str::from_utf8(&buf[*i - len..*i])
        .ok()
        .and_then(|s| u16::from_str_radix(s, 8).ok())
        .ok_or(UnescapeError::InvalidEscape)
        .and_then(|v| v.try_into().map_err(|_| UnescapeError::OctalOverflow))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        assert_eq!(&*tor_unescape(b"test").unwrap(), b"test");
        assert_eq!(&*tor_unescape(b"\"test\"").unwrap(), b"test");
    }

    #[test]
    fn quote_in_unquoted() {
        assert_eq!(&*tor_unescape(b"te\"st").unwrap(), b"te\"st");
        assert_eq!(&*tor_unescape(b"test\"").unwrap(), b"test\"");
    }

    #[test]
    fn empty() {
        assert_eq!(&*tor_unescape(b"").unwrap(), &[]);
        assert_eq!(&*tor_unescape(b"\"\"").unwrap(), &[]);
    }

    #[test]
    fn unescape_simple() {
        assert_eq!(&*tor_unescape(b"\"\\n\"").unwrap(), b"\n");
        assert_eq!(&*tor_unescape(b"\"\\r\"").unwrap(), b"\r");
        assert_eq!(&*tor_unescape(b"\"\\t\"").unwrap(), b"\t");
        assert_eq!(&*tor_unescape(b"\"\\r\\n\"").unwrap(), b"\r\n");
        assert_eq!(&*tor_unescape(b"\"'\"").unwrap(), b"'");
        assert_eq!(&*tor_unescape(b"\"\\'\"").unwrap(), b"'");
        assert_eq!(&*tor_unescape(b"\"\\\"\"").unwrap(), b"\"");
        assert_eq!(&*tor_unescape(b"\"\\\\\"").unwrap(), b"\\");
    }

    #[test]
    fn unescape_hex() {
        assert_eq!(&*tor_unescape(b"\"\\x20\"").unwrap(), b" ");
        assert_eq!(&*tor_unescape(b"\"\\x20test\"").unwrap(), b" test");
        assert_eq!(&*tor_unescape(b"\"test\\x20\"").unwrap(), b"test ");
        assert_eq!(&*tor_unescape(b"\"te\\x20st\"").unwrap(), b"te st");

        assert_eq!(&*tor_unescape(b"\"\\X20\"").unwrap(), b" ");
        assert_eq!(&*tor_unescape(b"\"\\X20test\"").unwrap(), b" test");
        assert_eq!(&*tor_unescape(b"\"test\\X20\"").unwrap(), b"test ");
        assert_eq!(&*tor_unescape(b"\"te\\X20st\"").unwrap(), b"te st");

        assert_eq!(&*tor_unescape(b"\"\\x00\"").unwrap(), b"\0");
        assert_eq!(&*tor_unescape(b"\"test\\x00\"").unwrap(), b"test\0");
        assert_eq!(&*tor_unescape(b"\"\\x00test\"").unwrap(), b"\0test");
        assert_eq!(&*tor_unescape(b"\"\\X00\"").unwrap(), b"\0");
        assert_eq!(&*tor_unescape(b"\"TEST\\X00\"").unwrap(), b"TEST\0");
        assert_eq!(&*tor_unescape(b"\"\\X00TEST\"").unwrap(), b"\0TEST");
    }

    #[test]
    fn unescape_octal() {
        assert_eq!(&*tor_unescape(b"\"\\0\"").unwrap(), b"\0");
        assert_eq!(&*tor_unescape(b"\"\\00\"").unwrap(), b"\0");
        assert_eq!(&*tor_unescape(b"\"\\000\"").unwrap(), b"\0");

        assert_eq!(&*tor_unescape(b"\"\\2\"").unwrap(), b"\x02");
        assert_eq!(&*tor_unescape(b"\"\\02\"").unwrap(), b"\x02");
        assert_eq!(&*tor_unescape(b"\"\\002\"").unwrap(), b"\x02");

        assert_eq!(&*tor_unescape(b"\"\\40\"").unwrap(), b" ");
        assert_eq!(&*tor_unescape(b"\"\\040\"").unwrap(), b" ");

        assert_eq!(&*tor_unescape(b"\"\\40test\"").unwrap(), b" test");
        assert_eq!(&*tor_unescape(b"\"\\040test\"").unwrap(), b" test");
        assert_eq!(&*tor_unescape(b"\"\\40test\\0\"").unwrap(), b" test\0");
    }

    #[test]
    fn invalid_unicode() {
        // Raw invalid sequence, without quotes
        assert_eq!(*tor_unescape(b"\xF5").unwrap(), [0xF5u8]);
        // Raw invalid sequence, with quotes
        assert_eq!(*tor_unescape(b"\"\xF5\"").unwrap(), [0xF5u8]);
        // Escaped invalid values, we will unescape but keep them as they are.
        assert_eq!(*tor_unescape(b"\"\\xF5\"").unwrap(), [0xF5u8]);
        assert_eq!(*tor_unescape(b"\"\\365\"").unwrap(), [0xF5u8]);
    }

    #[test]
    fn unquoted_space() {
        assert_eq!(
            tor_unescape(b"test test").unwrap_err(),
            UnescapeError::SpaceInUnquotedString
        );
    }

    #[test]
    fn unterminated() {
        assert_eq!(
            tor_unescape(b"\"test").unwrap_err(),
            UnescapeError::Unterminated
        );
        assert_eq!(
            tor_unescape(b"\"test\\\"").unwrap_err(),
            UnescapeError::Unterminated
        );
        assert_eq!(
            tor_unescape(b"\"test\n\"").unwrap_err(),
            UnescapeError::Unterminated
        );
        assert_eq!(
            tor_unescape(b"\"test\n").unwrap_err(),
            UnescapeError::Unterminated
        );
        assert_eq!(
            tor_unescape(b"\"test\0\"").unwrap_err(),
            UnescapeError::Unterminated
        );
        assert_eq!(
            tor_unescape(b"\"test\0").unwrap_err(),
            UnescapeError::Unterminated
        );

        // Not having a final quote shortcircuits other errors.
        assert_eq!(
            tor_unescape(b"\"test \" test").unwrap_err(),
            UnescapeError::Unterminated
        );
    }

    #[test]
    fn unescaped_quote() {
        assert_eq!(
            tor_unescape(b"\"test \" test\"").unwrap_err(),
            UnescapeError::UnescapedQuote
        );
    }

    #[test]
    fn invalid_escape() {
        assert_eq!(
            tor_unescape(b"\"\\z\"").unwrap_err(),
            UnescapeError::InvalidEscape,
        );

        assert_eq!(
            tor_unescape(b"\"\\xy\"").unwrap_err(),
            UnescapeError::InvalidEscape,
        );
        assert_eq!(
            tor_unescape(b"\"\\XY\"").unwrap_err(),
            UnescapeError::InvalidEscape,
        );
        assert_eq!(
            tor_unescape(b"\"\\x1Y\"").unwrap_err(),
            UnescapeError::InvalidEscape,
        );
        assert_eq!(
            tor_unescape(b"\"\\x1\"").unwrap_err(),
            UnescapeError::InvalidEscape,
        );
        assert_eq!(
            tor_unescape(b"\"\\x\\xF5z\"").unwrap_err(),
            UnescapeError::InvalidEscape,
        );
    }

    #[test]
    fn octal_overflow() {
        assert_eq!(
            tor_unescape(b"\"\\777\"").unwrap_err(),
            UnescapeError::OctalOverflow,
        );
    }
}
