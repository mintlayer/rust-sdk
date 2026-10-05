// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer transaction endpoints.

use serde::Deserialize;

use super::{Error, MerklePath, OffsetMode, Page, PageOpts, Pager, Transaction};
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

    /// Lists transactions along the global keyset cursor walk
    /// (`GET /transaction` with `cursor`), newest block first, in block
    /// order within each block.
    ///
    /// When `cursor` is `None` the walk starts from the beginning. A
    /// cursor cannot be combined with an `offset_mode` (the server rejects
    /// the pair with [`Error::BadRequest`]), so this method takes no
    /// offset; for offset-based listings use [`Client::list_transactions`]
    /// or [`Client::list_transactions_with_offset_mode`]. A cursor
    /// silently overrides the `offset` page position on the server, so
    /// this method takes no offset either.
    pub async fn list_transactions_paged(
        &self,
        cursor: Option<&str>,
        items: u32,
    ) -> Result<Page<Transaction>, Error> {
        let query = [
            ("cursor", cursor.unwrap_or("").to_owned()),
            ("items", crate::indexer::clamp_items(items).to_string()),
        ];
        self.get("/transaction", &query).await
    }

    /// Lists transactions with explicit offset semantics and no cursor
    /// (`GET /transaction` with `offset_mode`). [`OffsetMode::Legacy`] is
    /// the server default (see [`Client::list_transactions`]);
    /// [`OffsetMode::Absolute`] treats `offset` as a global transaction
    /// index, stable across scanner catch-up.
    pub async fn list_transactions_with_offset_mode(
        &self,
        mode: OffsetMode,
        opts: PageOpts,
    ) -> Result<Vec<Transaction>, Error> {
        let mut query = crate::indexer::page_query(opts.offset, opts.items);
        query.push(("offset_mode", mode.as_str().to_owned()));
        self.get("/transaction", &query).await
    }

    /// Walks the global transaction listing page by page, `items`
    /// transactions per page (clamped to 1..=100).
    pub fn transactions_pager(&self, items: u32) -> Pager<Transaction> {
        let items = crate::indexer::clamp_items(items);
        let client = self.clone();
        Pager::new(move |cursor| {
            let client = client.clone();
            Box::pin(async move { client.list_transactions_paged(cursor.as_deref(), items).await })
        })
    }
}
