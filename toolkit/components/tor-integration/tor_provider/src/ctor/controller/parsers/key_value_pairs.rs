// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use lazy_static::lazy_static;
use regex::bytes::Regex;
use std::collections::HashMap;

use crate::ctor::controller::ControllerError;

lazy_static! {
    // Matching the final group will assure we reject quotes in any unquoted
    // string.
    static ref KEY_VALUE_PAIR_REGEX: Regex = Regex::new(r#"^ *([A-Za-z0-9_]+)=("(?:\\.|[^"])*"|[^ "]*)(?: +|$)"#)
        .expect("The regex is hardcoded, it should be good.");
}

/// Get multiple key-value pairs from a line.
///
/// Often the commands expect values to follow a certain format for the values.
/// Validation needs to be performed by the caller.
///
/// Also, usually keywords are defined with double quotes, therefore they should
/// be treated as case-insensitive.
/// This function compares the keyword it finds with the keywords from the list
/// in a case-insensitive fashion, and uses the list's string as a key.
///
/// All arguments are processed regardless of them being in the list, and the
/// order in allowed_keys does not influence how this function works.
/// In other words, this function does not validate the order in which the
/// key/value pairs appear in the line.
///
/// Repeated entries are silently tolerated, and the last matching element will
/// be assigned to the returned map. However, callers should not depend on this
/// behavior.
pub(super) fn parse_key_value_pairs<'a>(
    line: &'a [u8],
    allowed_keys: &[&'static str],
) -> Result<HashMap<&'static str, &'a [u8]>, ControllerError> {
    let mut values = HashMap::new();
    let mut pos = 0;
    while pos < line.len() {
        let Some(kv) = KEY_VALUE_PAIR_REGEX.captures(&line[pos..]) else {
            return Err(ControllerError::MalformedReply(format!(
                "invalid key/value pair: {}",
                String::from_utf8_lossy(&line[pos..])
            )));
        };
        pos += kv[0].len();

        let key = match allowed_keys
            .iter()
            .find(|k| kv[1].eq_ignore_ascii_case(k.as_bytes()))
        {
            Some(k) => *k,
            None => {
                continue;
            }
        };
        // Notice that kv[2] does not work here as it has explicitly the same
        // lifetime as the Captures, rather than the original buffer's.
        let value = kv.get(2).expect("Group 2 is compulsory.").as_bytes();
        values.insert(key, value);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_pair() {
        {
            let values = parse_key_value_pairs(b"KEY=Value", &["KEY"]).unwrap();
            assert_eq!(values.len(), 1);
            assert_eq!(values.get("KEY").unwrap(), b"Value");
        }
        {
            let values = parse_key_value_pairs(b"KEY=\"Value with quotes\"", &["KEY"]).unwrap();
            assert_eq!(values.len(), 1);
            assert_eq!(values.get("KEY").unwrap(), b"\"Value with quotes\"");
        }
        {
            let values = parse_key_value_pairs(b"KEY=\"Value\"", &["KEY"]).unwrap();
            assert_eq!(values.len(), 1);
            assert_eq!(values.get("KEY").unwrap(), b"\"Value\"");
        }
    }

    #[test]
    fn empty_value() {
        let values = parse_key_value_pairs(b"KEY=", &["KEY"]).unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values.get("KEY").unwrap(), b"");
    }

    #[test]
    fn quotes() {
        let values = parse_key_value_pairs(b"KEY=\"Value with \\\" esccape\"", &["KEY"]).unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values.get("KEY").unwrap(), b"\"Value with \\\" esccape\"");
    }

    #[test]
    fn test_multiple() {
        let values = parse_key_value_pairs(
            b"GOOD=yes QUOTED=\"Quoted value\" ANOTHER=1",
            &["GOOD", "NOT_FOUND", "QUOTED", "ANOTHER"],
        )
        .unwrap();
        assert_eq!(values.len(), 3);
        assert_eq!(values.get("GOOD").unwrap(), b"yes");
        assert_eq!(values.get("QUOTED").unwrap(), b"\"Quoted value\"");
        assert_eq!(values.get("ANOTHER").unwrap(), b"1");
        assert!(values.get("NOT_FOUND").is_none());
    }

    #[test]
    fn ignore_if_unallowed() {
        let values = parse_key_value_pairs(b"A=1 B=2 C=\"Quoted\"", &["A", "C"]).unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(values.get("A").unwrap(), b"1");
        assert!(values.get("B").is_none());
        assert_eq!(values.get("C").unwrap(), b"\"Quoted\"");
    }

    #[test]
    fn only_in_allow_list() {
        let values = parse_key_value_pairs(b"A=1 C=\"Quoted\"", &["A", "B", "C"]).unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(values.get("A").unwrap(), b"1");
        assert!(values.get("B").is_none());
        assert_eq!(values.get("C").unwrap(), b"\"Quoted\"");
    }

    #[test]
    fn case_insensitive_keys() {
        let values =
            parse_key_value_pairs(b"A=123 tESt=value KEY=val", &["a", "TesT", "kEy"]).unwrap();
        assert_eq!(values.len(), 3);
        assert_eq!(values.get("a").unwrap(), b"123");
        assert_eq!(values.get("TesT").unwrap(), b"value");
        assert_eq!(values.get("kEy").unwrap(), b"val");
    }

    #[test]
    fn missing_equal_sign() {
        assert!(parse_key_value_pairs(b"KEY", &["KEY"]).is_err());
        assert!(parse_key_value_pairs(
            b"VALUE=something MISSING COUNT=3 QUOTED=\"Quoted value\"",
            &["VALUE", "MISSING", "COUNT", "QUOTED"]
        )
        .is_err());
        assert!(parse_key_value_pairs(
            b"MISSING VALUE=something COUNT=3 QUOTED=\"Quoted value\"",
            &["VALUE", "MISSING", "COUNT", "QUOTED"]
        )
        .is_err());
        assert!(parse_key_value_pairs(
            b"VALUE=something COUNT=3 QUOTED=\"Quoted value\" MISSING",
            &["VALUE", "MISSING", "COUNT", "QUOTED"]
        )
        .is_err());
    }

    #[test]
    fn problematic_quotes() {
        assert!(parse_key_value_pairs(b"KEY=\"Unbalanced quote", &["KEY"]).is_err());
        assert!(parse_key_value_pairs(b"KEY=value\"", &["KEY"]).is_err());
        assert!(parse_key_value_pairs(b"KEY=val\"ue", &["KEY"]).is_err());
        assert!(parse_key_value_pairs(
            b"GOOD=yes INVALID=\"Unbalanced QUOTED=\"Quoted value\" ANOTHER=1",
            &["GOOD", "INVALID", "QUOTED", "ANOTHER"]
        )
        .is_err());
        assert!(parse_key_value_pairs(
            b"GOOD=yes QUOTED=\"Quoted value\" INVALID=\"Unbalanced ANOTHER=1",
            &["GOOD", "INVALID", "QUOTED", "ANOTHER"]
        )
        .is_err());
        assert!(parse_key_value_pairs(
            b"GOOD=yes QUOTED=\"Quoted value\" ANOTHER=1 INVALID=\"Unbalanced ",
            &["GOOD", "INVALID", "QUOTED", "ANOTHER"]
        )
        .is_err());
    }
}
