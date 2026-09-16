# Mintlayer Rust SDK

A Rust SDK for the [Mintlayer](https://www.mintlayer.org/) blockchain.

```toml
[dependencies]
mintlayer-sdk = { git = "https://github.com/mintlayer/rust-sdk", features = ["full"] }
```

Requires Rust 1.92+ (edition 2024). Async (tokio + reqwest/rustls). No unsafe code.

> **Consumer requirement:** because the SDK depends on unpublished
> mintlayer-core crates, Cargo ignores this repository's `[patch.crates-io]`
> section in downstream builds. Consumers MUST copy the `parity-scale-codec`
> git patch (see the root `Cargo.toml`) into their own workspace, or
> transactions encoded by the `crypto` feature may not match the encodings
> produced by mintlayer-core.

---

## Documentation

| Guide | Description |
|-------|-------------|
| [docs/indexer.md](docs/indexer.md) | Full indexer client reference: chain, blocks, transactions, addresses, pools, tokens, orders, statistics |
| [docs/node.md](docs/node.md) | Full node client reference: chainstate, mempool, P2P, block submission |
| [docs/wallet.md](docs/wallet.md) | Full wallet client reference: lifecycle, accounts, balances, transactions, tokens, orders |
| [docs/crypto.md](docs/crypto.md) | Native cryptography reference: keys, addresses, inputs, outputs, signing |
| [docs/transactions.md](docs/transactions.md) | Step-by-step guide to building and signing transactions without the wallet daemon |
| [docs/staking.md](docs/staking.md) | Staking pools and delegations: creation, funding, withdrawal |
| [docs/tokens.md](docs/tokens.md) | Fungible token and NFT lifecycle: issuance, minting, freezing, authority |

---

## Overview

The SDK is organised as three remote sub-clients plus a native cryptography
module, and a top-level `Client` that wires the sub-clients together.

| Module | Purpose | Default port |
|---|---|---|
| `node` *(feature)* | JSON-RPC 2.0 client for the node daemon | 3030 (mainnet) |
| `indexer` *(feature)* | REST client for the indexer (api-web-server) | 3000 |
| `wallet` *(feature)* | JSON-RPC 2.0 client for the wallet daemon | 3034 (mainnet) |
| `crypto` *(feature)* | Cryptography & transaction building, backed by mintlayer-core natively | — |

Features: default is `node`, `indexer`, `wallet`; `crypto` is opt-in because
it pulls in the mintlayer-core dependency graph; `full` enables everything.

Unlike the go-sdk there is no embedded WASM runtime, no `InitWASM` and no
`Close`: the cryptography module calls mintlayer-core directly and exchanges
typed values (`Transaction`, `TxOutput`, `PrivateKey`, `Amount`, ...) instead
of opaque byte arrays. Every type participates in the SCALE encoding, and
`mintlayer_sdk::crypto::types::{Encode, DecodeAll}` provide byte-level access.

---

## Quick start

```rust
use mintlayer_sdk::{Client, crypto};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .node_url("http://127.0.0.1:3030")
        .indexer_url("http://127.0.0.1:3000")
        .wallet_url("http://127.0.0.1:3034")
        .build()?;

    // Query the chain tip from the indexer.
    let tip = client.indexer.as_ref().unwrap().tip().await?;
    println!("chain tip: height={} id={}", tip.block_height, tip.block_id);

    // Derive an address natively (no runtime initialisation).
    let account = crypto::make_default_account_privkey(MNEMONIC, crypto::Network::Mainnet, None)?;
    let spend = crypto::make_receiving_address(&account, 0)?;
    let address = crypto::pubkey_to_pubkeyhash_address(
        &crypto::public_key_from_private_key(&spend),
        crypto::Network::Mainnet,
    );
    println!("address: {address}");
    Ok(())
}

const MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
```

---

## Node client (`node`)

JSON-RPC 2.0 client for the Mintlayer node daemon. Supports basic auth for
nodes with authentication enabled.

```rust
use mintlayer_sdk::node::{self, TrustPolicy};

let c = node::Client::builder("http://127.0.0.1:3030")
    .basic_auth("user", "pass")   // optional
    .timeout(std::time::Duration::from_secs(10)) // optional
    .build()?;

// Chain state
let info = c.chainstate_info().await?;
let height = c.best_block_height().await?;
let block_id = c.best_block_id().await?;

// Look up a block
let block_hex = c.block(&block_id).await?;
let block_json = c.block_json(&block_id).await?;

// Token / order info
let token = c.token_info("mmltk1...").await?;
let order = c.order_info("mordr1...").await?;

// Mempool
c.submit_transaction(&signed_tx_hex, TrustPolicy::Untrusted).await?;
let fee_rate = c.fee_rate(1).await?;

// P2P
let peers = c.peer_count().await?;
c.broadcast_transaction(&signed_tx_hex, TrustPolicy::Untrusted).await?;
```

Daemon errors are returned as `node::Error::Rpc { code, message }`.

## Indexer client (`indexer`)

REST client for `api-web-server`. All paths are relative to `/api/v2/`.
The indexer API is unauthenticated.

```rust
use mintlayer_sdk::indexer::{self, PageOpts, PoolListOpts, PoolSort};

let c = indexer::Client::builder("http://127.0.0.1:3000")
    .timeout(std::time::Duration::from_secs(15))
    .build()?;

// Chain
let tip = c.tip().await?;
let id_at = c.block_id_at_height(100_000).await?;

// Address
let utxos = c.spendable_utxos("mtc1q...").await?;
let info = c.address_info("mtc1q...").await?;

// Pool / staking
let pools = c.list_pools(PoolListOpts { sort: Some(PoolSort::ByPledge), ..Default::default() }).await?;
let pool = c.pool("mpool1...").await?;

// Tokens
let token = c.token("mmltk1...").await?;
let tokens = c.tokens_by_ticker("MYTOKEN", PageOpts { items: 10, ..Default::default() }).await?;

// Orders
let orders = c.list_orders(PageOpts::default()).await?;

// Statistics
let stats = c.coin_statistics().await?;

// Submit a signed transaction (requires --enable-post-routes)
let tx_id = c.submit_transaction(&signed_tx_hex).await?;
```

Non-2xx responses are returned as `indexer::Error::Http { status_code, body }`.

## Wallet client (`wallet`)

JSON-RPC 2.0 client for `wallet-rpc-daemon`. The wallet daemon manages key
storage, signing and broadcasting.

```rust
use mintlayer_sdk::wallet::{self, Amount, IssueTokenParams, MintParams, TokenMetadata, TokenSupply, TxOptions};

let c = wallet::Client::builder("http://127.0.0.1:3034")
    .basic_auth("user", "pass")   // optional
    .build()?;

// Wallet lifecycle
c.open_wallet("/path/to/wallet.dat", None).await?;
c.sync_wallet().await?;

// Accounts and addresses
let info = c.wallet_info().await?;
let address = c.new_address(0).await?;
let balance = c.balance(0).await?;

// Send coins (account 0)
let result = c.send(wallet::SendParams {
    account: 0,
    address: "mtc1q...".into(),
    amount: Amount::from_atoms(100_000_000_000), // 1 ML
    selected_utxos: vec![],
    options: TxOptions::default(),
}).await?;
println!("tx id: {}", result.tx_id);

// Token operations
let issue = c.issue_token(IssueTokenParams {
    account: 0,
    destination_address: address.clone(),
    metadata: TokenMetadata {
        token_ticker: "MYTOKEN".into(),
        number_of_decimals: 2,
        metadata_uri: "https://example.com/token".into(),
        token_supply: TokenSupply::Lockable,
        is_freezable: false,
    },
    options: TxOptions::default(),
}).await?;

let mint = c.mint_tokens(MintParams {
    account: 0,
    token_id: issue.token_id,
    address,
    amount: Amount::from_atoms(1000),
    options: TxOptions::default(),
}).await?;

// Staking
c.start_staking(0).await?;
let pools = c.list_owned_pools(0).await?;

// Compose and sign a raw transaction (cold wallet flow)
let composed = c.compose_transaction(params).await?;
let signed = c.sign_raw_transaction(0, &composed.hex).await?;
let submitted = c.submit_transaction(&signed.hex, false).await?;
```

Daemon errors are returned as `wallet::Error::Rpc { code, message }`.

## Cryptography (`crypto`)

Native key management and transaction building. The functions mirror the
go-sdk `wasm` sub-client operation for operation; see
[docs/crypto.md](docs/crypto.md) and
[docs/transactions.md](docs/transactions.md) for full walkthroughs.

```rust
use mintlayer_sdk::crypto::{self, Amount, Network, SigHashType, SourceId, TxAdditionalInfo};
use mintlayer_sdk::crypto::types::*;

let account = crypto::make_default_account_privkey(MNEMONIC, Network::Mainnet, None)?;
let spend = crypto::make_receiving_address(&account, 0)?;
let pubkey = crypto::public_key_from_private_key(&spend);
let address = crypto::pubkey_to_pubkeyhash_address(&pubkey, Network::Mainnet);

let outpoint = crypto::encode_outpoint_source_id(hash_of_txid, SourceId::Transaction);
let input = crypto::encode_input_for_utxo(outpoint, 0);
let output = crypto::encode_output_transfer(Amount::from_atoms(100_000_000_000), &to, Network::Mainnet)?;
let tx = crypto::encode_transaction(vec![input], outputs, 0)?;
let tx_id = crypto::transaction_id(&tx);

let witness = crypto::encode_witness(
    SigHashType::all(), &spend, &address, &tx,
    &[Some(output)], 0, &TxAdditionalInfo::new(), tip_height + 1, Network::Mainnet,
)?;
let signed = crypto::encode_signed_transaction(tx, vec![witness])?;
let signed_hex = hex::encode(signed.encode());
```

---

## Top-level client

`Client::builder()` creates only the sub-clients whose URL is set.
Basic-auth credentials apply to the node and wallet clients only; the
indexer API is unauthenticated.

```rust
// Node + wallet only — no indexer client is created.
let client = Client::builder()
    .node_url("http://127.0.0.1:3030")
    .wallet_url("http://127.0.0.1:3034")
    .basic_auth("user", "pass")
    .build()?;
```

`mintlayer_sdk::prelude` re-exports the shared crypto types (`Amount`,
`Network`, `SigHashType`, `TokenTotalSupply`, key and transaction types, the
SCALE `Encode`/`DecodeAll` traits, ...) so single-import callers do not need
to reach into `crypto`.

---

## Examples

| Example | Description |
|---|---|
| [examples/send-coins](examples/send-coins.rs) | Derive key → fetch UTXOs → build, sign and submit a transaction |
| [examples/issue-token](examples/issue-token.rs) | Issue a fungible token and mint an initial supply via the wallet daemon |

```text
cargo run --example send-coins --features crypto,indexer -- --mnemonic "..." --to mtc1q... --amount 100000000000 --network testnet
cargo run --example issue-token --features wallet -- --wallet /tmp/wallet.dat --ticker MYTOKEN --supply 1000000
```

---

## Amounts

All coin and token amounts are atom counts — the smallest indivisible unit.
**1 ML = 100,000,000,000 atoms** (11 decimal places).

- The `crypto` module uses `common::primitives::Amount`, a `u128` atom value.
- The remote clients model the daemon wire format: the node returns
  `{"atoms": "..."}` objects decoded into a `u128`-backed `Amount`; the
  indexer/wallet amounts carry `atoms` and `decimal` fields.

---

## Networks

| `crypto::Network` | Use |
|---|---|
| `Network::Mainnet` | Production network |
| `Network::Testnet` | Public test network |
| `Network::Regtest` | Local regression testing |
| `Network::Signet` | Signet |

Every crypto operation that derives addresses or is fork-sensitive takes the
network (and, where applicable, the predicted inclusion block height) as a
parameter.

---

## Error handling

- `node::Error` / `wallet::Error` — `Rpc { code, message }` for JSON-RPC
  errors, plus transport/decoding variants.
- `indexer::Error` — `Http { status_code, body }` for non-2xx responses,
  plus transport/decoding variants.
- `crypto::Error` — typed variants for key, address, signing and encoding
  failures.

Transport hardening: request ids are atomic and unique per client, basic
auth is redacted from `Debug` output, and response bodies are capped at
64 MiB (error bodies are additionally truncated and stripped of control
characters).

---

## Supply chain

The `crypto` feature consumes mintlayer-core crates pinned to an exact
commit of `github.com/mintlayer/mintlayer-core`; the SCALE codec is pinned
to `github.com/paritytech/parity-scale-codec` at the same rev mintlayer-core
uses (an unreleased fix — see the root `Cargo.toml` for the mandatory
consumer patch). `mintlayer-core-primitives` is pulled transitively from
`github.com/mintlayer/mintlayer-core-primitives`. Keep the committed
`Cargo.lock` for reproducible builds.

## License

MIT — see [LICENSE](LICENSE).
