// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wire-format tests for the indexer client, mirroring the go-sdk
//! `indexer/client_test.go` suite.

mod common;

use common::respond;
use httpmock::prelude::*;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

use mintlayer_sdk::indexer::{
    Amount, Client, Error, HoldersOpts, OffsetMode, OrderBookOpts, OrderBookSide, PageOpts,
    PerThousand, PoolListOpts, PoolSort, Uint64, Utxo, UtxoOutpoint,
};

#[tokio::test]
async fn tip_and_height_paths() {
    let server = MockServer::start();
    let tip_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/chain/tip");
        respond(
            then,
            200,
            json!({"block_height": 42000, "block_id": "0000ab"}).to_string(),
        );
    });
    let height_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/chain/1337");
        respond(then, 200, json!("0000cd").to_string());
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
            200,
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
            })
            .to_string(),
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
        respond(then, 200, json!([]).to_string());
    });
    let unpaginated = server.mock(|when, then| {
        when.method(GET).path("/api/v2/transaction").matches(|request| {
            let params = request.query_params.as_deref().unwrap_or(&[]);
            !params.iter().any(|(name, _)| *name == "offset" || *name == "items")
        });
        respond(then, 200, json!([]).to_string());
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
            200,
            json!([{
                "pool_id": "mpool1x",
                "decommission_destination": "mtct1x",
                "staker_balance": {"atoms": "1000", "decimal": "0.00000001"},
                "margin_ratio_per_thousand": "3.5%",
                "cost_per_block": {"atoms": "1000", "decimal": "0.00000001"},
                "vrf_public_key": "vrfpub1x",
                "delegations_balance": {"atoms": "2000", "decimal": "0.00000002"}
            }])
            .to_string(),
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
        respond(then, 200, json!({"block_count": 42}).to_string());
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
        respond(then, 200, json!({"tx_id": "aabb"}).to_string());
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
            200,
            json!({
                "delegation_id": "tdelg1x",
                "pool_id": "mpool1x",
                "next_nonce": "7",
                "spend_destination": "mtct1x",
                "balance": {"atoms": "500", "decimal": "0.000000005"},
                "creation_block_height": "10000"
            })
            .to_string(),
        );
    });
    let numeric_delegation = server.mock(|when, then| {
        when.method(GET).path("/api/v2/delegation/tdelg2x");
        respond(
            then,
            200,
            json!({
                "delegation_id": "tdelg2x",
                "pool_id": "mpool1x",
                "next_nonce": 9,
                "spend_destination": "mtct1x",
                "balance": {"atoms": 500, "decimal": "0.000000005"},
                "creation_block_height": 10000
            })
            .to_string(),
        );
    });
    let order_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/order/mordr1x");
        respond(
            then,
            200,
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
            })
            .to_string(),
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
            200,
            json!([{
                "outpoint": {"source_id": "aabb", "index": 0},
                "utxo": {"type": "Transfer"}
            }])
            .to_string(),
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
async fn pending_transaction_null_fields() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/transaction/aabb");
        respond(
            then,
            200,
            json!({
                "id": "aabb",
                "inputs": [],
                "outputs": [],
                "block_id": null,
                "timestamp": null,
                "confirmations": null
            })
            .to_string(),
        );
    });

    let client = Client::new(server.url(""));
    let tx = client.transaction("aabb").await.unwrap();

    assert_eq!(tx.id, "aabb");
    assert_eq!(tx.block_id, None);
    assert_eq!(tx.timestamp, None);
    assert_eq!(tx.confirmations, None);
    mock.assert();
}

#[tokio::test]
async fn fee_rate_param_handling() {
    let server = MockServer::start();
    let default_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/feerate")
            .matches(|request| request.query_params.as_deref().unwrap_or(&[]).is_empty());
        respond(then, 200, json!("12.5").to_string());
    });
    let top_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/feerate").query_param("in_top_x_mb", "5");
        respond(then, 200, json!("500").to_string());
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
        respond(
            then,
            200,
            json!({"block_height": 1, "block_id": "0000ab"}).to_string(),
        );
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

#[tokio::test]
async fn path_segments_are_validated() {
    let server = MockServer::start();
    let client = Client::new(server.url(""));
    for input in ["../../admin", "a/b", "a?x=1", "a b", "%2e%2e", ""] {
        let err = client.block(input).await.unwrap_err();
        assert!(
            matches!(err, Error::InvalidUrl { .. }),
            "got {err:?} for {input:?}"
        );
    }
}

#[tokio::test]
async fn error_body_is_sanitized() {
    let server = MockServer::start();
    let sanitized_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/order/unknown");
        then.status(404)
            .header("content-type", "text/plain")
            .body("not\nfound\rwith\x07bell");
    });
    let truncated_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/order/truncated");
        then.status(404).header("content-type", "text/plain").body("x".repeat(20_000));
    });

    let client = Client::new(server.url(""));
    match client.order("unknown").await {
        Err(Error::Http { status_code, body }) => {
            assert_eq!(status_code, 404);
            assert_eq!(body, "notfoundwithbell");
        }
        other => panic!("expected Error::Http, got {other:?}"),
    }

    match client.order("truncated").await {
        Err(Error::Http { status_code, body }) => {
            assert_eq!(status_code, 404);
            assert_eq!(body.chars().count(), 8192);
            assert!(body.chars().all(|c| c == 'x'));
        }
        other => panic!("expected Error::Http, got {other:?}"),
    }

    sanitized_mock.assert();
    truncated_mock.assert();
}

#[tokio::test]
async fn orders_by_pair_path_and_validation() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/order/pair/ML_mmltk1abc")
            .query_param("items", "5");
        respond(
            then,
            200,
            json!([{
                "order_id": "mordr1x",
                "conclude_destination": "mtct1x",
                "give_currency": {},
                "initially_given": {"atoms": "1", "decimal": "0.00000000001"},
                "give_balance": {"atoms": "1", "decimal": "0.00000000001"},
                "ask_currency": {},
                "initially_asked": {"atoms": "1", "decimal": "0.00000000001"},
                "ask_balance": {"atoms": "1", "decimal": "0.00000000001"},
                "nonce": "5"
            }])
            .to_string(),
        );
    });

    let client = Client::new(server.url(""));
    let orders = client
        .orders_by_pair(
            "ML",
            "mmltk1abc",
            PageOpts {
                offset: 0,
                items: 5,
            },
        )
        .await
        .unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].order_id, "mordr1x");
    assert_eq!(orders[0].conclude_destination, "mtct1x");
    assert_eq!(orders[0].nonce, Uint64(5));
    assert_eq!(
        orders[0].ask_balance,
        Amount {
            atoms: 1,
            decimal: "0.00000000001".to_owned()
        }
    );
    mock.assert();

    // A currency containing a slash would alter the request path and must be
    // rejected locally, before any request is sent.
    let err = client
        .orders_by_pair(
            "bad/slash",
            "x",
            PageOpts {
                offset: 0,
                items: 5,
            },
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, Error::InvalidUrl { .. }),
        "expected InvalidUrl, got {err:?}"
    );
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn oversized_response_rejected() {
    let server = MockServer::start();
    // A real oversized body: httpmock derives the content-length header from
    // the body length (65 MiB + 10 > the 64 MiB cap), so the transport's
    // content-length pre-check rejects the response before it is read.
    let oversized = "0".repeat(65 * 1024 * 1024 + 10);
    server.mock(|when, then| {
        when.method(GET).path("/api/v2/chain/tip");
        respond(then, 200, oversized);
    });

    let client = Client::new(server.url(""));
    match client.tip().await {
        Err(Error::ResponseTooLarge { limit }) => {
            assert_eq!(limit, 64 * 1024 * 1024);
        }
        other => panic!("expected ResponseTooLarge, got: {other:?}"),
    }
}

#[tokio::test]
async fn remaining_types_decode() {
    let server = MockServer::start();

    let stats_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/statistics/coin");
        respond(
            then,
            200,
            json!({
                "circulating_supply": {"atoms": "1000", "decimal": "0.00000001"},
                "preminted": {"atoms": "2000", "decimal": "0.00000002"},
                "burned": {"atoms": "3000", "decimal": "0.00000003"},
                "staked": {"atoms": "4000", "decimal": "0.00000004"},
            })
            .to_string(),
        );
    });
    let token_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/token/mmltk1full");
        respond(
            then,
            200,
            json!({
                "authority": "mtc1qauth",
                "is_locked": false,
                "circulating_supply": {"atoms": "700", "decimal": "0.7"},
                "token_ticker": "MTK",
                "metadata_uri": "https://example.com/token.json",
                "number_of_decimals": 2,
                "total_supply": {"type": "Lockable"},
                "frozen": false,
                "is_token_freezable": true,
                "next_nonce": "3"
            })
            .to_string(),
        );
    });
    let nft_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/nft/mmltk1nft");
        respond(
            then,
            200,
            json!({
                "owner": "mtc1qowner",
                "token_id": "mmltk1nft",
                "metadata": {
                    "creator": "02a1b2c3",
                    "name": "Genesis NFT",
                    "description": "The first NFT",
                    "ticker": "GNFT",
                    "icon_uri": null,
                    "additional_metadata_uri": null,
                    "media_uri": "https://example.com/media.png",
                    "media_hash": "aabb"
                }
            })
            .to_string(),
        );
    });
    let merkle_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/transaction/aabb/merkle-path");
        respond(
            then,
            200,
            json!({
                "block_id": "0000aa",
                "transaction_index": 2,
                "merkle_root": "ccdd",
                "path": ["eeff", "1122"]
            })
            .to_string(),
        );
    });
    let address_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/address/mxtc1q");
        respond(
            then,
            200,
            json!({
                "coin_balance": {"atoms": "1000", "decimal": "0.00000001"},
                "locked_coin_balance": {"atoms": "500", "decimal": "0.000000005"},
                "transaction_history": ["aabb"],
                "tokens": [
                    {"token_id": "mmltk1x", "amount": {"atoms": "700", "decimal": "0.7"}}
                ]
            })
            .to_string(),
        );
    });
    let delegations_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/pool/mpool1x/delegations");
        respond(
            then,
            200,
            json!([{
                "delegation_id": "tdelg1x",
                "next_nonce": "7",
                "spend_destination": "mtct1x",
                "balance": {"atoms": "500", "decimal": "0.000000005"},
                "creation_block_height": 10000
            }])
            .to_string(),
        );
    });
    let token_txs_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/token/mmltk1x/transactions");
        respond(
            then,
            200,
            json!([{ "tx_global_index": 11, "tx_id": "aabb" }]).to_string(),
        );
    });
    let tx_ids_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/block/0000aa/transaction-ids");
        respond(then, 200, json!(["aabb", "ccdd"]).to_string());
    });
    let reward_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/block/0000aa/reward");
        respond(then, 200, json!([{ "type": "Transfer" }]).to_string());
    });
    let genesis_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/chain/genesis");
        respond(
            then,
            200,
            json!({
                "block_id": "000000genesishash",
                "genesis_message": "Mintlayer genesis",
                "timestamp": {"timestamp": 1_700_000_000},
                "utxos": []
            })
            .to_string(),
        );
    });
    let all_utxos_mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/address/mxtc1q/all-utxos");
        respond(
            then,
            200,
            json!([{
                "outpoint": {"source_id": "aabb", "index": 1},
                "utxo": {"type": "Transfer"}
            }])
            .to_string(),
        );
    });

    let client = Client::new(server.url(""));

    let stats = client.coin_statistics().await.unwrap();
    assert_eq!(stats.circulating_supply.atoms, 1000);
    assert_eq!(stats.staked.atoms, 4000);

    let token = client.token("mmltk1full").await.unwrap();
    assert_eq!(token.token_ticker, "MTK");
    assert_eq!(token.is_token_freezable, Some(true));
    assert_eq!(token.next_nonce, Uint64(3));

    let nft = client.nft("mmltk1nft").await.unwrap();
    assert_eq!(nft.owner, "mtc1qowner");
    assert_eq!(nft.metadata.name, "Genesis NFT");
    assert_eq!(
        nft.metadata.media_uri.as_deref(),
        Some("https://example.com/media.png")
    );

    let merkle = client.transaction_merkle_path("aabb").await.unwrap();
    assert_eq!(merkle.block_id, "0000aa");
    assert_eq!(merkle.transaction_index, 2);
    assert_eq!(merkle.path, vec!["eeff".to_owned(), "1122".to_owned()]);

    let address = client.address_info("mxtc1q").await.unwrap();
    assert_eq!(address.coin_balance.atoms, 1000);
    assert_eq!(address.tokens.len(), 1);
    assert_eq!(address.tokens[0].token_id, "mmltk1x");
    assert_eq!(address.tokens[0].amount.atoms, 700);

    let delegations = client.pool_delegations("mpool1x").await.unwrap();
    assert_eq!(delegations.len(), 1);
    assert_eq!(delegations[0].delegation_id, "tdelg1x");
    assert_eq!(delegations[0].creation_block_height, Uint64(10000));

    let token_txs = client.token_transactions("mmltk1x", PageOpts::default()).await.unwrap();
    assert_eq!(token_txs.len(), 1);
    assert_eq!(token_txs[0].tx_global_index, 11);
    assert_eq!(token_txs[0].tx_id, "aabb");

    assert_eq!(
        client.block_transaction_ids("0000aa").await.unwrap(),
        vec!["aabb".to_owned(), "ccdd".to_owned()]
    );

    let reward = client.block_reward("0000aa").await.unwrap();
    assert_eq!(reward, vec![json!({ "type": "Transfer" })]);

    let genesis = client.genesis().await.unwrap();
    assert_eq!(genesis.genesis_message, "Mintlayer genesis");
    assert_eq!(genesis.timestamp.timestamp, 1_700_000_000);

    let utxos = client.all_utxos("mxtc1q").await.unwrap();
    assert_eq!(utxos.len(), 1);
    assert_eq!(utxos[0].outpoint.index, 1);
    assert_eq!(utxos[0].output, json!({ "type": "Transfer" }));

    stats_mock.assert();
    token_mock.assert();
    nft_mock.assert();
    merkle_mock.assert();
    address_mock.assert();
    delegations_mock.assert();
    token_txs_mock.assert();
    tx_ids_mock.assert();
    reward_mock.assert();
    genesis_mock.assert();
    all_utxos_mock.assert();
}

// ---------------------------------------------------------------------------
// Cursor pagination, holders, and order book (api-server v2, PR #2130)
// ---------------------------------------------------------------------------

fn holder(address: &str, atoms: &str, decimal: &str) -> serde_json::Value {
    json!({"address": address, "amount": {"atoms": atoms, "decimal": decimal}})
}

fn full_pool(pool_id: &str) -> serde_json::Value {
    json!({
        "pool_id": pool_id,
        "decommission_destination": "mtct1x",
        "staker_balance": {"atoms": "1000", "decimal": "0.00000001"},
        "margin_ratio_per_thousand": "3.5%",
        "cost_per_block": {"atoms": "1000", "decimal": "0.00000001"},
        "vrf_public_key": "vrfpub1x",
        "delegations_balance": {"atoms": "2000", "decimal": "0.00000002"}
    })
}

#[tokio::test]
async fn coin_holders_cursor_walk() {
    let server = MockServer::start();
    let first = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/statistics/coin/holders")
            .query_param("items", "2")
            .matches(|request| {
                let params = request.query_params.as_deref().unwrap_or(&[]);
                !params.iter().any(|(name, _)| *name == "cursor")
            });
        respond(
            then,
            200,
            json!({
                "items": [
                    holder("mt1qrich", "9000000000", "90"),
                    holder("mt1qpoor", "1000000000", "10"),
                ],
                "next_cursor": "cursor-1",
            })
            .to_string(),
        );
    });
    let second = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/statistics/coin/holders")
            .query_param("cursor", "cursor-1")
            .query_param("items", "2");
        respond(
            then,
            200,
            json!({"items": [holder("mt1qmid", "5000000000", "50")], "next_cursor": null})
                .to_string(),
        );
    });

    let client = Client::new(server.url(""));
    let mut pager = client.coin_holders_pager(2);
    let mut walked = Vec::new();
    while let Some(item) = pager.next().await {
        walked.push(item.unwrap().address);
    }
    assert_eq!(walked, ["mt1qrich", "mt1qpoor", "mt1qmid"]);
    first.assert();
    second.assert();
}

#[tokio::test]
async fn token_holders_walk_and_last_page() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/statistics/token/tok1/holders")
            .query_param("cursor", "resume");
        respond(
            then,
            200,
            json!({"items": [holder("mt1qone", "7", "0.00000007")], "next_cursor": null})
                .to_string(),
        );
    });

    let client = Client::new(server.url(""));
    let page = client
        .token_holders(
            "tok1",
            HoldersOpts {
                offset: 0,
                items: 0,
                cursor: Some("resume".to_owned()),
            },
        )
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].amount.atoms, 7);
    assert_eq!(page.next_cursor, None);
    mock.assert();
}

#[tokio::test]
async fn token_holders_unknown_token_is_token_not_found() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/statistics/token/tok1/holders");
        respond(then, 404, json!({"error": "Token not found"}).to_string());
    });

    let client = Client::new(server.url(""));
    let error = client.token_holders("tok1", HoldersOpts::default()).await.unwrap_err();
    assert!(matches!(error, Error::TokenNotFound), "{error:?}");
    mock.assert();
}

#[tokio::test]
async fn pager_clamps_page_size_to_server_limits() {
    let server = MockServer::start();
    let clamped_high = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/statistics/coin/holders")
            .query_param("items", "100");
        respond(
            then,
            200,
            json!({"items": [], "next_cursor": null}).to_string(),
        );
    });
    let clamped_low = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/statistics/coin/holders")
            .query_param("items", "1");
        respond(
            then,
            200,
            json!({"items": [], "next_cursor": null}).to_string(),
        );
    });

    let client = Client::new(server.url(""));
    client.coin_holders_pager(250).next().await;
    client.coin_holders_pager(0).next().await;
    clamped_high.assert();
    clamped_low.assert();
}

#[tokio::test]
async fn oversized_items_is_invalid_num_items() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/statistics/coin/holders")
            .query_param("items", "150");
        respond(
            then,
            400,
            json!({"error": "Invalid number of items"}).to_string(),
        );
    });

    let client = Client::new(server.url(""));
    let error = client
        .coin_holders(HoldersOpts {
            offset: 0,
            items: 150,
            cursor: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(error, Error::InvalidNumItems), "{error:?}");
    mock.assert();
}

#[tokio::test]
async fn order_book_decodes_both_sides() {
    let server = MockServer::start();
    let ask = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/order/pair/ML_tok1/book")
            .query_param("side", "ask")
            .query_param("items", "2");
        respond(
            then,
            200,
            json!({
                "items": [
                    {"price": {"atoms": "3/2", "decimal": "1.5"},
                     "amount": {"atoms": "200000000", "decimal": "2"}},
                    {"price": {"atoms": "2/1", "decimal": "2"},
                     "amount": {"atoms": "100000000", "decimal": "1"}},
                ],
                "next_cursor": "ask-cursor",
            })
            .to_string(),
        );
    });
    let bid = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/order/pair/ML_tok1/book")
            .query_param("side", "bid");
        respond(
            then,
            200,
            json!({
                "items": [
                    {"price": {"atoms": "1/3", "decimal": "0.33"},
                     "amount": {"atoms": "100000000", "decimal": "1"}},
                ],
                "next_cursor": "bid-cursor",
            })
            .to_string(),
        );
    });

    let client = Client::new(server.url(""));
    let ask_book = client
        .order_pair_book(
            "ML",
            "tok1",
            OrderBookSide::Ask,
            OrderBookOpts {
                offset: 0,
                items: 2,
                cursor: None,
            },
        )
        .await
        .unwrap();
    assert!(!ask_book.truncated);
    assert_eq!(ask_book.next_cursor.as_deref(), Some("ask-cursor"));
    assert_eq!(ask_book.items[0].price.atoms, "3/2");
    assert_eq!(ask_book.items[0].price.decimal, "1.5");
    assert_eq!(ask_book.items[0].amount.atoms, 200_000_000);

    let bid_book = client
        .order_pair_book("ML", "tok1", OrderBookSide::Bid, OrderBookOpts::default())
        .await
        .unwrap();
    assert_eq!(bid_book.items[0].price.atoms, "1/3");
    ask.assert();
    bid.assert();
}

#[tokio::test]
async fn truncated_order_book_stops_the_pager() {
    let server = MockServer::start();
    // A truncated book always arrives with `next_cursor: null`; a pager
    // walk must stop instead of trying to continue.
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/order/pair/ML_tok1/book")
            .query_param("side", "ask");
        respond(
            then,
            200,
            json!({
                "items": [
                    {"price": {"atoms": "3/2", "decimal": "1.5"},
                     "amount": {"atoms": "200000000", "decimal": "2"}},
                ],
                "next_cursor": null,
                "truncated": true,
            })
            .to_string(),
        );
    });

    let client = Client::new(server.url(""));
    let book = client
        .order_pair_book("ML", "tok1", OrderBookSide::Ask, OrderBookOpts::default())
        .await
        .unwrap();
    assert!(book.truncated);
    assert_eq!(book.next_cursor, None);

    let mut pager = client.order_book_pager("ML", "tok1", OrderBookSide::Ask, 10).unwrap();
    let mut levels = 0;
    while let Some(level) = pager.next().await {
        level.unwrap();
        levels += 1;
    }
    assert_eq!(levels, 1);
    mock.assert_hits(2); // one direct fetch, one pager fetch; no continuation
}

#[tokio::test]
async fn wrong_side_cursor_is_invalid_cursor() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/order/pair/ML_tok1/book")
            .query_param("side", "bid")
            .query_param("cursor", "ask-cursor");
        respond(then, 400, json!({"error": "Invalid cursor"}).to_string());
    });

    let client = Client::new(server.url(""));
    let error = client
        .order_pair_book(
            "ML",
            "tok1",
            OrderBookSide::Bid,
            OrderBookOpts {
                offset: 0,
                items: 0,
                cursor: Some("ask-cursor".to_owned()),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(error, Error::InvalidCursor), "{error:?}");
    mock.assert();
}

#[tokio::test]
async fn pools_cursor_walk() {
    let server = MockServer::start();
    let first = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/pool")
            .query_param("cursor", "")
            .query_param("items", "1");
        respond(
            then,
            200,
            json!({"items": [full_pool("p1")], "next_cursor": "pool-cursor"}).to_string(),
        );
    });
    let second = server.mock(|when, then| {
        when.method(GET).path("/api/v2/pool").query_param("cursor", "pool-cursor");
        respond(
            then,
            200,
            json!({"items": [full_pool("p2")], "next_cursor": null}).to_string(),
        );
    });

    let client = Client::new(server.url(""));
    let mut pager = client.pools_pager(1);
    let mut walked = Vec::new();
    while let Some(pool) = pager.next().await {
        walked.push(pool.unwrap().pool_id);
    }
    assert_eq!(walked, ["p1", "p2"]);
    first.assert();
    second.assert();
}

#[tokio::test]
async fn transactions_cursor_walk_with_pending_block_id() {
    let server = MockServer::start();
    let first = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/transaction")
            .query_param("cursor", "")
            .query_param("items", "1");
        respond(
            then,
            200,
            json!({
                "items": [{
                    "id": "tx1",
                    "inputs": [],
                    "outputs": [],
                    "block_id": "0000aa",
                    "timestamp": "1700000000",
                    "confirmations": "10",
                    "tx_global_index": 41,
                }],
                "next_cursor": "tx-cursor",
            })
            .to_string(),
        );
    });
    let second = server.mock(|when, then| {
        when.method(GET).path("/api/v2/transaction").query_param("cursor", "tx-cursor");
        respond(
            then,
            200,
            json!({
                "items": [{
                    "id": "tx2",
                    "inputs": [],
                    "outputs": [],
                    "block_id": null,
                    "timestamp": null,
                    "confirmations": null,
                }],
                "next_cursor": null,
            })
            .to_string(),
        );
    });

    let client = Client::new(server.url(""));
    let mut pager = client.transactions_pager(1);
    let mut walked = Vec::new();
    while let Some(tx) = pager.next().await {
        let tx = tx.unwrap();
        assert_eq!(tx.block_id.is_none(), tx.id == "tx2");
        walked.push(tx.id);
    }
    assert_eq!(walked, ["tx1", "tx2"]);
    first.assert();
    second.assert();
}

#[tokio::test]
async fn transactions_offset_mode_is_sent() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/transaction")
            .query_param("offset_mode", "absolute")
            .query_param("offset", "5")
            .query_param("items", "2");
        respond(then, 200, json!([]).to_string());
    });

    let client = Client::new(server.url(""));
    let transactions = client
        .list_transactions_with_offset_mode(
            OffsetMode::Absolute,
            PageOpts {
                offset: 5,
                items: 2,
            },
        )
        .await
        .unwrap();
    assert!(transactions.is_empty());
    mock.assert();
}

#[tokio::test]
async fn incompatible_parameters_surface_as_bad_request() {
    let server = MockServer::start();
    // A cursor combined with a non-default pools sort (or with
    // `offset_mode`) is rejected server-side; the typed methods cannot
    // produce that pair, but the mapping must surface the server's verdict
    // verbatim.
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/pool").query_param("cursor", "pool-cursor");
        respond(then, 400, json!({"error": "Bad request"}).to_string());
    });

    let client = Client::new(server.url(""));
    let error = client.list_pools_paged(Some("pool-cursor"), 10).await.unwrap_err();
    assert!(matches!(error, Error::BadRequest), "{error:?}");
    mock.assert();
}

#[tokio::test]
async fn unmatched_server_errors_fall_back_to_http() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/v2/statistics/coin/holders");
        respond(
            then,
            500,
            json!({"error": "Internal server error"}).to_string(),
        );
    });

    let client = Client::new(server.url(""));
    let error = client.coin_holders(HoldersOpts::default()).await.unwrap_err();
    match error {
        Error::Http { status_code, body } => {
            assert_eq!(status_code, 500);
            assert!(body.contains("Internal server error"), "{body}");
        }
        other => panic!("expected Http, got {other:?}"),
    }
    mock.assert();
}

#[tokio::test]
async fn pager_retry_after_failed_fetch_resumes_from_same_cursor() {
    let server = MockServer::start();
    let first = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v2/statistics/coin/holders")
            .query_param("items", "1")
            .matches(|request| {
                let params = request.query_params.as_deref().unwrap_or(&[]);
                !params.iter().any(|(name, _)| *name == "cursor")
            });
        respond(
            then,
            200,
            json!({"items": [holder("mt1qa", "3", "0.00000003")], "next_cursor": "cursor-1"})
                .to_string(),
        );
    });

    // The first request carrying cursor-1 fails; the retry (same request)
    // succeeds. httpmock's `matches` takes a plain fn pointer (no
    // captures), so the two identical requests are routed via a static
    // request counter; httpmock serves the first registered match.
    static RETRY_HITS: AtomicUsize = AtomicUsize::new(0);
    let failing = server.mock(|when, then| {
        when.method(GET).path("/api/v2/statistics/coin/holders").matches(|request| {
            let params = request.query_params.as_deref().unwrap_or(&[]);
            if !params.iter().any(|(name, value)| name == "cursor" && value == "cursor-1") {
                return false;
            }
            RETRY_HITS.fetch_add(1, Ordering::SeqCst) == 0
        });
        respond(then, 500, json!({"error": "boom"}).to_string());
    });
    let succeeding = server.mock(|when, then| {
        when.method(GET).path("/api/v2/statistics/coin/holders").matches(|request| {
            let params = request.query_params.as_deref().unwrap_or(&[]);
            if !params.iter().any(|(name, value)| name == "cursor" && value == "cursor-1") {
                return false;
            }
            RETRY_HITS.load(Ordering::SeqCst) >= 1
        });
        respond(
            then,
            200,
            json!({"items": [holder("mt1qb", "4", "0.00000004")], "next_cursor": null}).to_string(),
        );
    });

    let client = Client::new(server.url(""));
    let mut pager = client.coin_holders_pager(1);
    let mut walked = Vec::new();
    let mut errors = 0;
    while let Some(item) = pager.next().await {
        match item {
            Ok(h) => walked.push(h.address),
            Err(_) => errors += 1,
        }
    }
    // The error is surfaced once, then the retry resumes from cursor-1:
    // "mt1qa" is not re-emitted and the walk still reaches "mt1qb".
    assert_eq!(errors, 1);
    assert_eq!(walked, ["mt1qa", "mt1qb"]);
    assert_eq!(pager.cursor(), None);
    first.assert_hits(1);
    failing.assert_hits(1);
    succeeding.assert_hits(1);
}
