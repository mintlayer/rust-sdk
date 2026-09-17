// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wire-format tests for the node daemon client, mirroring the go-sdk
//! `node/client_test.go` suite.

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{mock_rpc, respond, rpc_error, rpc_ok, rpc_ok_no_id};
use httpmock::prelude::*;
use serde_json::json;

use mintlayer_sdk::node::{
    Amount, BannedPeer, BannedTime, Client, Error, FeeRate, FeeRatePoint, Outpoint,
    OutpointSourceId, TrustPolicy,
};

#[tokio::test]
async fn chainstate_info_roundtrip() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/")
            .body_contains("\"jsonrpc\":\"2.0\"")
            .body_contains("\"method\":\"chainstate_info\"")
            .body_contains("\"params\":{}");
        respond(
            then,
            200,
            rpc_ok(
                1,
                json!({
                    "best_block_height": 123456,
                    "best_block_id":
                        "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f",
                    "best_block_timestamp": { "timestamp": 1_700_000_000 },
                    "median_time": { "timestamp": 1_699_999_500 },
                    "is_initial_block_download": false,
                }),
            ),
        );
    });

    let client = Client::new(server.url("/"));
    let info = client.chainstate_info().await.unwrap();

    assert_eq!(info.best_block_height, 123456);
    assert_eq!(
        info.best_block_id,
        "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"
    );
    assert_eq!(info.best_block_timestamp.timestamp, 1_700_000_000);
    assert_eq!(info.median_time.timestamp, 1_699_999_500);
    assert!(!info.is_initial_block_download);
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn scalar_methods_and_id_counter() {
    let server = MockServer::start();
    let id_mock = mock_rpc(&server, "\"id\":1,".to_string(), rpc_ok(1, json!("0000ff")));
    let height_mock = mock_rpc(&server, "\"id\":2,".to_string(), rpc_ok(2, json!(7)));
    let tx_mock = mock_rpc(&server, "\"id\":3,".to_string(), rpc_ok(3, json!(true)));
    let memory_mock = mock_rpc(&server, "\"id\":4,".to_string(), rpc_ok(4, json!(2048)));

    let client = Client::new(server.url("/"));
    assert_eq!(client.best_block_id().await.unwrap(), "0000ff");
    assert_eq!(client.best_block_height().await.unwrap(), 7);
    assert!(client.contains_tx("aabb1234").await.unwrap());
    assert_eq!(client.memory_usage().await.unwrap(), 2048);

    assert_eq!(id_mock.hits(), 1);
    assert_eq!(height_mock.hits(), 1);
    assert_eq!(tx_mock.hits(), 1);
    assert_eq!(memory_mock.hits(), 1);
}

#[tokio::test]
async fn optional_results_decode_null_as_none() {
    let server = MockServer::start();
    // Each response must carry the id of its request; the calls below run
    // sequentially, so request ids 1, 2, 3 map 1:1 onto the three mocks.
    let block_mock = mock_rpc(
        &server,
        "\"id\":1,\"method\":\"chainstate_block_id_at_height\"".to_string(),
        rpc_ok(1, json!(null)),
    );
    let pool_mock = mock_rpc(
        &server,
        "\"id\":2,\"method\":\"chainstate_stake_pool_balance\"".to_string(),
        rpc_ok(2, json!(null)),
    );
    let token_mock = mock_rpc(
        &server,
        "\"id\":3,\"method\":\"chainstate_token_info\"".to_string(),
        rpc_ok(3, json!(null)),
    );

    let client = Client::new(server.url("/"));
    assert_eq!(client.block_id_at_height(999_999).await.unwrap(), None);
    assert_eq!(client.stake_pool_balance("pool1abc").await.unwrap(), None);
    assert_eq!(client.token_info("token1abc").await.unwrap(), None);
    assert_eq!(block_mock.hits(), 1);
    assert_eq!(pool_mock.hits(), 1);
    assert_eq!(token_mock.hits(), 1);
}

#[tokio::test]
async fn missing_response_id_is_rejected() {
    let server = MockServer::start();
    mock_rpc(&server, "\"jsonrpc\"".to_string(), rpc_ok_no_id(json!(42)));

    let client = Client::new(server.url("/"));
    match client.best_block_height().await {
        Err(Error::IdMismatch { expected, actual }) => {
            assert_eq!(expected, 1);
            assert_eq!(actual, serde_json::Value::Null);
        }
        other => panic!("expected IdMismatch, got: {other:?}"),
    }
}

#[tokio::test]
async fn rpc_error_is_surfaced() {
    let server = MockServer::start();
    mock_rpc(
        &server,
        "\"jsonrpc\"".to_string(),
        rpc_error(1, -32601, "Method not found"),
    );

    let client = Client::new(server.url("/"));
    let err = client.best_block_height().await.unwrap_err();
    let display = err.to_string();
    match err {
        Error::Rpc { code, message } => {
            assert_eq!(code, -32601);
            assert_eq!(message, "Method not found");
        }
        other => panic!("expected Error::Rpc, got {other:?}"),
    }
    assert_eq!(display, "RPC error -32601: Method not found");
}

#[tokio::test]
async fn basic_auth_header_is_sent() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/").header("authorization", "Basic dXNlcjpwYXNz");
        respond(then, 200, rpc_ok(1, json!(5)));
    });

    let client = Client::builder(server.url("/")).basic_auth("user", "pass").build().unwrap();
    assert_eq!(client.best_block_height().await.unwrap(), 5);
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn get_utxo_serializes_tagged_outpoint() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/").json_body_partial(
            json!({
                "method": "chainstate_get_utxo",
                "params": {
                    "outpoint": {
                        "source_id": {
                            "type": "Transaction",
                            "content": { "tx_id": "aabb" },
                        },
                        "index": 3,
                    },
                },
            })
            .to_string(),
        );
        respond(then, 200, rpc_ok(1, json!({ "type": "Transfer" })));
    });

    let client = Client::new(server.url("/"));
    let outpoint = Outpoint {
        source_id: OutpointSourceId::Transaction {
            tx_id: "aabb".to_string(),
        },
        index: 3,
    };
    let utxo = client.utxo(&outpoint).await.unwrap().unwrap();
    assert_eq!(utxo, json!({ "type": "Transfer" }));
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn fee_rate_points_decode_tuple_wire() {
    let server = MockServer::start();
    mock_rpc(
        &server,
        "\"jsonrpc\"".to_string(),
        rpc_ok(
            1,
            json!([
                [1024, { "amount_per_kb": { "atoms": "500" } }],
                [2048, { "amount_per_kb": { "atoms": "750" } }],
            ]),
        ),
    );

    let client = Client::new(server.url("/"));
    let points = client.fee_rate_points().await.unwrap();
    assert_eq!(
        points,
        vec![
            FeeRatePoint {
                size: 1024,
                rate: FeeRate {
                    amount_per_kb: Amount::from_atoms(500),
                },
            },
            FeeRatePoint {
                size: 2048,
                rate: FeeRate {
                    amount_per_kb: Amount::from_atoms(750),
                },
            },
        ]
    );
}

#[tokio::test]
async fn banned_peers_decode_tuple_wire() {
    let server = MockServer::start();
    mock_rpc(
        &server,
        "\"jsonrpc\"".to_string(),
        rpc_ok(1, json!([["1.2.3.4", { "time": [1_700_000_000, 123] }]])),
    );

    let client = Client::new(server.url("/"));
    assert_eq!(
        client.list_banned().await.unwrap(),
        vec![BannedPeer {
            address: "1.2.3.4".to_string(),
            ban_time: BannedTime {
                seconds: 1_700_000_000,
                nanos: 123,
            },
        }]
    );
}

#[tokio::test]
async fn order_nonce_accepts_string_number_and_null() {
    let server = MockServer::start();
    let string_nonce_order = json!({
        "conclude_key": "02a1b2c3",
        "initially_asked": { "type": "Coin", "content": { "atoms": "1000" } },
        "initially_given": { "type": "Token", "content": "ttoken1abc" },
        "ask_balance": { "atoms": "1000" },
        "give_balance": { "atoms": "250" },
        "nonce": "5",
        "is_frozen": false,
    });
    let null_nonce_order = json!({
        "conclude_key": "02a1b2c3",
        "initially_asked": { "type": "Coin", "content": { "atoms": "1000" } },
        "initially_given": { "type": "Token", "content": "ttoken1abc" },
        "ask_balance": { "atoms": "1000" },
        "give_balance": { "atoms": "250" },
        "nonce": null,
        "is_frozen": false,
    });
    let string_mock = mock_rpc(
        &server,
        "\"id\":1,".to_string(),
        rpc_ok(1, string_nonce_order),
    );
    let null_mock = mock_rpc(
        &server,
        "\"id\":2,".to_string(),
        rpc_ok(2, null_nonce_order),
    );

    let client = Client::new(server.url("/"));
    let order = client.order_info("order1").await.unwrap().unwrap();
    assert_eq!(order.nonce, Some(5));
    let order = client.order_info("order1").await.unwrap().unwrap();
    assert_eq!(order.nonce, None);
    assert_eq!(order.ask_balance, Amount::from_atoms(1000));
    assert_eq!(order.give_balance, Amount::from_atoms(250));

    assert_eq!(string_mock.hits(), 1);
    assert_eq!(null_mock.hits(), 1);
}

#[tokio::test]
async fn submit_transaction_sends_trust_policy() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/").json_body_partial(
            json!({
                "method": "mempool_submit_transaction",
                "params": {
                    "tx": "deadbeef",
                    "options": { "trust_policy": "Untrusted" },
                },
            })
            .to_string(),
        );
        respond(then, 200, rpc_ok(1, json!(null)));
    });

    let client = Client::new(server.url("/"));
    client.submit_transaction("deadbeef", TrustPolicy::Untrusted).await.unwrap();
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn broadcast_transaction_uses_p2p_method() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/").matches(|req| {
            let body =
                std::str::from_utf8(req.body.as_deref().unwrap_or_default()).unwrap_or_default();
            body.contains("\"method\":\"p2p_submit_transaction\"")
                && body.contains("\"tx\":\"deadbeef\"")
                && body.contains("\"trust_policy\":\"Trusted\"")
                && !body.contains("\"method\":\"mempool_submit_transaction\"")
        });
        respond(then, 200, rpc_ok(1, json!(null)));
    });

    let client = Client::new(server.url("/"));
    client.broadcast_transaction("deadbeef", TrustPolicy::Trusted).await.unwrap();
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn connected_peers_and_mempool_tx_decode() {
    let server = MockServer::start();
    let peers_mock = mock_rpc(
        &server,
        "\"method\":\"p2p_get_connected_peers\"".to_string(),
        rpc_ok(
            1,
            json!([{
                "peer_id": 7,
                "address": "10.0.0.1:9333",
                "peer_role": "OutboundFullRelay",
                "ban_score": 0,
                "user_agent": "mintlayer/1.3.0",
                "software_version": "1.3.0",
                "ping_wait": null,
                "ping_last": 42,
                "ping_min": 17,
                "last_tip_block_time": 1_700_000_000,
            }]),
        ),
    );
    let tx_mock = mock_rpc(
        &server,
        "\"method\":\"mempool_get_transaction\"".to_string(),
        rpc_ok(
            2,
            json!({ "id": "aabb", "status": "InMempool", "transaction": "deadbeef" }),
        ),
    );

    let client = Client::new(server.url("/"));
    let peers = client.connected_peers().await.unwrap();
    assert_eq!(peers.len(), 1);
    let peer = &peers[0];
    assert_eq!(peer.peer_id, 7);
    assert_eq!(peer.address, "10.0.0.1:9333");
    assert_eq!(peer.peer_role, "OutboundFullRelay");
    assert_eq!(peer.ban_score, 0);
    assert_eq!(peer.user_agent, "mintlayer/1.3.0");
    assert_eq!(peer.software_version, "1.3.0");
    assert_eq!(peer.ping_wait, None);
    assert_eq!(peer.ping_last, Some(42));
    assert_eq!(peer.ping_min, Some(17));
    assert_eq!(peer.last_tip_block_time, Some(1_700_000_000));

    let tx = client.transaction("aabb").await.unwrap().expect("tx is in the mempool");
    assert_eq!(tx.id, "aabb");
    assert_eq!(tx.status, "InMempool");
    assert_eq!(tx.transaction, "deadbeef");

    assert_eq!(peers_mock.hits(), 1);
    assert_eq!(tx_mock.hits(), 1);
}

#[tokio::test]
async fn ban_serializes_duration_as_tuple() {
    let server = MockServer::start();
    let mock = mock_rpc(
        &server,
        "\"address\":\"1.2.3.4\",\"duration\":[3600,500000000]".to_string(),
        rpc_ok(1, json!(null)),
    );

    let client = Client::new(server.url("/"));
    client.ban("1.2.3.4", Duration::from_millis(3_600_500)).await.unwrap();
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn debug_output_redacts_basic_auth() {
    let server = MockServer::start();
    let builder = Client::builder(server.url("/")).basic_auth("secretuser", "secretpass");
    let builder_debug = format!("{builder:?}");
    let client = builder.build().unwrap();
    let client_debug = format!("{client:?}");
    for output in [builder_debug, client_debug] {
        assert!(!output.contains("secretuser"));
        assert!(!output.contains("secretpass"));
        assert!(output.contains("***"));
    }
}

#[tokio::test]
async fn oversized_responses_are_rejected() {
    let server = MockServer::start();
    // A real oversized body: httpmock derives the content-length header from
    // the body length (65 MiB + 10 > the 64 MiB cap), so the transport's
    // content-length pre-check rejects the response before it is read.
    let oversized = "0".repeat(65 * 1024 * 1024 + 10);
    server.mock(|when, then| {
        when.method(POST).path("/");
        respond(then, 200, oversized);
    });
    let client = Client::new(server.url("/"));
    match client.best_block_height().await {
        Err(Error::ResponseTooLarge { limit }) => {
            assert_eq!(limit, 64 * 1024 * 1024);
        }
        other => panic!("expected ResponseTooLarge, got: {other:?}"),
    }
}

#[tokio::test]
async fn concurrent_calls_use_unique_ids() {
    let server = MockServer::start();
    let mut mocks = Vec::new();
    for n in 1..=10u64 {
        let matcher = format!("\"id\":{n},\"method\":\"chainstate_best_block_height\"");
        mocks.push(mock_rpc(&server, matcher, rpc_ok(n, json!(n))));
    }

    let client = Client::new(server.url("/"));
    let shared = Arc::new(client.clone());
    let mut handles = Vec::new();
    for _ in 0..10 {
        let client = Arc::clone(&shared);
        handles.push(tokio::spawn(async move {
            client.best_block_height().await.unwrap()
        }));
    }

    let mut heights = Vec::new();
    for handle in handles {
        heights.push(handle.await.unwrap());
    }
    heights.sort_unstable();
    assert_eq!(heights, (1u64..=10).collect::<Vec<u64>>());
    for mock in &mocks {
        assert_eq!(mock.hits(), 1);
    }
}
