// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer stake-pool endpoints.

use serde::Deserialize;

use super::{Error, Pool, PoolDelegation, PoolListOpts};
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
        self.get(&format!("/pool/{id}/delegations"), &[]).await
    }
}
