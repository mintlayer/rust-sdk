// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer DEX order endpoints.

use super::{Error, Order, PageOpts};
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
}
