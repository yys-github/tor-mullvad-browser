// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use lazy_static::lazy_static;
use regex::bytes::{Regex, RegexBuilder};
use smallvec::SmallVec;

use super::parse_key_value_pairs;
use crate::ctor::controller::{types::*, ControllerError};

lazy_static! {
    // https://spec.torproject.org/control-spec/replies.html#CIRC
    static ref CIRCUIT_BUILT_REGEX: Regex = RegexBuilder::new(
        "^(?<CircuitID>[a-zA-Z0-9]{1,16}) BUILT (?<Path>[A-Za-z0-9,$~]+)(?: (?<Args>.*))?$",
    )
    .case_insensitive(true)
    .build()
    .expect("The regex is hardcoded, it should be good.");
    static ref NODE_REGEX: Regex =
        Regex::new("^\\$(?<Fingerprint>[0-9a-fA-F]{40})(?:~[a-zA-Z0-9]+)?$")
            .expect("The regex is hardcoded, it should be good.");
}

/// The keywords we are interested in parsing for circuit statuses.
const CIRC_KEYWORDS: &[&'static str] = &["CONFLUX_ID"];

/// Parse CIRCUIT BUILT events.
/// We parse lines that look like `<circuitID> BUILT <path> <args>`.
/// We ignore circuits that do not match it, so that it can be used also to
/// parse the output of `GETINFO circuit-status`, as long as every line is
/// passed individually.
/// However, if a CIRCUIT looks like a BUILT but has some errors, we might
/// return an error instead (e.g., if one of the nodes in the path has a wrong
/// fingerprint length, or if the keyword-value pairs are encoded in a wrong
/// format).
///
/// The only keyword argument we parse is the conflux ID.
pub fn parse_circuit_built(line: &[u8]) -> Result<Option<Circuit>, ControllerError> {
    let Some(circuit) = CIRCUIT_BUILT_REGEX.captures(line) else {
        // We might parse the status of all circuits together.
        // Therefore we do not want to error when we are dealing with a status
        // that is not "BUILT" but something else.
        // Also, our regex forces a path to be available, even though the spec
        // does not explicitly require it. However, we are already returning
        // None, so we do not need to handle that case separately.
        return Ok(None);
    };

    let id = CircuitID::new(&circuit["CircuitID"]).ok_or_else(|| {
        ControllerError::MalformedReply(format!(
            "invalid circuit id {}",
            String::from_utf8_lossy(&circuit["CircuitID"])
        ))
    })?;

    let path = circuit["Path"]
        .split(|b| *b == b',')
        .map(|node| {
            let node = NODE_REGEX.captures(node).ok_or_else(|| {
                ControllerError::MalformedReply(format!(
                    "Invalid path item: {}",
                    String::from_utf8_lossy(node)
                ))
            })?;
            let mut fingerprint = NodeFingerprint::default();
            hex::decode_to_slice(&node["Fingerprint"], &mut fingerprint)
                .map_err(|e| ControllerError::MalformedReply(e.to_string()))?;
            Ok(fingerprint)
        })
        .collect::<Result<SmallVec<_>, ControllerError>>()?;
    debug_assert!(
        !path.is_empty(),
        "The path in the regex was not empty, if we arrived here we must have a path item."
    );

    let conflux_id = circuit
        .name("Args")
        .map(|args| parse_key_value_pairs(args.as_bytes(), CIRC_KEYWORDS))
        .transpose()?
        .and_then(|args| {
            let mut id = ConfluxID::default();
            args.get("CONFLUX_ID")
                .and_then(|s| hex::decode_to_slice(s, &mut id).ok())?;
            Some(id)
        });

    Ok(Some(Circuit {
        id,
        path,
        conflux_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint_from_hex(hex: &str) -> NodeFingerprint {
        let mut fp = NodeFingerprint::default();
        hex::decode_to_slice(hex, &mut fp).unwrap();
        fp
    }

    fn conflux_id_from_hex(hex: &str) -> ConfluxID {
        let mut id = ConfluxID::default();
        hex::decode_to_slice(hex, &mut id).unwrap();
        id
    }

    #[test]
    fn parse_single_node_circuit() {
        let input = b"1 BUILT $0000000000000000000000000000000000000001";
        let circuit = parse_circuit_built(input).unwrap().unwrap();

        assert_eq!(circuit.id.as_str(), "1");
        assert_eq!(circuit.path.len(), 1);
        assert_eq!(
            circuit.path[0],
            fingerprint_from_hex("0000000000000000000000000000000000000001")
        );
        assert_eq!(circuit.conflux_id, None);
    }

    #[test]
    fn parse_three_node_circuit() {
        let input = b"abc123 BUILT $aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,$bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,$cccccccccccccccccccccccccccccccccccccccc";
        let circuit = parse_circuit_built(input).unwrap().unwrap();

        assert_eq!(circuit.id.as_str(), "abc123");
        assert_eq!(circuit.path.len(), 3);
        assert_eq!(
            circuit.path[0],
            fingerprint_from_hex("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert_eq!(
            circuit.path[1],
            fingerprint_from_hex("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        );
        assert_eq!(
            circuit.path[2],
            fingerprint_from_hex("cccccccccccccccccccccccccccccccccccccccc")
        );
    }

    #[test]
    fn parse_with_conflux_id() {
        let input = b"qwerty999 BUILT $1111111111111111111111111111111111111111 CONFLUX_ID=0102030405060708090a0b0c0d0e0f10";
        let circuit = parse_circuit_built(input).unwrap().unwrap();

        assert_eq!(circuit.id.as_str(), "qwerty999");
        assert_eq!(circuit.path.len(), 1);
        assert_eq!(
            circuit.conflux_id.unwrap(),
            conflux_id_from_hex("0102030405060708090a0b0c0d0e0f10")
        );
    }

    #[test]
    fn parse_with_nicknames() {
        let input = b"1234 BUILT $1111111111111111111111111111111111111111~relay1,$2222222222222222222222222222222222222222~relay2,$3333333333333333333333333333333333333333~relay3";
        let circuit = parse_circuit_built(input).unwrap().unwrap();

        assert_eq!(circuit.id.as_str(), "1234");
        assert_eq!(circuit.path.len(), 3);
        assert_eq!(
            circuit.path[0],
            fingerprint_from_hex("1111111111111111111111111111111111111111")
        );
        assert_eq!(
            circuit.path[1],
            fingerprint_from_hex("2222222222222222222222222222222222222222")
        );
        assert_eq!(
            circuit.path[2],
            fingerprint_from_hex("3333333333333333333333333333333333333333")
        );
    }

    #[test]
    fn parse_mixed_case() {
        let input = b"test BuiLt $AaBbCcDdEeFf0123456789AaBbCcDdEeFf012345 conflux_ID=0102030405060708090a0b0c0d0E0F10";
        let circuit = parse_circuit_built(input).unwrap().unwrap();

        assert_eq!(circuit.path.len(), 1);
        assert_eq!(
            circuit.path[0],
            fingerprint_from_hex("aabbccddeeff0123456789aabbccddeeff012345")
        );
        assert_eq!(
            circuit.conflux_id.unwrap(),
            conflux_id_from_hex("0102030405060708090a0b0c0d0e0f10")
        );
    }

    #[test]
    fn parse_multiple_args() {
        let input = b"ID1 built $1111111111111111111111111111111111111111 PURPOSE=GENERAL CONFLUX_ID=aabbccddeeff00112233445566778899";
        let circuit = parse_circuit_built(input).unwrap().unwrap();

        assert_eq!(circuit.id.as_str(), "ID1");
        assert_eq!(
            circuit.conflux_id.unwrap(),
            conflux_id_from_hex("aabbccddeeff00112233445566778899")
        );
    }

    #[test]
    fn long_circuit() {
        let input = b"id1 BUILT $1111111111111111111111111111111111111111,$2222222222222222222222222222222222222222,$3333333333333333333333333333333333333333,$4444444444444444444444444444444444444444";
        let circuit = parse_circuit_built(input).unwrap().unwrap();
        assert_eq!(circuit.path.len(), 4);
    }

    #[test]
    fn not_built() {
        {
            let input = b"1000 EXTENDED $1111111111111111111111111111111111111111,$2222222222222222222222222222222222222222";
            assert!(parse_circuit_built(input).unwrap().is_none());
        }
        {
            let input = b"2000 CLOSED";
            assert!(parse_circuit_built(input).unwrap().is_none());
        }
        {
            let input = b"3000 GUARD_WAIT";
            assert!(parse_circuit_built(input).unwrap().is_none());
        }
    }

    #[test]
    fn missing_path() {
        let input = b"circuit1 BUILT";
        assert!(parse_circuit_built(input).unwrap().is_none());
    }

    #[test]
    fn missing_circuit_id() {
        assert!(
            parse_circuit_built(b"BUILT $1111111111111111111111111111111111111111")
                .unwrap()
                .is_none()
        );
        assert!(
            parse_circuit_built(b" BUILT $1111111111111111111111111111111111111111")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn missing_status_keyword() {
        let input = b"circuit1 $1111111111111111111111111111111111111111";
        assert!(parse_circuit_built(input).unwrap().is_none());
    }

    #[test]
    fn invalid_circuit_id() {
        {
            // CircuitID must be alphanumeric only
            let input = b"circuit+1 BUILT $1111111111111111111111111111111111111111";
            assert!(parse_circuit_built(input).unwrap().is_none());
        }
        {
            // CircuitID max length is 16
            let input = b"12345678901234567 BUILT $1111111111111111111111111111111111111111";
            assert!(parse_circuit_built(input).unwrap().is_none());
        }
    }

    #[test]
    fn invalid_fingerprint() {
        assert!(parse_circuit_built(b"circuit1 BUILT $123456").is_err());
        assert!(parse_circuit_built(
            b"circuit1 BUILT $11111111111111111111111111111111111111112222"
        )
        .is_err());
        assert!(
            parse_circuit_built(b"circuit1 BUILT $ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ")
                .is_err()
        );
    }

    #[test]
    fn partially_invalid_path() {
        assert!(parse_circuit_built(b"1 BUILT $1111111111111111111111111111111111111111,$222222222222222222222222222222222222222").is_err());
        assert!(parse_circuit_built(b"2 BUILT $1111111111111111111111111111111111111111,junk,$2222222222222222222222222222222222222222").is_err());
        assert!(parse_circuit_built(b"3 BUILT BUILT $1111111111111111111111111111111111111111$2222222222222222222222222222222222222222").is_err());
        assert!(
            parse_circuit_built(b"4 BUILT $1111111111111111111111111111111111111111,").is_err()
        );
    }

    #[test]
    fn invalid_conflux_id() {
        // Wrong conflux ID is not a fatal error.
        {
            let input =
            b"circuit1 BUILT $1111111111111111111111111111111111111111 CONFLUX_ID=GGGGGGGGGGGGGGGG";
            let result = parse_circuit_built(input).unwrap().unwrap();
            assert!(result.conflux_id.is_none());
        }
        {
            let input = b"circuit1 BUILT $1111111111111111111111111111111111111111 CONFLUX_ID=0102";
            let result = parse_circuit_built(input).unwrap().unwrap();
            assert!(result.conflux_id.is_none());
        }
        {
            let input = b"circuit1 BUILT $1111111111111111111111111111111111111111 CONFLUX_ID=000000001111111122222222";
            let result = parse_circuit_built(input).unwrap().unwrap();
            assert!(result.conflux_id.is_none());
        }
    }

    #[test]
    fn invalid_path() {
        let input = b"circuit1 BUILT $";
        assert!(parse_circuit_built(input).is_err());
    }

    #[test]
    fn missing_dollar_sign() {
        let input = b"circuit1 BUILT 1111111111111111111111111111111111111111";
        assert!(parse_circuit_built(input).is_err());
    }
}
