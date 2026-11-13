// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Stake pool helpers: pool data construction, pledge-capped effective
//! balance and spend maturity.

use mintlayer_sdk::crypto::types::Encode;

use super::error::BindingsError;
use super::network::Network;
use super::types::{amount_from_atoms, amount_to_atoms};
use mintlayer_sdk::crypto;

/// Builds the pool data object used by
/// [`encodeOutputCreateStakePool`](crate::encode_output_create_stake_pool).
/// The result is a runtime blob to be passed there as `poolData`.
#[uniffi::export]
pub fn encode_stake_pool_data(
    value: String,
    staker: String,
    vrf_public_key: String,
    decommission_key: String,
    margin_ratio_per_thousand: u16,
    cost_per_block: String,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let data = crypto::encode_stake_pool_data(
        amount_from_atoms(&value)?,
        &staker,
        &vrf_public_key,
        &decommission_key,
        margin_ratio_per_thousand,
        amount_from_atoms(&cost_per_block)?,
        network.into(),
    )?;
    Ok(data.encode())
}

/// Calculates the pledge-capped effective balance of a pool, in atoms.
#[uniffi::export]
pub fn effective_pool_balance(
    network: Network,
    pledge_amount: String,
    pool_balance: String,
) -> Result<String, BindingsError> {
    let balance = crypto::effective_pool_balance(
        network.into(),
        amount_from_atoms(&pledge_amount)?,
        amount_from_atoms(&pool_balance)?,
    )?;
    Ok(amount_to_atoms(balance))
}

/// Returns the number of blocks until a decommissioned pool's funds become
/// spendable.
#[uniffi::export]
pub fn staking_pool_spend_maturity_block_count(current_block_height: u64, network: Network) -> u64 {
    crypto::staking_pool_spend_maturity_block_count(current_block_height, network.into())
}
