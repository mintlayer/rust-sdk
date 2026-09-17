// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Shared mock-RPC harness for the integration tests.
//!
//! Every `tests/*.rs` binary that talks HTTP includes this module via
//! `mod common;`. Because each test binary compiles its own copy, and not
//! every binary uses every helper, dead-code analysis is silenced for the
//! module as a whole.

#![allow(dead_code)]

use httpmock::prelude::*;
use httpmock::{Mock, Then};
use serde_json::json;

/// Content-type header applied to every mocked JSON-RPC / REST response.
pub const JSON_HEADERS: [(&str, &str); 1] = [("content-type", "application/json")];

/// Full JSON-RPC 2.0 success envelope carrying the request's `id`.
pub fn rpc_ok(id: u64, result: serde_json::Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

/// JSON-RPC 2.0 success envelope without an `id` (id-validation tests).
pub fn rpc_ok_no_id(result: serde_json::Value) -> String {
    json!({ "jsonrpc": "2.0", "result": result }).to_string()
}

/// JSON-RPC 2.0 error envelope carrying the request's `id`.
pub fn rpc_error(id: u64, code: i64, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
    .to_string()
}

/// Answer a request with `status`, a JSON content-type header, and `body`.
pub fn respond(then: Then, status: u16, body: String) {
    let mut builder = then.status(status);
    for (name, value) in JSON_HEADERS {
        builder = builder.header(name, value);
    }
    builder.body(body);
}

/// Mock a `POST /` JSON-RPC call whose request body contains
/// `request_fragment`, answering with `body`.
pub fn mock_rpc(server: &MockServer, request_fragment: String, body: String) -> Mock<'_> {
    server.mock(move |when, then| {
        when.method(POST).path("/").body_contains(request_fragment);
        respond(then, 200, body);
    })
}
