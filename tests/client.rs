// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Tests for the top-level client and crate re-exports.

use std::time::Duration;

use httpmock::Then;
use httpmock::prelude::*;
use serde_json::json;

use mintlayer_sdk::Client;
use mintlayer_sdk::crypto::make_private_key;
use mintlayer_sdk::prelude::*;

fn respond(then: Then, body: serde_json::Value) {
    then.status(200)
        .header("content-type", "application/json")
        .body(body.to_string());
}

fn rpc_ok(id: u64, result: serde_json::Value) -> serde_json::Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn chainstate_info_result() -> serde_json::Value {
    json!({
        "best_block_height": 123456,
        "best_block_id":
            "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
        "best_block_timestamp": { "timestamp": 1_700_000_000 },
        "median_time": { "timestamp": 1_699_999_500 },
        "is_initial_block_download": false,
    })
}

#[test]
fn builder_creates_only_configured_clients() {
    let node_only = Client::builder().node_url("http://127.0.0.1:3030").build().unwrap();
    assert!(node_only.node.is_some());
    assert!(node_only.indexer.is_none());
    assert!(node_only.wallet.is_none());

    let empty = Client::builder().build().unwrap();
    assert!(empty.node.is_none());
    assert!(empty.indexer.is_none());
    assert!(empty.wallet.is_none());

    let all = Client::builder()
        .node_url("http://127.0.0.1:3030")
        .indexer_url("http://127.0.0.1:3000")
        .wallet_url("http://127.0.0.1:3034")
        .build()
        .unwrap();
    assert!(all.node.is_some());
    assert!(all.indexer.is_some());
    assert!(all.wallet.is_some());
}

#[tokio::test]
async fn umbrella_client_end_to_end_request() {
    let server = MockServer::start();
    let node_mock = server.mock(|when, then| {
        when.method(POST).path("/node").body_contains("\"method\":\"chainstate_info\"");
        respond(then, rpc_ok(1, chainstate_info_result()));
    });
    let wallet_mock = server.mock(|when, then| {
        when.method(POST).path("/wallet");
        respond(then, rpc_ok(1, json!(null)));
    });

    let client = Client::builder()
        .node_url(server.url("/node"))
        .wallet_url(server.url("/wallet"))
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    assert!(client.indexer.is_none());

    let info = client.node.as_ref().unwrap().chainstate_info().await.unwrap();
    assert_eq!(info.best_block_height, 123456);
    assert_eq!(
        info.best_block_id,
        "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"
    );
    assert_eq!(info.best_block_timestamp.timestamp, 1_700_000_000);
    assert_eq!(info.median_time.timestamp, 1_699_999_500);
    assert!(!info.is_initial_block_download);

    assert_eq!(node_mock.hits(), 1);
    assert_eq!(wallet_mock.hits(), 0);
}

#[tokio::test]
async fn basic_auth_only_affects_rpc_clients() {
    let server = MockServer::start();
    let node_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/node")
            .header("authorization", "Basic dXNlcjpwYXNz")
            .body_contains("\"method\":\"chainstate_info\"");
        respond(then, rpc_ok(1, chainstate_info_result()));
    });
    let indexer_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/chain/tip").matches(|request| {
            !request
                .headers
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        });
        respond(then, json!({ "block_height": 42000, "block_id": "0000ab" }));
    });

    let client = Client::builder()
        .node_url(server.url("/node"))
        .wallet_url(server.url("/wallet"))
        .indexer_url(server.url(""))
        .basic_auth("user", "pass")
        .build()
        .unwrap();

    let info = client.node.as_ref().unwrap().chainstate_info().await.unwrap();
    assert_eq!(info.best_block_height, 123456);

    let tip = client.indexer.as_ref().unwrap().tip().await.unwrap();
    assert_eq!(tip.block_height, 42000);
    assert_eq!(tip.block_id, "0000ab");

    assert_eq!(node_mock.hits(), 1);
    assert_eq!(indexer_mock.hits(), 1);
}

#[test]
fn prelude_reexports() {
    assert_eq!(Amount::from_atoms(1u128).into_atoms() as u128, 1);

    let label = match Network::Mainnet {
        Network::Mainnet => "mainnet",
        Network::Testnet => "testnet",
        Network::Regtest => "regtest",
        Network::Signet => "signet",
    };
    assert_eq!(label, "mainnet");

    let private_key = make_private_key();
    let encoded = private_key.encode();
    assert_eq!(encoded.len(), 33);
    let decoded = PrivateKey::decode_all(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.encode(), encoded);
}
