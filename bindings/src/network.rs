// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! The network selector shared by all network-parameterized operations.

use mintlayer_sdk::crypto;

/// The network for which an operation is performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Network {
    /// Production network.
    Mainnet,
    /// Public test network.
    Testnet,
    /// Local regression-testing network.
    Regtest,
    /// Signet.
    Signet,
}

impl From<Network> for crypto::Network {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => Self::Mainnet,
            Network::Testnet => Self::Testnet,
            Network::Regtest => Self::Regtest,
            Network::Signet => Self::Signet,
        }
    }
}
