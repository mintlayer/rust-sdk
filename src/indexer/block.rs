// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer block endpoints.

use super::{Block, BlockHeader, Error};
use crate::indexer::Client;

impl Client {
    /// Returns a block with header and body (`GET /block/{id}`).
    pub async fn block(&self, id: &str) -> Result<Block, Error> {
        self.get(&format!("/block/{id}"), &[]).await
    }

    /// Returns only the header of a block (`GET /block/{id}/header`).
    pub async fn block_header(&self, id: &str) -> Result<BlockHeader, Error> {
        self.get(&format!("/block/{id}/header"), &[]).await
    }

    /// Returns the reward outputs of a block (`GET /block/{id}/reward`).
    pub async fn block_reward(&self, id: &str) -> Result<Vec<serde_json::Value>, Error> {
        self.get(&format!("/block/{id}/reward"), &[]).await
    }

    /// Returns the ids of the transactions in a block
    /// (`GET /block/{id}/transaction-ids`).
    pub async fn block_transaction_ids(&self, id: &str) -> Result<Vec<String>, Error> {
        self.get(&format!("/block/{id}/transaction-ids"), &[]).await
    }
}
