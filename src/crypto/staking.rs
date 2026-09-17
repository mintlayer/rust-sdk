// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Stake pool helpers.

use ml_common as common;
use ml_consensus as consensus;

use common::chain::stakelock::StakePoolData;
use common::primitives::per_thousand::PerThousand;
use common::primitives::{Amount, BlockHeight};

use super::{Error, Network, chain_config, parse_addressable};

/// Builds the pool data object used by
/// [`encode_output_create_stake_pool`](super::encode_output_create_stake_pool).
pub fn encode_stake_pool_data(
    value: Amount,
    staker: &str,
    vrf_public_key: &str,
    decommission_key: &str,
    margin_ratio_per_thousand: u16,
    cost_per_block: Amount,
    network: Network,
) -> Result<StakePoolData, Error> {
    let staker = parse_addressable(network, staker)?;
    let vrf_public_key = parse_addressable(network, vrf_public_key)?;
    let decommission_key = parse_addressable(network, decommission_key)?;
    let margin_ratio = PerThousand::new(margin_ratio_per_thousand)
        .ok_or(Error::InvalidMarginRatio(margin_ratio_per_thousand))?;

    Ok(StakePoolData::new(
        value,
        staker,
        vrf_public_key,
        decommission_key,
        margin_ratio,
        cost_per_block,
    ))
}

/// Calculates the pledge-capped effective balance of a pool.
pub fn effective_pool_balance(
    network: Network,
    pledge_amount: Amount,
    pool_balance: Amount,
) -> Result<Amount, Error> {
    let final_supply = chain_config(network)
        .final_supply()
        .ok_or(Error::EffectiveBalance("final supply missing".to_owned()))?
        .to_amount_atoms();

    consensus::calculate_effective_pool_balance(pledge_amount, pool_balance, final_supply)
        .map_err(|error| Error::EffectiveBalance(error.to_string()))
}

/// Returns the number of blocks until a decommissioned pool's funds become
/// spendable.
pub fn staking_pool_spend_maturity_block_count(current_block_height: u64, network: Network) -> u64 {
    chain_config(network)
        .staking_pool_spend_maturity_block_count(BlockHeight::new(current_block_height))
        .to_int()
}
