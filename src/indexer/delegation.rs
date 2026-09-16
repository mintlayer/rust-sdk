// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer delegation endpoints.

use super::{Delegation, Error};
use crate::indexer::Client;

impl Client {
    /// Returns a delegation by its bech32 id (`GET /delegation/{id}`).
    pub async fn delegation(&self, id: &str) -> Result<Delegation, Error> {
        self.get(&format!("/delegation/{id}"), &[]).await
    }
}
