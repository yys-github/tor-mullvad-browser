// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use tinystr::TinyAsciiStr;

use crate::ctor::ControllerError;

/// Parse the country codes as passed by the
/// `GETINFO ip-to-country/<ip-address>` command.
/// The function checks that the country code is composed by two letters, or it
/// is ??. Anything else will be rejected.
/// Cases will be preserved.
pub fn parse_country_code(line: &[u8]) -> Result<TinyAsciiStr<2>, ControllerError> {
    if line.len() != 2 {
        return Err(ControllerError::MalformedReply(format!(
            "expected two bytes (got {})",
            line.len(),
        )));
    }
    let code = TinyAsciiStr::try_from_utf8(line)
        .map_err(|e| ControllerError::MalformedReply(e.to_string()))?;
    if code != "??" && !code.is_ascii_alphabetic() {
        return Err(ControllerError::MalformedReply(String::from(
            "country codes must be 2 letters or be ??",
        )));
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    fn country_code() {
        assert_eq!(parse_country_code(b"IT").unwrap(), "IT");
    }

    #[test]
    fn unknown() {
        assert_eq!(parse_country_code(b"??").unwrap(), "??");
    }

    #[test]
    fn case_preserved() {
        assert_eq!(parse_country_code(b"aa").unwrap(), "aa");
        assert_eq!(parse_country_code(b"aB").unwrap(), "aB");
        assert_eq!(parse_country_code(b"Ba").unwrap(), "Ba");
        assert_eq!(parse_country_code(b"BB").unwrap(), "BB");
    }

    #[test]
    fn too_short() {
        assert_matches!(
            parse_country_code(b"I").unwrap_err(),
            ControllerError::MalformedReply(_)
        );
    }

    #[test]
    fn too_long() {
        assert_matches!(
            parse_country_code(b"ITA").unwrap_err(),
            ControllerError::MalformedReply(_)
        );
    }

    #[test]
    fn not_ascii() {
        assert_matches!(
            parse_country_code(b"\xF0\xF0").unwrap_err(),
            ControllerError::MalformedReply(_)
        );
    }

    #[test]
    fn empty() {
        assert_matches!(
            parse_country_code(b"").unwrap_err(),
            ControllerError::MalformedReply(_)
        );
    }

    #[test]
    fn not_alphabetic() {
        assert_matches!(
            parse_country_code(b"12").unwrap_err(),
            ControllerError::MalformedReply(_)
        );
        assert_matches!(
            parse_country_code(b"A2").unwrap_err(),
            ControllerError::MalformedReply(_)
        );
        assert_matches!(
            parse_country_code(b"1B").unwrap_err(),
            ControllerError::MalformedReply(_)
        );
        assert_matches!(
            parse_country_code(b"?c").unwrap_err(),
            ControllerError::MalformedReply(_)
        );
        assert_matches!(
            parse_country_code(b"d?").unwrap_err(),
            ControllerError::MalformedReply(_)
        );
    }
}
