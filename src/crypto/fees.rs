// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Static and height-dependent protocol fees.

use ml_common as common;

use common::primitives::{Amount, BlockHeight};

use super::Network;

/// The fee for issuing a new fungible token.
pub fn fungible_token_issuance_fee(_current_block_height: u64, network: Network) -> Amount {
    super::chain_config(network).fungible_token_issuance_fee()
}

/// The fee for issuing a new NFT.
pub fn nft_issuance_fee(current_block_height: u64, network: Network) -> Amount {
    super::chain_config(network).nft_issuance_fee(BlockHeight::new(current_block_height))
}

/// The fee for minting or unminting tokens.
pub fn token_supply_change_fee(current_block_height: u64, network: Network) -> Amount {
    super::chain_config(network).token_supply_change_fee(BlockHeight::new(current_block_height))
}

/// The fee for freezing or unfreezing a token.
pub fn token_freeze_fee(current_block_height: u64, network: Network) -> Amount {
    super::chain_config(network).token_freeze_fee(BlockHeight::new(current_block_height))
}

/// The fee for changing the authority of a token.
pub fn token_change_authority_fee(current_block_height: u64, network: Network) -> Amount {
    super::chain_config(network).token_change_authority_fee(BlockHeight::new(current_block_height))
}

/// The fee for a data deposit output.
pub fn data_deposit_fee(current_block_height: u64, network: Network) -> Amount {
    super::chain_config(network).data_deposit_fee(BlockHeight::new(current_block_height))
}
