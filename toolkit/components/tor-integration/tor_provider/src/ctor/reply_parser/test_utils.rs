// Licensed under the Apache License, Version 2.0,
// <http://apache.org/licenses/LICENSE-2.0> or the MIT license
// <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use bytes::Bytes;

use super::{
    line::{DetailReplyLine, EndReplyLine},
    reply::Reply,
};

pub(super) fn check_250_ok(reply: &Reply, has_details: bool) {
    assert_eq!(reply.has_details(), has_details);
    assert_eq!(
        reply.end_line(),
        &EndReplyLine {
            code: 250,
            line: Bytes::from_static(b"OK")
        }
    );
}

#[test]
fn test_check_250_ok_without_details() {
    let reply = Reply::new(
        Vec::new(),
        EndReplyLine {
            code: 250,
            line: Bytes::from_static(b"OK"),
        },
    )
    .unwrap();
    check_250_ok(&reply, false);
}

#[test]
fn test_check_250_ok_with_details() {
    let reply = Reply::new(
        vec![DetailReplyLine::MidReplyLine {
            code: 400,
            line: Bytes::from_static(b"Error"),
        }],
        EndReplyLine {
            code: 250,
            line: Bytes::from_static(b"OK"),
        },
    )
    .unwrap();
    check_250_ok(&reply, true);
}
