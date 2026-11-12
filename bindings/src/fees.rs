// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Static and height-dependent protocol fees. Amounts are returned as
//! decimal atom strings.

use super::network::Network;
use super::types::amount_to_atoms;
use mintlayer_sdk::crypto;

/// The fee for issuing a new fungible token.
#[uniffi::export]
pub fn fungible_token_issuance_fee(current_block_height: u64, network: Network) -> String {
    amount_to_atoms(crypto::fungible_token_issuance_fee(
        current_block_height,
        network.into(),
    ))
}

/// The fee for issuing a new NFT.
#[uniffi::export]
pub fn nft_issuance_fee(current_block_height: u64, network: Network) -> String {
    amount_to_atoms(crypto::nft_issuance_fee(
        current_block_height,
        network.into(),
    ))
}

/// The fee for minting or unminting tokens.
#[uniffi::export]
pub fn token_supply_change_fee(current_block_height: u64, network: Network) -> String {
    amount_to_atoms(crypto::token_supply_change_fee(
        current_block_height,
        network.into(),
    ))
}

/// The fee for freezing or unfreezing a token.
#[uniffi::export]
pub fn token_freeze_fee(current_block_height: u64, network: Network) -> String {
    amount_to_atoms(crypto::token_freeze_fee(
        current_block_height,
        network.into(),
    ))
}

/// The fee for changing the authority of a token.
#[uniffi::export]
pub fn token_change_authority_fee(current_block_height: u64, network: Network) -> String {
    amount_to_atoms(crypto::token_change_authority_fee(
        current_block_height,
        network.into(),
    ))
}

/// The fee for a data deposit output.
#[uniffi::export]
pub fn data_deposit_fee(current_block_height: u64, network: Network) -> String {
    amount_to_atoms(crypto::data_deposit_fee(
        current_block_height,
        network.into(),
    ))
}
