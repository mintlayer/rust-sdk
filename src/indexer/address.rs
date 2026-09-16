// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Indexer address endpoints.

use super::{AddressInfo, DelegationInfo, Error, Utxo};
use crate::indexer::Client;

impl Client {
    /// Returns aggregated information about an address
    /// (`GET /address/{address}`).
    pub async fn address_info(&self, address: &str) -> Result<AddressInfo, Error> {
        self.get(&format!("/address/{address}"), &[]).await
    }

    /// Returns the spendable UTXOs of an address
    /// (`GET /address/{address}/spendable-utxos`).
    pub async fn spendable_utxos(&self, address: &str) -> Result<Vec<Utxo>, Error> {
        self.get(&format!("/address/{address}/spendable-utxos"), &[]).await
    }

    /// Returns all UTXOs of an address, including locked ones
    /// (`GET /address/{address}/all-utxos`).
    pub async fn all_utxos(&self, address: &str) -> Result<Vec<Utxo>, Error> {
        self.get(&format!("/address/{address}/all-utxos"), &[]).await
    }

    /// Returns the delegations owned by an address
    /// (`GET /address/{address}/delegations`).
    pub async fn delegations(&self, address: &str) -> Result<Vec<DelegationInfo>, Error> {
        self.get(&format!("/address/{address}/delegations"), &[]).await
    }

    /// Returns the token ids for which an address holds authority
    /// (`GET /address/{address}/token-authority`).
    pub async fn token_authority(&self, address: &str) -> Result<Vec<String>, Error> {
        self.get(&format!("/address/{address}/token-authority"), &[]).await
    }
}
