// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer statistics endpoints.

use super::{CoinStats, Error, Holder, HoldersOpts, Page, Pager};
use crate::indexer::Client;

impl Client {
    /// Returns global coin statistics (`GET /statistics/coin`).
    pub async fn coin_statistics(&self) -> Result<CoinStats, Error> {
        self.get("/statistics/coin", &[]).await
    }

    /// Returns statistics for a token (`GET /statistics/token/{token_id}`).
    pub async fn token_statistics(&self, token_id: &str) -> Result<CoinStats, Error> {
        let token_id = super::validate_segment(token_id)?;
        self.get(&format!("/statistics/token/{token_id}"), &[]).await
    }

    /// Returns the fee rate in atoms per kilobyte
    /// (`GET /feerate`). `in_top_x_mb` selects the mempool fill level;
    /// zero uses the server default of 5 MB.
    pub async fn fee_rate(&self, in_top_x_mb: u32) -> Result<String, Error> {
        let query = if in_top_x_mb == 0 {
            Vec::new()
        } else {
            vec![("in_top_x_mb", in_top_x_mb.to_string())]
        };
        self.get("/feerate", &query).await
    }

    /// Lists the holders of the native coin, largest balance first
    /// (`GET /statistics/coin/holders`). Amounts are rendered with the
    /// coin's decimals.
    pub async fn coin_holders(&self, opts: HoldersOpts) -> Result<Page<Holder>, Error> {
        let mut query = crate::indexer::page_query(opts.offset, opts.items);
        if let Some(cursor) = opts.cursor {
            query.push(("cursor", cursor));
        }
        self.get("/statistics/coin/holders", &query).await
    }

    /// Lists the holders of a token, largest balance first
    /// (`GET /statistics/token/{token_id}/holders`). Amounts are rendered
    /// with the token's decimals; an unknown token is rejected with
    /// [`Error::TokenNotFound`].
    pub async fn token_holders(
        &self,
        token_id: &str,
        opts: HoldersOpts,
    ) -> Result<Page<Holder>, Error> {
        let token_id = super::validate_segment(token_id)?;
        let mut query = crate::indexer::page_query(opts.offset, opts.items);
        if let Some(cursor) = opts.cursor {
            query.push(("cursor", cursor));
        }
        self.get(&format!("/statistics/token/{token_id}/holders"), &query).await
    }

    /// Walks the native coin holders page by page, `items` holders per
    /// page (clamped to 1..=100).
    pub fn coin_holders_pager(&self, items: u32) -> Pager<Holder> {
        let items = crate::indexer::clamp_items(items);
        let client = self.clone();
        Pager::new(move |cursor| {
            let client = client.clone();
            Box::pin(async move {
                client
                    .coin_holders(HoldersOpts {
                        offset: 0,
                        items,
                        cursor,
                    })
                    .await
            })
        })
    }

    /// Walks a token's holders page by page, `items` holders per page
    /// (clamped to 1..=100). The token id is validated up front, so the
    /// walk itself cannot fail with an invalid URL.
    pub fn token_holders_pager(&self, token_id: &str, items: u32) -> Result<Pager<Holder>, Error> {
        let token_id = super::validate_segment(token_id)?.to_owned();
        let items = crate::indexer::clamp_items(items);
        let client = self.clone();
        Ok(Pager::new(move |cursor| {
            let (client, token_id) = (client.clone(), token_id.clone());
            Box::pin(async move {
                client
                    .token_holders(
                        &token_id,
                        HoldersOpts {
                            offset: 0,
                            items,
                            cursor,
                        },
                    )
                    .await
            })
        }))
    }
}
