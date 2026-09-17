// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wire-format tests for the wallet daemon client, mirroring the go-sdk
//! `wallet/client_test.go` and `wallet/orders_test.go` suites.

mod common;

use std::sync::Arc;

use common::{mock_rpc, respond, rpc_error, rpc_ok};
use httpmock::prelude::*;
use serde_json::json;

use mintlayer_sdk::wallet::{
    Amount, Client, ComposeParams, CreateOrderParams, CreateWalletParams, CurrencyFilter, Error,
    IssueTokenParams, ListOrdersParams, LockSupplyParams, Outpoint, OutpointSourceId, OutputValue,
    RecoverWalletParams, SendParams, StakingStatus, TokenMetadata, TokenSendParams, TokenSupply,
    TxOptions, UtxoSpendParams,
};

const MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn send_result() -> serde_json::Value {
    json!({
        "tx_id": "f0f1f2",
        "fees": {
            "coins": { "atoms": "100", "decimal": "0.000000001" },
            "tokens": {},
        },
        "broadcasted": true,
    })
}

#[tokio::test]
async fn create_wallet_returns_mnemonic() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/")
            .body_contains("\"method\":\"wallet_create\"")
            .body_contains("\"store_seed_phrase\":true");
        respond(
            then,
            200,
            rpc_ok(
                1,
                json!({
                    "mnemonic": {
                        "type": "NewlyGenerated",
                        "content": { "mnemonic": MNEMONIC },
                    },
                }),
            ),
        );
    });

    let client = Client::new(server.url("/"));
    let params = CreateWalletParams {
        path: "/tmp/wallet.dat".to_string(),
        store_seed_phrase: true,
        mnemonic: None,
        passphrase: None,
        hardware_wallet: None,
    };
    let result = client.create_wallet(params).await.unwrap();

    let mnemonic = result.mnemonic.expect("daemon generated a mnemonic");
    assert_eq!(mnemonic.kind, "NewlyGenerated");
    let content = mnemonic.content.expect("mnemonic content is set");
    assert_eq!(content.mnemonic, MNEMONIC);
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn open_wallet_password_null_handling() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/")
            .body_contains("\"method\":\"wallet_open\"")
            .body_contains("\"path\":\"/tmp/wallet.dat\"")
            .body_contains("\"password\":null");
        respond(then, 200, rpc_ok(1, json!(null)));
    });

    let client = Client::new(server.url("/"));
    client.open_wallet("/tmp/wallet.dat", None).await.unwrap();
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn balance_params_and_result() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/")
            .body_contains("\"method\":\"account_balance\"")
            .body_contains("\"utxo_states\":[\"Confirmed\"]")
            .body_contains("\"with_locked\":null");
        respond(
            then,
            200,
            rpc_ok(
                1,
                json!({
                    "coins": { "atoms": "5000000000000", "decimal": "50.0" },
                    "tokens": { "mmltk1x": { "atoms": "700", "decimal": "0.7" } },
                }),
            ),
        );
    });

    let client = Client::new(server.url("/"));
    let balance = client.balance(0).await.unwrap();

    assert_eq!(balance.coins.atoms(), Some(5_000_000_000_000));
    assert_eq!(balance.coins.decimal(), Some("50.0"));
    let token = balance.tokens.get("mmltk1x").expect("token balance present");
    assert_eq!(token.atoms(), Some(700));
    assert_eq!(token.decimal(), Some("0.7"));
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn new_address_extracts_field() {
    let server = MockServer::start();
    let mock = mock_rpc(
        &server,
        "\"method\":\"address_new\"".to_string(),
        rpc_ok(1, json!({ "address": "mtc1qxyz" })),
    );

    let client = Client::new(server.url("/"));
    assert_eq!(client.new_address(0).await.unwrap(), "mtc1qxyz");
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn send_params_wire_shape() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/").matches(|req| {
            let body =
                std::str::from_utf8(req.body.as_deref().unwrap_or_default()).unwrap_or_default();
            body.contains("\"method\":\"address_send\"")
                && body.contains("\"amount\":{\"atoms\":\"1000000000000\"}")
                && body.contains("\"selected_utxos\":[]")
                && !body.contains("\"selected_utxos\":null")
                && body.contains("\"options\":{\"in_top_x_mb\":null,\"broadcast_to_mempool\":null}")
        });
        respond(then, 200, rpc_ok(1, send_result()));
    });

    let client = Client::new(server.url("/"));
    let params = SendParams {
        account: 0,
        address: "mtc1qsending".to_string(),
        amount: Amount::from_atoms(1_000_000_000_000),
        selected_utxos: Vec::new(),
        options: TxOptions::default(),
    };
    let result = client.send(params).await.unwrap();

    assert_eq!(result.tx_id, "f0f1f2");
    assert_eq!(result.fees.coins.atoms(), Some(100));
    assert_eq!(result.fees.coins.decimal(), Some("0.000000001"));
    assert!(result.fees.tokens.is_empty());
    assert!(result.broadcasted);
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn submit_transaction_hardcodes_trusted() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/")
            .body_contains("\"method\":\"node_submit_transaction\"")
            .body_contains("\"tx\":\"deadbeef\"")
            .body_contains("\"do_not_store\":true")
            .body_contains("\"options\":{\"trust_policy\":\"Trusted\"}");
        respond(then, 200, rpc_ok(1, json!({ "tx_id": "aabb" })));
    });

    let client = Client::new(server.url("/"));
    let result = client.submit_transaction("deadbeef", true).await.unwrap();
    assert_eq!(result.tx_id, "aabb");
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn reveal_public_key_returns_struct() {
    let server = MockServer::start();
    let mock = mock_rpc(
        &server,
        "\"method\":\"address_reveal_public_key\"".to_string(),
        rpc_ok(
            1,
            json!({
                "public_key_hex": "02a1b2c3d4e5f60718",
                "public_key_address": "mtc1qkeyaddr",
            }),
        ),
    );

    let client = Client::new(server.url("/"));
    let key = client.reveal_public_key(0, "mtc1qxyz").await.unwrap();
    assert_eq!(key.public_key_hex, "02a1b2c3d4e5f60718");
    assert_eq!(key.public_key_address, "mtc1qkeyaddr");
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn pool_balance_null_and_value() {
    let server = MockServer::start();
    let value_mock = mock_rpc(
        &server,
        "\"id\":1,\"method\":\"staking_pool_balance\"".to_string(),
        rpc_ok(
            1,
            json!({ "balance": { "atoms": "100", "decimal": "0.000000001" } }),
        ),
    );
    let null_mock = mock_rpc(
        &server,
        "\"id\":2,\"method\":\"staking_pool_balance\"".to_string(),
        rpc_ok(2, json!({ "balance": null })),
    );

    let client = Client::new(server.url("/"));
    let balance = client.pool_balance("pool1abc").await.unwrap().expect("pool has a balance");
    assert_eq!(balance.atoms(), Some(100));
    assert_eq!(balance.decimal(), Some("0.000000001"));
    assert!(client.pool_balance("pool1abc").await.unwrap().is_none());

    assert_eq!(value_mock.hits(), 1);
    assert_eq!(null_mock.hits(), 1);
}

#[tokio::test]
async fn staking_status_parses_enum() {
    let server = MockServer::start();
    let staking_mock = mock_rpc(
        &server,
        "\"id\":1,\"method\":\"staking_status\"".to_string(),
        rpc_ok(1, json!("Staking")),
    );
    let not_staking_mock = mock_rpc(
        &server,
        "\"id\":2,\"method\":\"staking_status\"".to_string(),
        rpc_ok(2, json!("NotStaking")),
    );

    let client = Client::new(server.url("/"));
    assert_eq!(
        client.staking_status(0).await.unwrap(),
        StakingStatus::Staking
    );
    assert_eq!(
        client.staking_status(0).await.unwrap(),
        StakingStatus::NotStaking
    );

    assert_eq!(staking_mock.hits(), 1);
    assert_eq!(not_staking_mock.hits(), 1);
}

#[tokio::test]
async fn create_order_wire_shapes() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/").matches(|req| {
            let body =
                std::str::from_utf8(req.body.as_deref().unwrap_or_default()).unwrap_or_default();
            body.contains("\"method\":\"order_create\"")
                && body.contains(
                    "\"ask\":{\"type\":\"Coin\",\"content\":{\"amount\":{\"atoms\":\"1000000000000\"}}}",
                )
                && body.contains(
                    "\"give\":{\"type\":\"Token\",\"content\":{\"id\":\"mmltk1x\",\"amount\":{\"atoms\":\"700\"}}}",
                )
        });
        respond(
            then,
            200,
            rpc_ok(
                1,
                json!({
                    "order_id": "order1abc",
                    "tx_id": "0f0e0d",
                    "broadcasted": true,
                }),
            ),
        );
    });

    let client = Client::new(server.url("/"));
    let params = CreateOrderParams {
        account: 0,
        ask: OutputValue::coins(1_000_000_000_000),
        give: OutputValue::token("mmltk1x", 700),
        conclude_address: "mtc1qconclude".to_string(),
        options: TxOptions::default(),
    };
    let order = client.create_order(params).await.unwrap();

    assert_eq!(order.order_id, "order1abc");
    assert_eq!(order.tx_id, "0f0e0d");
    assert!(order.broadcasted);
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn list_all_active_orders_null_filters() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/")
            .body_contains("\"method\":\"order_list_all_active\"")
            .body_contains("\"ask_currency\":null")
            .body_contains("\"give_currency\":null");
        respond(
            then,
            200,
            rpc_ok(
                1,
                json!([
                    {
                        "order_id": "order1xyz",
                        "initially_asked": {
                            "type": "Coin",
                            "content": { "amount": { "atoms": "1000" } },
                        },
                        "initially_given": {
                            "type": "Token",
                            "content": { "id": "mmltk1x", "amount": { "atoms": "50" } },
                        },
                        "ask_balance": { "atoms": "900" },
                        "give_balance": { "atoms": "45" },
                        "is_own": false,
                    },
                ]),
            ),
        );
    });

    let client = Client::new(server.url("/"));
    let params = ListOrdersParams {
        account: 0,
        ask_currency: None,
        give_currency: None,
    };
    let orders = client.list_all_active_orders(params).await.unwrap();

    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].order_id, "order1xyz");
    assert_eq!(orders[0].ask_balance.atoms(), Some(900));
    assert_eq!(orders[0].give_balance.atoms(), Some(45));
    assert!(!orders[0].is_own);
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn lock_supply_uses_account_index() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/").matches(|req| {
            let body =
                std::str::from_utf8(req.body.as_deref().unwrap_or_default()).unwrap_or_default();
            body.contains("\"method\":\"token_lock_supply\"")
                && body.contains("\"account_index\":3")
                && !body.contains("\"account\":")
        });
        respond(then, 200, rpc_ok(1, send_result()));
    });

    let client = Client::new(server.url("/"));
    let params = LockSupplyParams {
        account_index: 3,
        token_id: "mmltk1x".to_string(),
        options: TxOptions::default(),
    };
    client.lock_token_supply(params).await.unwrap();
    assert_eq!(mock.hits(), 1);
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
    let err = client.best_block().await.unwrap_err();
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
async fn output_value_and_currency_filter_serde() {
    assert_eq!(
        serde_json::to_string(&Amount::from_atoms(1_000_000_000_000)).unwrap(),
        r#"{"atoms":"1000000000000"}"#
    );
    assert_eq!(
        serde_json::to_string(&OutputValue::coins(1_000_000_000_000)).unwrap(),
        r#"{"type":"Coin","content":{"amount":{"atoms":"1000000000000"}}}"#
    );
    assert_eq!(
        serde_json::to_string(&OutputValue::token("mmltk1x", 700)).unwrap(),
        r#"{"type":"Token","content":{"id":"mmltk1x","amount":{"atoms":"700"}}}"#
    );
    assert_eq!(
        serde_json::to_string(&CurrencyFilter::Coin).unwrap(),
        r#"{"type":"Coin"}"#
    );
    assert_eq!(
        serde_json::to_string(&CurrencyFilter::Token("mmltk1x".to_string())).unwrap(),
        r#"{"type":"Token","content":"mmltk1x"}"#
    );

    let amount: Amount = serde_json::from_value(json!({ "atoms": "1000" })).unwrap();
    assert_eq!(amount.atoms(), Some(1000));
    assert_eq!(amount.decimal(), None);
    let amount: Amount = serde_json::from_value(json!({ "atoms": 1000 })).unwrap();
    assert_eq!(amount.atoms(), Some(1000));
    let amount: Amount =
        serde_json::from_value(json!({ "atoms": "1000", "decimal": "0.00000001" })).unwrap();
    assert_eq!(amount.atoms(), Some(1000));
    assert_eq!(amount.decimal(), Some("0.00000001"));
}

#[tokio::test]
async fn concurrent_calls_use_unique_ids() {
    let server = MockServer::start();
    let mut mocks = Vec::new();
    for n in 1..=10u64 {
        let matcher = format!("\"id\":{n},\"method\":\"wallet_best_block\"");
        let block = json!({ "height": n, "id": format!("{n:064x}") });
        mocks.push(mock_rpc(&server, matcher, rpc_ok(n, block)));
    }

    let client = Client::new(server.url("/"));
    let shared = Arc::new(client);
    let mut handles = Vec::new();
    for _ in 0..10 {
        let client = Arc::clone(&shared);
        handles.push(tokio::spawn(async move {
            client.best_block().await.unwrap().height
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

#[tokio::test]
async fn debug_output_redacts_mnemonics_and_passphrases() {
    let create_params = CreateWalletParams {
        mnemonic: Some("secret words here".to_string()),
        passphrase: Some("secret-pass".to_string()),
        path: "/tmp/w.dat".to_string(),
        store_seed_phrase: true,
        hardware_wallet: None,
    };
    let create_debug = format!("{create_params:?}");
    assert!(!create_debug.contains("secret words here"));
    assert!(!create_debug.contains("secret-pass"));
    assert!(create_debug.contains("***"));

    let recover_params = RecoverWalletParams {
        mnemonic: "secret words here".to_string(),
        passphrase: Some("secret-pass".to_string()),
        path: "/tmp/w.dat".to_string(),
        store_seed_phrase: true,
        hardware_wallet: None,
    };
    let recover_debug = format!("{recover_params:?}");
    assert!(!recover_debug.contains("secret words here"));
    assert!(!recover_debug.contains("secret-pass"));
    assert!(recover_debug.contains("***"));

    // The mnemonic returned by the daemon is redacted in result debug output.
    let server = MockServer::start();
    mock_rpc(
        &server,
        "\"method\":\"wallet_create\"".to_string(),
        rpc_ok(
            1,
            json!({
                "mnemonic": {
                    "type": "NewlyGenerated",
                    "content": { "mnemonic": "top secret seed phrase" },
                },
            }),
        ),
    );

    let client = Client::new(server.url("/"));
    let result = client.create_wallet(create_params).await.unwrap();
    let result_debug = format!("{result:?}");
    assert!(!result_debug.contains("top secret seed phrase"));
    assert!(result_debug.contains("<redacted>"));
}

#[tokio::test]
async fn debug_output_redacts_htlc_secrets() {
    let params = UtxoSpendParams {
        account: 0,
        utxo: Outpoint {
            source_id: OutpointSourceId::Transaction {
                tx_id: "aa".to_string(),
            },
            index: 0,
        },
        output_address: "mtc1qx".to_string(),
        htlc_secret: Some("supersecret123".to_string()),
        options: TxOptions::default(),
    };
    let spend_debug = format!("{params:?}");
    assert!(!spend_debug.contains("supersecret123"));
    assert!(spend_debug.contains("***"));

    let params = ComposeParams {
        inputs: vec![],
        outputs: vec![],
        htlc_secrets: Some(json!("supersecret456")),
        only_transaction: true,
    };
    let compose_debug = format!("{params:?}");
    assert!(!compose_debug.contains("supersecret456"));
    assert!(compose_debug.contains("***"));
}

#[tokio::test]
async fn send_token_wire_shape() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/").matches(|req| {
            let body =
                std::str::from_utf8(req.body.as_deref().unwrap_or_default()).unwrap_or_default();
            body.contains("\"method\":\"token_send\"")
                && body.contains("\"token_id\":\"mmltk1x\"")
                && body.contains("\"address\":\"mtc1qy\"")
                && body.contains("\"amount\":{\"atoms\":\"700\"}")
                && !body.contains("\"selected_utxos\"")
        });
        respond(then, 200, rpc_ok(1, send_result()));
    });

    let client = Client::new(server.url("/"));
    let params = TokenSendParams {
        account: 0,
        token_id: "mmltk1x".to_string(),
        address: "mtc1qy".to_string(),
        amount: Amount::from_atoms(700),
        options: TxOptions::default(),
    };
    let result = client.send_token(params).await.unwrap();

    assert_eq!(result.tx_id, "f0f1f2");
    assert_eq!(result.fees.coins.atoms(), Some(100));
    assert_eq!(result.fees.coins.decimal(), Some("0.000000001"));
    assert!(result.fees.tokens.is_empty());
    assert!(result.broadcasted);
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn issue_token_wire_shape_and_result() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/").matches(|req| {
            let body =
                std::str::from_utf8(req.body.as_deref().unwrap_or_default()).unwrap_or_default();
            body.contains("\"method\":\"token_issue_new\"")
                && body.contains("\"token_ticker\":\"MTK\"")
                && body.contains("\"token_supply\":{\"type\":\"Lockable\"}")
                && body.contains("\"is_freezable\":false")
        });
        respond(
            then,
            200,
            rpc_ok(1, json!({ "token_id": "mmltk1new", "tx_id": "aabb" })),
        );
    });

    let client = Client::new(server.url("/"));
    let params = IssueTokenParams {
        account: 0,
        destination_address: "mtc1qauthority".to_string(),
        metadata: TokenMetadata {
            token_ticker: "MTK".to_string(),
            number_of_decimals: 2,
            metadata_uri: "https://example.com/token.json".to_string(),
            token_supply: TokenSupply::Lockable,
            is_freezable: false,
        },
        options: TxOptions::default(),
    };
    let result = client.issue_token(params).await.unwrap();

    assert_eq!(result.token_id, "mmltk1new");
    assert_eq!(result.tx_id, "aabb");
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn token_supply_serde_forms() {
    let fixed = serde_json::to_string(&TokenSupply::Fixed(Amount::from_atoms(5))).unwrap();
    assert!(fixed.contains("\"type\":\"Fixed\""), "got {fixed}");
    assert!(fixed.contains("\"atoms\":\"5\""), "got {fixed}");

    assert_eq!(
        serde_json::to_string(&TokenSupply::Lockable).unwrap(),
        r#"{"type":"Lockable"}"#
    );
    assert_eq!(
        serde_json::to_string(&TokenSupply::Unlimited).unwrap(),
        r#"{"type":"Unlimited"}"#
    );

    let fixed: TokenSupply =
        serde_json::from_str(r#"{"type":"Fixed","content":{"atoms":"5"}}"#).unwrap();
    assert_eq!(fixed, TokenSupply::Fixed(Amount::from_atoms(5)));
    let lockable: TokenSupply = serde_json::from_str(r#"{"type":"Lockable"}"#).unwrap();
    assert_eq!(lockable, TokenSupply::Lockable);
    let unlimited: TokenSupply = serde_json::from_str(r#"{"type":"Unlimited"}"#).unwrap();
    assert_eq!(unlimited, TokenSupply::Unlimited);
}
