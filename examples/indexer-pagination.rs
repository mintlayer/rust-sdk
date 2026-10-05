// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Cursor (keyset) pagination against the Mintlayer indexer
//! (api-server v2). Walks the four paginated listings end to end:
//!
//! 1. the native coin holders,
//! 2. both sides of an order book,
//! 3. the global transaction listing (with cursor and with
//!    `offset_mode`),
//! 4. the stake pools (creation-height cursor walk).
//!
//! Usage:
//! ```text
//! cargo run --example indexer-pagination -- http://127.0.0.1:3000
//! ```
//!
//! Point it at a local indexer (`indexer --chain-config mainnet` or a
//! testnet instance on port 13000). Requires no wallet or node.

use mintlayer_sdk::indexer::{Client, OffsetMode, OrderBookOpts, OrderBookSide, PageOpts};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let base_url = args.next().unwrap_or_else(|| "http://127.0.0.1:3000".to_owned());
    let indexer = Client::new(&base_url);
    // The native coin ticker, e.g. "ML" (mainnet) or "TMCL" (testnet); the
    // server matches the coin ticker case-insensitively while token ids in
    // a pair must be exact bech32 strings.
    let coin = std::env::var("COIN").unwrap_or_else(|_| "ML".to_owned());

    // 1. Coin holders: largest balance first. `next()` yields items across
    //    pages and stops once the server sends `next_cursor: null`.
    println!("== {coin} holders ==");
    let mut holders = indexer.coin_holders_pager(50);
    let mut count = 0u64;
    while let Some(holder) = holders.next().await {
        let holder = holder?;
        println!(
            "{:>3}. {} — {}",
            count + 1,
            holder.address,
            holder.amount.decimal
        );
        count += 1;
        if count == 10 {
            println!(
                "   ... stopping after 10 (cursor so far: {:?})",
                holders.cursor()
            );
            break;
        }
    }

    // 2. Order book, both sides. Levels are aggregated remaining balances;
    //    a truncated book (10,000-order server cap) ends the walk early.
    for side in [OrderBookSide::Ask, OrderBookSide::Bid] {
        println!("== order book {coin}/USDT, side {side:?} ==");
        match indexer.order_book_pager(&coin, "USDT", side, 20) {
            Ok(mut book) => {
                let mut levels = 0u64;
                while let Some(level) = book.next().await {
                    let level = level?;
                    println!(
                        "price {} amount {}",
                        level.price.decimal, level.amount.decimal
                    );
                    levels += 1;
                    if levels == 10 {
                        break;
                    }
                }
                if levels == 0 {
                    println!("(empty book)");
                }
            }
            Err(error) => println!("book unavailable: {error}"),
        }
    }

    // 3. Global transaction listing: cursor walk first, then the
    //    offset-based listing with explicit `offset_mode`.
    println!("== latest transactions (cursor walk) ==");
    let mut transactions = indexer.transactions_pager(10);
    for _ in 0..5 {
        match transactions.next().await {
            Some(Ok(tx)) => println!("tx {} in {:?} at {:?}", tx.id, tx.block_id, tx.timestamp),
            Some(Err(error)) => return Err(error.into()),
            None => break,
        }
    }

    println!("== latest transactions (offset_mode = absolute) ==");
    for tx in indexer
        .list_transactions_with_offset_mode(
            OffsetMode::Absolute,
            PageOpts {
                offset: 0,
                items: 5,
            },
        )
        .await?
    {
        println!("tx {} in {:?}", tx.id, tx.block_id);
    }

    // 4. Stake pools: the cursor walk only exists for the default
    //    creation-height sort (a cursor with `by_pledge` is rejected by
    //    the server). For a pledge-sorted listing use `list_pools`.
    println!("== pools (creation-height walk) ==");
    let mut pools = indexer.pools_pager(50);
    for _ in 0..5 {
        match pools.next().await {
            Some(Ok(pool)) => println!(
                "pool {} staked {}",
                pool.pool_id, pool.staker_balance.decimal
            ),
            Some(Err(error)) => return Err(error.into()),
            None => break,
        }
    }

    // One-off paged access (without the pager) is equally available:
    let page = indexer.list_pools_paged(None, 10).await?;
    println!(
        "first pool page: {} pools, next_cursor {:?}",
        page.items.len(),
        page.next_cursor
    );
    let _ = OrderBookOpts::default(); // typed request options also exist
    Ok(())
}
