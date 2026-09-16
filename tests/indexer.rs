// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wire-format tests for the indexer client, mirroring the go-sdk
//! `indexer/client_test.go` suite.

use httpmock::Then;
use httpmock::prelude::*;
use serde_json::json;

use mintlayer_sdk::indexer::{
    Amount, Client, Error, PageOpts, PerThousand, PoolListOpts, PoolSort, Uint64, Utxo,
    UtxoOutpoint,
};

fn respond(then: Then, body: serde_json::Value) {
    then.status(200)
        .header("content-type", "application/json")
        .body(body.to_string());
}

#[tokio::test]
async fn tip_and_height_paths() {
    let server = MockServer::start();
    let tip_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/chain/tip");
        respond(then, json!({"block_height": 42000, "block_id": "0000ab"}));
    });
    let height_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/chain/1337");
        then.status(200)
            .header("content-type", "application/json")
            .body(json!("0000cd").to_string());
    });

    let client = Client::new(server.url(""));
    let tip = client.tip().await.unwrap();
    assert_eq!(tip.block_height, 42000);
    assert_eq!(tip.block_id, "0000ab");
    let block_id = client.block_id_at_height(1337).await.unwrap();
    assert_eq!(block_id, "0000cd");

    tip_mock.assert();
    height_mock.assert();
}

#[tokio::test]
async fn block_endpoints() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/block/0000aa");
        respond(
            then,
            json!({
                "height": 42000,
                "header": {
                    "previous_block_id": "0000aa00",
                    "timestamp": {"timestamp": 1700000000},
                    "merkle_root": "aabb",
                    "witness_merkle_root": "ccdd",
                    "consensus_data": {}
                },
                "body": {
                    "reward": [],
                    "transactions": []
                }
            }),
        );
    });

    let client = Client::new(server.url(""));
    let block = client.block("0000aa").await.unwrap();

    assert_eq!(block.height, 42000);
    assert_eq!(block.header.previous_block_id, "0000aa00");
    assert_eq!(block.header.timestamp.timestamp, 1_700_000_000);
    assert_eq!(block.header.merkle_root, "aabb");
    assert_eq!(block.header.witness_merkle_root, "ccdd");
    assert_eq!(block.header.consensus_data, json!({}));
    assert!(block.body.reward.is_empty());
    assert!(block.body.transactions.is_empty());
    mock.assert();
}

#[tokio::test]
async fn pagination_query_params() {
    let server = MockServer::start();
    let paged = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/transaction")
            .query_param("offset", "10")
            .query_param("items", "20");
        respond(then, json!([]));
    });
    let unpaginated = server.mock(|when, then| {
        when.method(GET).path("/api/v2/transaction").matches(|request| {
            let params = request.query_params.as_deref().unwrap_or(&[]);
            !params.iter().any(|(name, _)| *name == "offset" || *name == "items")
        });
        respond(then, json!([]));
    });

    let client = Client::new(server.url(""));
    let page = client
        .list_transactions(PageOpts {
            offset: 10,
            items: 20,
        })
        .await
        .unwrap();
    assert!(page.is_empty());
    let default_page = client.list_transactions(PageOpts::default()).await.unwrap();
    assert!(default_page.is_empty());

    paged.assert();
    unpaginated.assert();
}

#[tokio::test]
async fn pool_list_sort_param() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/pool")
            .query_param("sort", "by_pledge")
            .matches(|request| {
                let params = request.query_params.as_deref().unwrap_or(&[]);
                !params.iter().any(|(name, _)| *name == "offset" || *name == "items")
            });
        respond(
            then,
            json!([{
                "pool_id": "mpool1x",
                "decommission_destination": "mtct1x",
                "staker_balance": {"atoms": "1000", "decimal": "0.00000001"},
                "margin_ratio_per_thousand": "3.5%",
                "cost_per_block": {"atoms": "1000", "decimal": "0.00000001"},
                "vrf_public_key": "vrfpub1x",
                "delegations_balance": {"atoms": "2000", "decimal": "0.00000002"}
            }]),
        );
    });

    let client = Client::new(server.url(""));
    let pools = client
        .list_pools(PoolListOpts {
            offset: 0,
            items: 0,
            sort: Some(PoolSort::ByPledge),
        })
        .await
        .unwrap();

    assert_eq!(pools.len(), 1);
    let pool = &pools[0];
    assert_eq!(pool.pool_id, "mpool1x");
    assert_eq!(pool.decommission_destination, "mtct1x");
    assert_eq!(
        pool.staker_balance,
        Amount {
            atoms: 1000,
            decimal: "0.00000001".to_owned()
        }
    );
    assert_eq!(
        pool.cost_per_block,
        Amount {
            atoms: 1000,
            decimal: "0.00000001".to_owned()
        }
    );
    assert_eq!(pool.vrf_public_key, "vrfpub1x");
    assert_eq!(
        pool.delegations_balance,
        Amount {
            atoms: 2000,
            decimal: "0.00000002".to_owned()
        }
    );
    assert!((pool.margin_ratio_per_thousand.0 - 3.5).abs() < f64::EPSILON);
    assert_eq!(pool.margin_ratio_per_thousand, PerThousand(3.5));
    mock.assert();
}

#[tokio::test]
async fn pool_block_stats_query() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/pool/mpool1x/block-stats")
            .query_param("from", "1700000000")
            .query_param("to", "1700003600");
        respond(then, json!({"block_count": 42}));
    });

    let client = Client::new(server.url(""));
    let count = client.pool_block_stats("mpool1x", 1_700_000_000, 1_700_003_600).await.unwrap();

    assert_eq!(count, 42);
    mock.assert();
}

#[tokio::test]
async fn submit_transaction_posts_raw_hex() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v2/transaction")
            .header("content-type", "text/plain")
            .body("deadbeef");
        respond(then, json!({"tx_id": "aabb"}));
    });

    let client = Client::new(server.url(""));
    let tx_id = client.submit_transaction("deadbeef").await.unwrap();

    assert_eq!(tx_id, "aabb");
    mock.assert();
}

#[tokio::test]
async fn http_error_is_surfaced() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/order/unknown");
        then.status(404).body("not found");
    });

    let client = Client::new(server.url(""));
    let error = client.order("unknown").await.unwrap_err();

    match &error {
        Error::Http { status_code, body } => {
            assert_eq!(*status_code, 404);
            assert_eq!(body, "not found");
        }
        other => panic!("expected Error::Http, got {other:?}"),
    }
    assert_eq!(error.to_string(), "HTTP 404: not found");
    mock.assert();
}

#[tokio::test]
async fn lenient_number_fields() {
    let server = MockServer::start();
    let string_delegation = server.mock(|when, then| {
        when.method(GET).path("/api/v2/delegation/tdelg1x");
        respond(
            then,
            json!({
                "delegation_id": "tdelg1x",
                "pool_id": "mpool1x",
                "next_nonce": "7",
                "spend_destination": "mtct1x",
                "balance": {"atoms": "500", "decimal": "0.000000005"},
                "creation_block_height": "10000"
            }),
        );
    });
    let numeric_delegation = server.mock(|when, then| {
        when.method(GET).path("/api/v2/delegation/tdelg2x");
        respond(
            then,
            json!({
                "delegation_id": "tdelg2x",
                "pool_id": "mpool1x",
                "next_nonce": 9,
                "spend_destination": "mtct1x",
                "balance": {"atoms": 500, "decimal": "0.000000005"},
                "creation_block_height": 10000
            }),
        );
    });
    let order_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/order/mordr1x");
        respond(
            then,
            json!({
                "order_id": "mordr1x",
                "conclude_destination": "mtct1x",
                "give_currency": "Coin/ML",
                "initially_given": {"atoms": "1", "decimal": "0.00000000001"},
                "give_balance": {"atoms": "1", "decimal": "0.00000000001"},
                "ask_currency": "Coin/ML",
                "initially_asked": {"atoms": "1", "decimal": "0.00000000001"},
                "ask_balance": {"atoms": "1", "decimal": "0.00000000001"},
                "nonce": "5"
            }),
        );
    });

    let client = Client::new(server.url(""));

    let delegation = client.delegation("tdelg1x").await.unwrap();
    assert_eq!(delegation.next_nonce, Uint64(7));
    assert_eq!(delegation.creation_block_height, Uint64(10000));
    assert_eq!(
        delegation.balance,
        Amount {
            atoms: 500,
            decimal: "0.000000005".to_owned()
        }
    );

    let numeric = client.delegation("tdelg2x").await.unwrap();
    assert_eq!(numeric.next_nonce, Uint64(9));
    assert_eq!(numeric.creation_block_height, Uint64(10000));

    let order = client.order("mordr1x").await.unwrap();
    assert_eq!(order.nonce, Uint64(5));

    string_delegation.assert();
    numeric_delegation.assert();
    order_mock.assert();
}

#[tokio::test]
async fn utxo_wire_key_is_utxo() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/address/mxtc1q/spendable-utxos");
        respond(
            then,
            json!([{
                "outpoint": {"source_id": "aabb", "index": 0},
                "utxo": {"type": "Transfer"}
            }]),
        );
    });

    let client = Client::new(server.url(""));
    let utxos = client.spendable_utxos("mxtc1q").await.unwrap();

    assert_eq!(utxos.len(), 1);
    assert_eq!(
        utxos[0],
        Utxo {
            outpoint: UtxoOutpoint {
                source_id: "aabb".to_owned(),
                index: 0
            },
            output: json!({"type": "Transfer"})
        }
    );
    mock.assert();
}

#[tokio::test]
async fn unconfirmed_transaction_empty_string_fields() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/transaction/aabb");
        respond(
            then,
            json!({
                "id": "aabb",
                "inputs": [],
                "outputs": [],
                "block_id": "",
                "timestamp": "",
                "confirmations": ""
            }),
        );
    });

    let client = Client::new(server.url(""));
    let tx = client.transaction("aabb").await.unwrap();

    assert_eq!(tx.id, "aabb");
    assert_eq!(tx.block_id, "");
    assert_eq!(tx.timestamp, "");
    assert_eq!(tx.confirmations, "");
    mock.assert();
}

#[tokio::test]
async fn fee_rate_param_handling() {
    let server = MockServer::start();
    let default_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/feerate")
            .matches(|request| request.query_params.as_deref().unwrap_or(&[]).is_empty());
        then.status(200)
            .header("content-type", "application/json")
            .body(json!("12.5").to_string());
    });
    let top_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/feerate").query_param("in_top_x_mb", "5");
        then.status(200)
            .header("content-type", "application/json")
            .body(json!("500").to_string());
    });

    let client = Client::new(server.url(""));
    let default_rate = client.fee_rate(0).await.unwrap();
    assert_eq!(default_rate, "12.5");
    let top_rate = client.fee_rate(5).await.unwrap();
    assert_eq!(top_rate, "500");

    default_mock.assert();
    top_mock.assert();
}

#[tokio::test]
async fn base_url_trailing_slash_normalized() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/chain/tip");
        respond(then, json!({"block_height": 1, "block_id": "0000ab"}));
    });

    let client = Client::new(server.url("/") + "/");
    let tip = client.tip().await.unwrap();

    assert_eq!(tip.block_height, 1);
    assert_eq!(tip.block_id, "0000ab");
    mock.assert();
}

#[tokio::test]
async fn per_thousand_percent_forms() {
    let percent: PerThousand = serde_json::from_str("\"3.5%\"").unwrap();
    assert!((percent.0 - 3.5).abs() < f64::EPSILON);
    let whole_percent: PerThousand = serde_json::from_str("\"10%\"").unwrap();
    assert!((whole_percent.0 - 10.0).abs() < f64::EPSILON);
    let plain_string: PerThousand = serde_json::from_str("\"10\"").unwrap();
    assert!((plain_string.0 - 10.0).abs() < f64::EPSILON);
    let integer: PerThousand = serde_json::from_str("35").unwrap();
    assert!((integer.0 - 35.0).abs() < f64::EPSILON);

    let string_nonce: Uint64 = serde_json::from_str("\"7\"").unwrap();
    assert_eq!(string_nonce, Uint64(7));
    let integer_nonce: Uint64 = serde_json::from_str("9").unwrap();
    assert_eq!(integer_nonce, Uint64(9));
    assert_eq!(serde_json::to_string(&Uint64(7)).unwrap(), "7");
}
