// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer transaction endpoints.

use serde::Deserialize;

use super::{Error, MerklePath, PageOpts, Transaction};
use crate::indexer::Client;

impl Client {
    /// Lists transactions, newest first (`GET /transaction`).
    pub async fn list_transactions(&self, opts: PageOpts) -> Result<Vec<Transaction>, Error> {
        self.get(
            "/transaction",
            &crate::indexer::page_query(opts.offset, opts.items),
        )
        .await
    }

    /// Returns a transaction by id (`GET /transaction/{id}`).
    pub async fn transaction(&self, id: &str) -> Result<Transaction, Error> {
        let id = super::validate_segment(id)?;
        self.get(&format!("/transaction/{id}"), &[]).await
    }

    /// Returns the merkle path of a confirmed transaction
    /// (`GET /transaction/{id}/merkle-path`).
    pub async fn transaction_merkle_path(&self, id: &str) -> Result<MerklePath, Error> {
        let id = super::validate_segment(id)?;
        self.get(&format!("/transaction/{id}/merkle-path"), &[]).await
    }

    /// Returns one output of a transaction
    /// (`GET /transaction/{tx_id}/output/{index}`).
    pub async fn transaction_output(
        &self,
        tx_id: &str,
        index: u32,
    ) -> Result<serde_json::Value, Error> {
        let tx_id = super::validate_segment(tx_id)?;
        self.get(&format!("/transaction/{tx_id}/output/{index}"), &[]).await
    }

    /// Submits a hex-encoded signed transaction
    /// (`POST /transaction`). Requires the indexer to run with
    /// `--enable-post-routes`.
    pub async fn submit_transaction(&self, signed_tx_hex: &str) -> Result<String, Error> {
        #[derive(Deserialize)]
        struct Response {
            tx_id: String,
        }
        let response: Response = self.post("/transaction", signed_tx_hex).await?;
        Ok(response.tx_id)
    }
}
