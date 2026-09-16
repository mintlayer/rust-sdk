// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer chain endpoints.

use super::{ChainTip, Error, GenesisInfo};
use crate::indexer::Client;

impl Client {
    /// Returns the tip of the main chain (`GET /chain/tip`).
    pub async fn tip(&self) -> Result<ChainTip, Error> {
        self.get("/chain/tip", &[]).await
    }

    /// Returns the genesis block and consensus message
    /// (`GET /chain/genesis`).
    pub async fn genesis(&self) -> Result<GenesisInfo, Error> {
        self.get("/chain/genesis", &[]).await
    }

    /// Returns the hex-encoded block id at a height
    /// (`GET /chain/{height}`).
    pub async fn block_id_at_height(&self, height: u64) -> Result<String, Error> {
        self.get(&format!("/chain/{height}"), &[]).await
    }
}
