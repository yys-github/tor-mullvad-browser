// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::fmt::Write;

/// Adaptation of tor's esc_for_log (src/lib/log/escape.c).
///
/// However, we prefer the hex representation to the octal one.
pub fn tor_escape_into<T: AsRef<[u8]>>(buf: T, dest: &mut String) {
    dest.reserve(buf.as_ref().len() + 2);
    dest.push('"');
    for b in buf.as_ref() {
        match *b {
            b'\'' | b'"' | b'\\' => {
                dest.push('\\');
                dest.push(*b as char);
            }
            b'\n' => {
                dest.push_str("\\n");
            }
            b'\t' => {
                dest.push_str("\\t");
            }
            b'\r' => {
                dest.push_str("\\r");
            }
            0x20..=0x7E => {
                dest.push(*b as char);
            }
            _ => {
                write!(dest, "\\x{:02X}", b).expect("String::write_str always returns Ok(()).");
            }
        }
    }
    dest.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tor_escape<T: AsRef<[u8]>>(buf: T) -> String {
        let mut dest = String::new();
        tor_escape_into(buf, &mut dest);
        dest
    }

    #[test]
    fn simple() {
        assert_eq!(tor_escape("test"), "\"test\"");
    }

    #[test]
    fn empty() {
        assert_eq!(tor_escape(""), "\"\"");
    }

    #[test]
    fn escaped_special_chars() {
        assert_eq!(tor_escape("'"), "\"\\'\"");
        assert_eq!(tor_escape("\""), "\"\\\"\"");
        assert_eq!(tor_escape("\\"), "\"\\\\\"");
        assert_eq!(tor_escape("'\"\\\r\n\t"), "\"\\'\\\"\\\\\\r\\n\\t\"");
    }

    #[test]
    fn escaped_control_chars() {
        assert_eq!(tor_escape("\n"), "\"\\n\"");
        assert_eq!(tor_escape("\t"), "\"\\t\"");
        assert_eq!(tor_escape("\r"), "\"\\r\"");
    }

    #[test]
    fn ascii_printable_boundaries() {
        // 0x1F is just below the printable range and must be hex-escaped.
        assert_eq!(tor_escape(b"\x1F"), "\"\\x1F\"");
        // 0x20 (space) is the first byte of the printable range.
        assert_eq!(tor_escape(b"\x20"), "\" \"");
        // 0x7E ('~') is the last byte of the printable range.
        assert_eq!(tor_escape(b"\x7E"), "\"~\"");
        // 0x7F (DEL) is just above the printable range and must be hex-escaped.
        assert_eq!(tor_escape(b"\x7F"), "\"\\x7F\"");
    }

    #[test]
    fn hex_zero_padding() {
        assert_eq!(tor_escape("\0"), "\"\\x00\"");
        assert_eq!(tor_escape(b"\x0B"), "\"\\x0B\"");
        assert_eq!(tor_escape(b"\x01"), "\"\\x01\"");
    }

    #[test]
    fn hex_uppercase() {
        assert_eq!(tor_escape(b"\xAB"), "\"\\xAB\"");
        assert_eq!(tor_escape(b"\xFF"), "\"\\xFF\"");
    }

    #[test]
    fn non_utf8_bytes() {
        // Raw invalid UTF-8 bytes are escaped byte-by-byte, same as any
        // other non-printable byte.
        assert_eq!(tor_escape(b"\xF5"), "\"\\xF5\"");
        assert_eq!(tor_escape(b"\xFF\xFE"), "\"\\xFF\\xFE\"");
        assert_eq!(tor_escape(b"te\xF5st"), "\"te\\xF5st\"");
    }

    #[test]
    fn unicode() {
        assert_eq!(tor_escape("\u{1F9C5}"), "\"\\xF0\\x9F\\xA7\\x85\"");
    }
}
