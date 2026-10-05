// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer DEX order endpoints.

use super::{
    Error, Order, OrderBook, OrderBookLevel, OrderBookOpts, OrderBookSide, Page, PageOpts, Pager,
};
use crate::indexer::Client;

impl Client {
    /// Lists DEX orders (`GET /order`).
    pub async fn list_orders(&self, opts: PageOpts) -> Result<Vec<Order>, Error> {
        self.get(
            "/order",
            &crate::indexer::page_query(opts.offset, opts.items),
        )
        .await
    }

    /// Returns an order by its bech32 id (`GET /order/{id}`).
    pub async fn order(&self, id: &str) -> Result<Order, Error> {
        let id = super::validate_segment(id)?;
        self.get(&format!("/order/{id}"), &[]).await
    }

    /// Lists orders by currency pair; currencies are the coin ticker (e.g.
    /// `ML`) or a bech32 token id (`GET /order/pair/{ask}_{give}`).
    pub async fn orders_by_pair(
        &self,
        ask_currency: &str,
        give_currency: &str,
        opts: PageOpts,
    ) -> Result<Vec<Order>, Error> {
        let ask_currency = super::validate_segment(ask_currency)?;
        let give_currency = super::validate_segment(give_currency)?;
        self.get(
            &format!("/order/pair/{ask_currency}_{give_currency}"),
            &crate::indexer::page_query(opts.offset, opts.items),
        )
        .await
    }

    /// Returns one page of the order book of a trading pair
    /// (`GET /order/pair/{base}_{quote}/book`).
    ///
    /// The native coin ticker in the pair matches case-insensitively on
    /// the server, while token ids are exact bech32 strings — both
    /// currencies are sent exactly as given. Levels are aggregated
    /// remaining balances per price level, ascending on the
    /// [`ask`](OrderBookSide::Ask) side and descending on the
    /// [`bid`](OrderBookSide::Bid) side; `side` is required by the
    /// server.
    ///
    /// Cursors are side-specific: reusing an ask cursor on a bid walk is
    /// rejected with [`Error::InvalidCursor`]. A cursor silently overrides
    /// the `offset` page position on the server. See [`OrderBook`] for the
    /// truncation and snapshot invariants.
    pub async fn order_pair_book(
        &self,
        base: &str,
        quote: &str,
        side: OrderBookSide,
        opts: OrderBookOpts,
    ) -> Result<OrderBook, Error> {
        let base = super::validate_segment(base)?;
        let quote = super::validate_segment(quote)?;
        let mut query = crate::indexer::page_query(opts.offset, opts.items);
        query.push(("side", side.as_str().to_owned()));
        if let Some(cursor) = opts.cursor {
            query.push(("cursor", cursor));
        }
        self.get(&format!("/order/pair/{base}_{quote}/book"), &query).await
    }

    /// Walks one side of an order book page by page, `items` levels per
    /// page (clamped to 1..=100).
    ///
    /// The walk stops when the server reports the last page — including a
    /// book that was truncated by the server's per-request order cap,
    /// which is signalled with `next_cursor: null` (see
    /// [`OrderBook::truncated`]). Note the walk is not a consistent
    /// snapshot of a moving book.
    pub fn order_book_pager(
        &self,
        base: &str,
        quote: &str,
        side: OrderBookSide,
        items: u32,
    ) -> Result<Pager<OrderBookLevel>, Error> {
        let base = super::validate_segment(base)?.to_owned();
        let quote = super::validate_segment(quote)?.to_owned();
        let items = crate::indexer::clamp_items(items);
        let client = self.clone();
        Ok(Pager::new(move |cursor| {
            let (client, base, quote) = (client.clone(), base.clone(), quote.clone());
            Box::pin(async move {
                let book = client
                    .order_pair_book(
                        &base,
                        &quote,
                        side,
                        OrderBookOpts {
                            offset: 0,
                            items,
                            cursor,
                        },
                    )
                    .await?;
                Ok(Page {
                    items: book.items,
                    next_cursor: book.next_cursor,
                })
            })
        }))
    }
}
