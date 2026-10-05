// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer stake-pool endpoints.

use serde::Deserialize;

use super::{Error, Page, Pager, Pool, PoolDelegation, PoolListOpts};
use crate::indexer::Client;

impl Client {
    /// Lists stake pools (`GET /pool`).
    pub async fn list_pools(&self, opts: PoolListOpts) -> Result<Vec<Pool>, Error> {
        let mut query = crate::indexer::page_query(opts.offset, opts.items);
        if let Some(sort) = opts.sort {
            query.push(("sort", sort.as_str().to_owned()));
        }
        self.get("/pool", &query).await
    }

    /// Returns a stake pool by its bech32 id (`GET /pool/{id}`).
    pub async fn pool(&self, id: &str) -> Result<Pool, Error> {
        let id = super::validate_segment(id)?;
        self.get(&format!("/pool/{id}"), &[]).await
    }

    /// Returns the number of blocks a pool produced in the half-open
    /// interval `[from, to)` of UNIX timestamps
    /// (`GET /pool/{id}/block-stats`).
    pub async fn pool_block_stats(&self, id: &str, from: i64, to: i64) -> Result<u64, Error> {
        #[derive(Deserialize)]
        struct Response {
            block_count: u64,
        }
        let id = super::validate_segment(id)?;
        let response: Response = self
            .get(
                &format!("/pool/{id}/block-stats"),
                &[("from", from.to_string()), ("to", to.to_string())],
            )
            .await?;
        Ok(response.block_count)
    }

    /// Returns the delegations of a stake pool
    /// (`GET /pool/{id}/delegations`).
    pub async fn pool_delegations(&self, id: &str) -> Result<Vec<PoolDelegation>, Error> {
        let id = super::validate_segment(id)?;
        self.get(&format!("/pool/{id}/delegations"), &[]).await
    }

    /// Lists stake pools along the creation-height cursor walk
    /// (`GET /pool` with `cursor`).
    ///
    /// The cursor walk only exists for the server's default `by_height`
    /// sort: combining a cursor with any other sort is rejected with
    /// [`Error::BadRequest`], so this method sends no `sort` parameter.
    /// For a `by_pledge` listing use [`Client::list_pools`].
    ///
    /// When `cursor` is `None` the walk starts from the beginning. A
    /// cursor silently overrides the `offset` page position on the server,
    /// so this method takes no offset.
    pub async fn list_pools_paged(
        &self,
        cursor: Option<&str>,
        items: u32,
    ) -> Result<Page<Pool>, Error> {
        let query = [
            ("cursor", cursor.unwrap_or("").to_owned()),
            ("items", crate::indexer::clamp_items(items).to_string()),
        ];
        self.get("/pool", &query).await
    }

    /// Walks the stake pools along the creation-height cursor walk,
    /// `items` pools per page (clamped to 1..=100). See
    /// [`Client::list_pools_paged`] for the sort restriction.
    pub fn pools_pager(&self, items: u32) -> Pager<Pool> {
        let items = crate::indexer::clamp_items(items);
        let client = self.clone();
        Pager::new(move |cursor| {
            let client = client.clone();
            Box::pin(async move { client.list_pools_paged(cursor.as_deref(), items).await })
        })
    }
}
