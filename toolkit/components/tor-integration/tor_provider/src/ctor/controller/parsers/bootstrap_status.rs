// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use lazy_static::lazy_static;
use regex::bytes::{Regex, RegexBuilder};

use super::{parse_key_value_pairs, tor_unescape};
use crate::ctor::controller::{types::*, ControllerError};

lazy_static! {
    static ref STATUS_REGEX: Regex = RegexBuilder::new(r"^(NOTICE|WARN) BOOTSTRAP (.+)$")
        .case_insensitive(true)
        .build()
        .expect("The regex is hardcoded, it should be good.");
}

const STATUS_KEYWORDS: &[&'static str] =
    &["PROGRESS", "TAG", "SUMMARY", "RECOMMENDATION", "REASON"];

/// Parse a STATUS_CLIENT event.
///
/// We handle events that have this format:
/// NOTICE BOOTSTRAP PROGRESS=n TAG=keyword SUMMARY="Human readable string" REASON=keyword
/// WARN BOOTSTRAP PROGRESS=n TAG=keyword SUMMARY="Human readable string" REASON=keyword
/// WARN BOOTSTRAP PROGRESS=n TAG=keyword SUMMARY="Human readable string" REASON=keyword RECOMMENDATION=warn
pub fn parse_bootstrap_status(value: &[u8]) -> Result<BootstrapStatus, ControllerError> {
    let groups = STATUS_REGEX
        .captures(value)
        .ok_or_else(|| ControllerError::WrongFormat(String::from_utf8_lossy(value).into_owned()))?;

    let severity = groups[1].to_ascii_uppercase();
    let severity = match severity.as_slice() {
        b"NOTICE" => StatusSeverity::Notice,
        b"WARN" => StatusSeverity::Warn,
        _ => {
            // Notice: the regex should make this case impossible...
            return Err(ControllerError::MalformedReply(format!(
                "unknown severity: {}",
                String::from_utf8_lossy(&severity)
            )));
        }
    };

    let arguments = parse_key_value_pairs(
        groups
            .get(2)
            .ok_or_else(|| {
                ControllerError::MalformedReply(String::from(
                    "the bootstrap status does not contain the compulsory values",
                ))
            })?
            .as_bytes(),
        STATUS_KEYWORDS,
    )?;

    let progress = arguments
        .get("PROGRESS")
        .ok_or_else(|| ControllerError::KeyNotFound(String::from("progress")))
        .and_then(|v| str::from_utf8(v).map_err(|e| ControllerError::MalformedReply(e.to_string())))
        .and_then(|v| {
            v.parse().map_err(|e| {
                ControllerError::MalformedReply(format!("progress is not an integer: {}", e))
            })
        })?;
    if progress > 100 {
        return Err(ControllerError::MalformedReply(format!(
            "progress exceeds 100: {}",
            progress
        )));
    }

    // For robustness, we do not check this value.
    let tag = String::from_utf8_lossy(
        arguments
            .get("TAG")
            .ok_or_else(|| ControllerError::KeyNotFound(String::from("tag")))?,
    )
    .into_owned();
    let summary = String::from_utf8_lossy(
        &tor_unescape(
            arguments
                .get("SUMMARY")
                .ok_or_else(|| ControllerError::KeyNotFound(String::from("summary")))?,
        )
        .map_err(|e| ControllerError::MalformedReply(e.to_string()))?,
    )
    .into_owned();
    let reason = arguments
        .get("REASON")
        .map(|r| String::from_utf8_lossy(r).into_owned());
    let should_warn = arguments
        .get("RECOMMENDATION")
        // Recommendation can be either "warn" or "ignore".
        // We chose to match warn as we want to be sure before issuing a warn,
        // as connection assist will interrupt the bootstrap.
        .map_or(false, |r| r.eq_ignore_ascii_case(b"warn"));

    Ok(BootstrapStatus {
        severity,
        progress,
        tag,
        summary,
        reason,
        should_warn,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::assert_matches;

    #[test]
    fn without_recommendation() {
        let line = b"NOTICE BOOTSTRAP PROGRESS=10 TAG=conn_done SUMMARY=\"Connected to a relay\"";
        let status = parse_bootstrap_status(line).unwrap();
        assert_eq!(status.severity, StatusSeverity::Notice);
        assert_eq!(status.progress, 10);
        assert_eq!(status.tag, "conn_done");
        assert_eq!(status.summary, "Connected to a relay");
        assert!(status.reason.is_none());
        assert!(!status.should_warn);
    }

    #[test]
    fn with_recommendation() {
        const PREFIX: &[u8] =
            b"WARN BOOTSTRAP PROGRESS=14 TAG=handshake SUMMARY=\"Handshaking with a relay\" ";
        {
            let line = vec![PREFIX, b"RECOMMENDATION=warn"].concat();
            let status = parse_bootstrap_status(&line).unwrap();
            assert!(status.should_warn);
        }
        {
            let line = vec![PREFIX, b"RECOMMENDATION=ignore"].concat();
            let status = parse_bootstrap_status(&line).unwrap();
            assert!(!status.should_warn);
        }
        {
            let line = vec![PREFIX, b"RECOMMENDATION=invalid"].concat();
            let status = parse_bootstrap_status(&line).unwrap();
            assert!(!status.should_warn);
        }
        {
            let line = vec![PREFIX, b"RECOMMENDATION="].concat();
            let status = parse_bootstrap_status(&line).unwrap();
            assert!(!status.should_warn);
        }
    }

    #[test]
    fn with_reason() {
        let line = b"WARN BOOTSTRAP PROGRESS=30 TAG=conn SUMMARY=\"Connecting to a relay\" REASON=CLOCK_SKEW";
        let status = parse_bootstrap_status(&line[..]).unwrap();
        assert_eq!(status.severity, StatusSeverity::Warn);
        assert_eq!(status.progress, 30);
        assert_eq!(status.tag, "conn");
        assert_eq!(status.summary, "Connecting to a relay");
        assert_eq!(status.reason.unwrap(), "CLOCK_SKEW");
        assert!(!status.should_warn);
    }

    #[test]
    fn case_insensitive() {
        let line = b"wARn Bootstrap pRoGrEsS=10 tAg=conn_DONE sUmmaRY=\"connected TO a ReLaY\" reCOmmeNdATIOn=waRn";
        let status = parse_bootstrap_status(line).unwrap();
        assert_eq!(status.severity, StatusSeverity::Warn);
        assert_eq!(status.progress, 10);
        assert_eq!(status.tag, "conn_DONE");
        assert_eq!(status.summary, "connected TO a ReLaY");
        assert!(status.should_warn);
    }

    #[test]
    fn invalid_severity() {
        let line = b"info bootstrap PROGRESS=10 TAG=test SUMMARY=\"Test\"";
        assert_matches!(
            parse_bootstrap_status(line).unwrap_err(),
            ControllerError::WrongFormat(_)
        );
    }

    #[test]
    fn invalid_progress() {
        // Sanity tests.
        assert!(
            parse_bootstrap_status(b"NOTICE bootstrap PROGRESS=0 TAG=test SUMMARY=\"Test\"")
                .is_ok()
        );
        assert!(
            parse_bootstrap_status(b"NOTICE bootstrap PROGRESS=1 TAG=test SUMMARY=\"Test\"")
                .is_ok()
        );
        assert!(
            parse_bootstrap_status(b"NOTICE bootstrap PROGRESS=10 TAG=test SUMMARY=\"Test\"")
                .is_ok()
        );
        assert!(
            parse_bootstrap_status(b"NOTICE bootstrap PROGRESS=99 TAG=test SUMMARY=\"Test\"")
                .is_ok()
        );
        assert!(
            parse_bootstrap_status(b"NOTICE bootstrap PROGRESS=100 TAG=test SUMMARY=\"Test\"")
                .is_ok()
        );

        // Invalid lines.
        assert_matches!(
            parse_bootstrap_status(b"NOTICE bootstrap PROGRESS=aaa TAG=test SUMMARY=\"Test\"")
                .unwrap_err(),
            ControllerError::MalformedReply(_)
        );
        assert_matches!(
            parse_bootstrap_status(b"NOTICE bootstrap PROGRESS=101 TAG=test SUMMARY=\"Test\"")
                .unwrap_err(),
            ControllerError::MalformedReply(_)
        );
    }

    #[test]
    fn missing_args() {
        assert_matches!(
            parse_bootstrap_status(b"NOTICE bootstrap TAG=test SUMMARY=\"Test\"").unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
        assert_matches!(
            parse_bootstrap_status(b"NOTICE bootstrap PROGRESS=3 SUMMARY=\"Test\"").unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
        assert_matches!(
            parse_bootstrap_status(b"NOTICE bootstrap PROGRESS=3 TAG=test").unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
        assert_matches!(
            parse_bootstrap_status(b"NOTICE bootstrap PROGRESS=3").unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
        assert_matches!(
            parse_bootstrap_status(b"NOTICE bootstrap TAG=test").unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
        assert_matches!(
            parse_bootstrap_status(b"NOTICE bootstrap SUMMARY=\"Test\"").unwrap_err(),
            ControllerError::KeyNotFound(_)
        );
    }

    #[test]
    fn reject_garbage() {
        assert_matches!(
            parse_bootstrap_status(b"UNALLOWED_PREFIX NOTICE BOOTSTRAP PROGRESS=10 TAG=conn_done SUMMARY=\"Connected to a relay\"").unwrap_err(),
            ControllerError::WrongFormat(_)
        );
        assert!(parse_bootstrap_status(b"NOTICE BOOTSTRAP PROGRESS=10 TAG=conn_done SUMMARY=\"Connected to a relay\" UNKNOWN_ARG=\"value\"").is_ok());
        assert!(parse_bootstrap_status(b"NOTICE BOOTSTRAP PROGRESS=10 UNKNOWN_ARG=\"value\" TAG=conn_done SUMMARY=\"Connected to a relay\"").is_ok());
        assert!(parse_bootstrap_status(b"NOTICE BOOTSTRAP PROGRESS=10 TAG=conn_done SUMMARY=\"Connected to a relay\" UNKNOWN_ARG=").is_ok());
        assert!(parse_bootstrap_status(b"NOTICE BOOTSTRAP PROGRESS=10 UNKNOWN_ARG= TAG=conn_done SUMMARY=\"Connected to a relay\"").is_ok());
        // Arguments that are not key=value pair (e.g., they are missing the
        // equal sign) are rejected.
        assert!(parse_bootstrap_status(b"NOTICE BOOTSTRAP INVALID_ARG PROGRESS=10 TAG=conn_done SUMMARY=\"Connected to a relay\"").is_err());
        assert!(parse_bootstrap_status(b"NOTICE BOOTSTRAP PROGRESS=10 INVALID_ARG TAG=conn_done SUMMARY=\"Connected to a relay\"").is_err());
        assert!(parse_bootstrap_status(b"NOTICE BOOTSTRAP PROGRESS=10 TAG=conn_done SUMMARY=\"Connected to a relay\" INVALID_ARG").is_err());
    }
}
