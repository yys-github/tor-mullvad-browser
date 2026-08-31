// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

mod escape;
mod unescape;

use escape::*;
use unescape::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(buf: &[u8]) {
        let mut escaped = String::new();
        tor_escape_into(buf, &mut escaped);
        assert_eq!(&*tor_unescape(escaped.as_bytes()).unwrap(), buf);
    }

    #[test]
    fn round_trip_simple() {
        round_trip(b"test");
    }

    #[test]
    fn round_trip_empty() {
        round_trip(b"");
    }

    #[test]
    fn round_trip_special_chars() {
        round_trip(b"'\"\\\r\n\t");
    }

    #[test]
    fn round_trip_non_utf8() {
        round_trip(b"\xF5\xFF\xFE");
    }

    #[test]
    fn round_trip_all_bytes() {
        // Every possible byte value, escaped then unescaped,
        // must come back unchanged.
        let all_bytes: Vec<u8> = (0..=255).collect();
        round_trip(&all_bytes);
    }
}
