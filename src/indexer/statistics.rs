// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer statistics endpoints.

use super::{CoinStats, Error};
use crate::indexer::Client;

impl Client {
    /// Returns global coin statistics (`GET /statistics/coin`).
    pub async fn coin_statistics(&self) -> Result<CoinStats, Error> {
        self.get("/statistics/coin", &[]).await
    }

    /// Returns statistics for a token (`GET /statistics/token/{token_id}`).
    pub async fn token_statistics(&self, token_id: &str) -> Result<CoinStats, Error> {
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
}
