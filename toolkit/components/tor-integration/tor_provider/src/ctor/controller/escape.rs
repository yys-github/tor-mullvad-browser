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
    fn escaped() {
        assert_eq!(tor_escape("'\"\\\r\n\t"), "\"\\'\\\"\\\\\\r\\n\\t\"");
        assert_eq!(tor_escape("\0"), "\"\\x00\"");
        assert_eq!(tor_escape("\u{1F9C5}"), "\"\\xF0\\x9F\\xA7\\x85\"");
    }
}
