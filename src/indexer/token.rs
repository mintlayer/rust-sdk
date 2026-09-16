// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer token and NFT endpoints.

use super::{Error, NftInfo, PageOpts, TokenInfo, TokenTx};
use crate::indexer::Client;

impl Client {
    /// Lists bech32 token ids (`GET /token`).
    pub async fn list_tokens(&self, opts: PageOpts) -> Result<Vec<String>, Error> {
        self.get(
            "/token",
            &crate::indexer::page_query(opts.offset, opts.items),
        )
        .await
    }

    /// Returns a token by its bech32 id (`GET /token/{id}`).
    pub async fn token(&self, id: &str) -> Result<TokenInfo, Error> {
        self.get(&format!("/token/{id}"), &[]).await
    }

    /// Returns the transfers of a token (`GET /token/{id}/transactions`).
    pub async fn token_transactions(
        &self,
        id: &str,
        opts: PageOpts,
    ) -> Result<Vec<TokenTx>, Error> {
        self.get(
            &format!("/token/{id}/transactions"),
            &crate::indexer::page_query(opts.offset, opts.items),
        )
        .await
    }

    /// Finds token ids by ticker (`GET /token/ticker/{ticker}`).
    pub async fn tokens_by_ticker(
        &self,
        ticker: &str,
        opts: PageOpts,
    ) -> Result<Vec<String>, Error> {
        self.get(
            &format!("/token/ticker/{ticker}"),
            &crate::indexer::page_query(opts.offset, opts.items),
        )
        .await
    }

    /// Returns an NFT by its bech32 token id (`GET /nft/{id}`).
    pub async fn nft(&self, id: &str) -> Result<NftInfo, Error> {
        self.get(&format!("/nft/{id}"), &[]).await
    }
}
