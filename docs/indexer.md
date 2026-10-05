# Indexer client

The `indexer` module is a REST client for the Mintlayer indexer
(`api-web-server`). All paths are relative to `/api/v2/`, which the client
appends to the base URL automatically.

```rust
use mintlayer_sdk::indexer::{self, PageOpts, PoolListOpts, PoolSort};

let c = indexer::Client::new("http://127.0.0.1:3000");

// Or with options:
let c = indexer::Client::builder("http://127.0.0.1:3000")
    .timeout(std::time::Duration::from_secs(15)) // optional, default 30s
    .http_client(reqwest::Client::new())         // optional
    .build()?;
```

**Default ports:** 3000 (mainnet), 13000 (testnet).

The indexer API is unauthenticated; there are no credential options.

Errors are returned as `indexer::Error`:

- `Error::Http { status_code, body }` — any non-2xx response that does not
  match a known server error below; the body is trimmed and stripped of
  control characters.
- `Error::InvalidCursor` — `400 "Invalid cursor"`: the cursor is malformed
  or oversized, belongs to a different listing, or was minted for the other
  side of an order book.
- `Error::InvalidNumItems` — `400 "Invalid number of items"`: every
  paginated v2 endpoint accepts 1..=100 items.
- `Error::BadRequest` — `400 "Bad request"`: mutually incompatible query
  parameters (a cursor together with `offset_mode`, or a cursor with a
  non-default pools sort).
- `Error::TokenNotFound` — `404 "Token not found"`.
- `Error::Transport(reqwest::Error)` — the request failed.
- `Error::Json(serde_json::Error)` — the response could not be decoded.
- `Error::InvalidUrl { message }` — a path segment contained characters
  outside `[A-Za-z0-9_-]` (ids, addresses and tickers are validated before
  being interpolated into the path).
- `Error::ResponseTooLarge { limit }` — the response exceeded 64 MiB.

---


> **Transport security:** loopback `http://` is fine; the indexer API is
> unauthenticated. For remote indexers prefer an `https://` URL so response
> data cannot be tampered with in transit.

## Pagination

List endpoints take `PageOpts`:

```rust
use mintlayer_sdk::indexer::PageOpts;

let opts = PageOpts { offset: 20, items: 10 };
// zero values are omitted from the query; the server defaults
// to offset 0 and 10 items per page
```

`PoolListOpts` extends the same pagination with a `sort` field
(`PoolSort::ByHeight`, the server default, or `PoolSort::ByPledge`).

### Cursor (keyset) pagination

The pools listing, the global transaction listing, both holders listings,
and the order book also support keyset pagination: with a `cursor` in the
request the response is the envelope
`Page<T> { items, next_cursor }`, where `next_cursor` is the opaque cursor
of the last returned item and is `None` on the last page. Without a cursor
parameter the pools and transaction listings return a plain array (the
offset methods), while the holders listings and the order book always
return the envelope.
Cursors are minted by the indexer — never construct or mutate one; pass it
back verbatim. A cursor silently overrides the `offset` page position on
the server (`items` still applies).

```rust
use mintlayer_sdk::indexer::Client;

let c = Client::new("http://127.0.0.1:3000");

// Page by page:
let mut cursor = None;
loop {
    let page = c.list_pools_paged(cursor.as_deref(), 100).await?;
    for pool in &page.items { /* ... */ }
    match page.next_cursor {
        Some(next) => cursor = Some(next),
        None => break,
    }
}

// Or item by item with the shared pager:
let mut pools = c.pools_pager(100);
while let Some(pool) = pools.next().await {
    let pool = pool?;
}
```

Per-endpoint rules (the server enforces them; the errors are typed):

- **Pools** — the cursor walk only exists for the default `by_height`
  sort; a cursor combined with any other sort is rejected with
  `Error::BadRequest`. For `by_pledge` use `list_pools`.
- **Transactions** — a cursor cannot be combined with `offset_mode`
  (`Error::BadRequest`). For offset-based listings use `list_transactions`
  or `list_transactions_with_offset_mode` (`OffsetMode::Legacy`, the
  server default, or `OffsetMode::Absolute`, which treats the offset as a
  stable global transaction index).
- **Order book** — cursors are side-specific: an ask-side cursor on a bid
  walk is rejected with `Error::InvalidCursor`. When the per-request cap
  of 10,000 aggregated orders truncates the book, the response carries
  `truncated: true` and `next_cursor: null` — the walk cannot be
  continued, and the `*_pager` stops there (see `OrderBook`).

Semantics worth knowing on every cursor walk: pages are only guaranteed
stable once the indexer's scanner is fully caught up — during catch-up or
a reorg a walk may skip or repeat an entry — and the order book is
computed fresh on every request, so a walk is not a consistent snapshot of
a moving book. Offset-based methods remain the simple alternative for
shallow listings.

---

## Chain

| Method | REST path | Notes |
|--------|-----------|-------|
| `tip() -> Result<ChainTip, Error>` | `GET /chain/tip` | `ChainTip { block_height, block_id }` |
| `genesis() -> Result<GenesisInfo, Error>` | `GET /chain/genesis` | Genesis block id, message, UTXO set |
| `block_id_at_height(height: u64) -> Result<String, Error>` | `GET /chain/{height}` | 404 `Error::Http` beyond the tip |

## Blocks

| Method | REST path | Notes |
|--------|-----------|-------|
| `block(id: &str) -> Result<Block, Error>` | `GET /block/{id}` | Header, reward outputs, transactions |
| `block_header(id: &str) -> Result<BlockHeader, Error>` | `GET /block/{id}/header` | Cheaper than `block` |
| `block_reward(id: &str) -> Result<Vec<serde_json::Value>, Error>` | `GET /block/{id}/reward` | Raw JSON outputs |
| `block_transaction_ids(id: &str) -> Result<Vec<String>, Error>` | `GET /block/{id}/transaction-ids` | |

## Addresses

| Method | REST path | Notes |
|--------|-----------|-------|
| `address_info(address: &str) -> Result<AddressInfo, Error>` | `GET /address/{address}` | Coin and token balances, history |
| `spendable_utxos(address: &str) -> Result<Vec<Utxo>, Error>` | `GET /address/{address}/spendable-utxos` | Confirmed, immediately spendable |
| `all_utxos(address: &str) -> Result<Vec<Utxo>, Error>` | `GET /address/{address}/all-utxos` | Includes locked outputs |
| `delegations(address: &str) -> Result<Vec<DelegationInfo>, Error>` | `GET /address/{address}/delegations` | |
| `token_authority(address: &str) -> Result<Vec<String>, Error>` | `GET /address/{address}/token-authority` | Token ids the address controls |

## Transactions

| Method | REST path | Notes |
|--------|-----------|-------|
| `list_transactions(opts: PageOpts) -> Result<Vec<Transaction>, Error>` | `GET /transaction` | Newest first |
| `list_transactions_paged(cursor, items) -> Result<Page<Transaction>, Error>` | `GET /transaction` | Cursor walk; `None` starts from the beginning |
| `list_transactions_with_offset_mode(mode, opts) -> Result<Vec<Transaction>, Error>` | `GET /transaction` | Explicit offset semantics, no cursor |
| `transactions_pager(items) -> Pager<Transaction>` | `GET /transaction` | Item-wise cursor walk |
| `transaction(id: &str) -> Result<Transaction, Error>` | `GET /transaction/{id}` | Pending transactions carry `None` fields, see below |
| `transaction_merkle_path(id: &str) -> Result<MerklePath, Error>` | `GET /transaction/{id}/merkle-path` | 404 before confirmation |
| `transaction_output(tx_id: &str, index: u32) -> Result<serde_json::Value, Error>` | `GET /transaction/{tx_id}/output/{index}` | Raw JSON |
| `submit_transaction(signed_tx_hex: &str) -> Result<String, Error>` | `POST /transaction` | Requires `--enable-post-routes`; returns the tx id |

**Quirk:** for pending (mempool) transactions the `Transaction` fields
`block_id`, `timestamp` and `confirmations` are `None`; on the global
listing a pending transaction has `block_id: None` while confirmed ones
carry the block hash.

## Pools and delegations

| Method | REST path | Notes |
|--------|-----------|-------|
| `list_pools(opts: PoolListOpts) -> Result<Vec<Pool>, Error>` | `GET /pool` | `sort=by_height` (default) or `by_pledge` |
| `list_pools_paged(cursor, items) -> Result<Page<Pool>, Error>` | `GET /pool` | Cursor walk (creation-height sort only) |
| `pools_pager(items) -> Pager<Pool>` | `GET /pool` | Item-wise cursor walk |
| `pool(id: &str) -> Result<Pool, Error>` | `GET /pool/{id}` | |
| `pool_block_stats(id: &str, from: i64, to: i64) -> Result<u64, Error>` | `GET /pool/{id}/block-stats` | Block count in `[from, to)`, UNIX seconds |
| `pool_delegations(id: &str) -> Result<Vec<PoolDelegation>, Error>` | `GET /pool/{id}/delegations` | Includes `creation_block_height` |
| `delegation(id: &str) -> Result<Delegation, Error>` | `GET /delegation/{id}` | |

## Tokens and NFTs

| Method | REST path | Notes |
|--------|-----------|-------|
| `list_tokens(opts: PageOpts) -> Result<Vec<String>, Error>` | `GET /token` | Bech32 token ids |
| `token(id: &str) -> Result<TokenInfo, Error>` | `GET /token/{id}` | |
| `token_transactions(id: &str, opts: PageOpts) -> Result<Vec<TokenTx>, Error>` | `GET /token/{id}/transactions` | |
| `tokens_by_ticker(ticker: &str, opts: PageOpts) -> Result<Vec<String>, Error>` | `GET /token/ticker/{ticker}` | Tickers are not unique |
| `nft(id: &str) -> Result<NftInfo, Error>` | `GET /nft/{id}` | Owner, token id, metadata |

## Orders

| Method | REST path | Notes |
|--------|-----------|-------|
| `list_orders(opts: PageOpts) -> Result<Vec<Order>, Error>` | `GET /order` | |
| `order(id: &str) -> Result<Order, Error>` | `GET /order/{id}` | |
| `orders_by_pair(ask, give, opts) -> Result<Vec<Order>, Error>` | `GET /order/pair/{ask}_{give}` | Currencies are the coin ticker (e.g. `ML`) or a bech32 token id |
| `order_pair_book(base, quote, side, opts) -> Result<OrderBook, Error>` | `GET /order/pair/{base}_{quote}/book` | Aggregated price levels; `side` is required; see `OrderBook` invariants |
| `order_book_pager(base, quote, side, items) -> Result<Pager<OrderBookLevel>, Error>` | | Item-wise cursor walk; stops on a truncated book |

## Statistics and fees

| Method | REST path | Notes |
|--------|-----------|-------|
| `coin_statistics() -> Result<CoinStats, Error>` | `GET /statistics/coin` | Circulating, preminted, burned, staked — all four counters are always present; unwritten counters are zero |
| `token_statistics(token_id: &str) -> Result<CoinStats, Error>` | `GET /statistics/token/{token_id}` | Same counters; 404 `Error::TokenNotFound` for unknown tokens |
| `coin_holders(opts: HoldersOpts) -> Result<Page<Holder>, Error>` | `GET /statistics/coin/holders` | Largest balance first; amounts in coin decimals |
| `token_holders(token_id, opts) -> Result<Page<Holder>, Error>` | `GET /statistics/token/{token_id}/holders` | Amounts in token decimals |
| `coin_holders_pager(items) / token_holders_pager(token_id, items)` | | Item-wise cursor walks |
| `fee_rate(in_top_x_mb: u32) -> Result<String, Error>` | `GET /feerate` | Atoms per KB; `0` uses the server default of 5 MB |

---

## Amounts and string-encoded numbers

The indexer serializes some numeric fields as JSON strings or mixed
representations. The client types absorb this transparently, but be aware
of the shapes:

- `Amount` carries both fields: `atoms: u128` and `decimal: String`
  (whole coins).
- `Uint64` fields (`next_nonce`, `creation_block_height`, `Order::nonce`)
  may arrive as a JSON integer or a decimal string; `.0` gives the `u64`,
  `Display` prints the number.
- `PerThousand` (pool `margin_ratio_per_thousand`) may arrive as a number
  or as a decimal string with an optional trailing `%`.

Example:

```rust
use mintlayer_sdk::indexer::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let c = Client::new("http://127.0.0.1:3000");

    let tip = c.tip().await?;
    println!("tip: {} at height {}", tip.block_id, tip.block_height);

    let pool = c.pool("mpool1...").await?;
    println!(
        "staker {} delegated {}",
        pool.staker_balance.decimal, pool.delegations_balance.decimal
    );

    // Submit a signed transaction (requires --enable-post-routes).
    // let tx_id = c.submit_transaction(&signed_hex).await?;
    Ok(())
}
```
