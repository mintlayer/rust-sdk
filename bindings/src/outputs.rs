// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Transaction output constructors. Each function returns the runtime blob
//! of one `TxOutput`; concatenate the blobs for
//! [`encodeTransaction`](crate::encode_transaction).

use mintlayer_sdk::crypto::types::{DecodeAll, Encode, OutputTimeLock, PublicKey, StakePoolData};

use super::error::{BindingsError, decoding};
use super::network::Network;
use super::types::{FreezableToken, TotalSupply, amount_from_atoms};
use mintlayer_sdk::crypto::{self, TokenTotalSupply};

/// Builds a coin transfer output.
#[uniffi::export]
pub fn encode_output_transfer(
    amount: String,
    address: String,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(
        crypto::encode_output_transfer(amount_from_atoms(&amount)?, &address, network.into())?
            .encode(),
    )
}

/// Builds a token transfer output.
#[uniffi::export]
pub fn encode_output_token_transfer(
    amount: String,
    address: String,
    token_id: String,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_output_token_transfer(
        amount_from_atoms(&amount)?,
        &address,
        &token_id,
        network.into(),
    )?
    .encode())
}

/// Builds a time-locked coin transfer output. `lock` is a blob produced by
/// one of the `encodeLock*` functions.
#[uniffi::export]
pub fn encode_output_lock_then_transfer(
    amount: String,
    address: String,
    lock: Vec<u8>,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let lock = OutputTimeLock::decode_all(&mut &lock[..]).map_err(decoding)?;
    Ok(crypto::encode_output_lock_then_transfer(
        amount_from_atoms(&amount)?,
        &address,
        lock,
        network.into(),
    )?
    .encode())
}

/// Builds a time-locked token transfer output.
#[uniffi::export]
pub fn encode_output_token_lock_then_transfer(
    amount: String,
    address: String,
    token_id: String,
    lock: Vec<u8>,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let lock = OutputTimeLock::decode_all(&mut &lock[..]).map_err(decoding)?;
    Ok(crypto::encode_output_token_lock_then_transfer(
        amount_from_atoms(&amount)?,
        &address,
        &token_id,
        lock,
        network.into(),
    )?
    .encode())
}

/// Builds a coin burn output.
#[uniffi::export]
pub fn encode_output_coin_burn(amount: String) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_output_coin_burn(amount_from_atoms(&amount)?).encode())
}

/// Builds a token burn output.
#[uniffi::export]
pub fn encode_output_token_burn(
    amount: String,
    token_id: String,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(
        crypto::encode_output_token_burn(amount_from_atoms(&amount)?, &token_id, network.into())?
            .encode(),
    )
}

/// Builds an output that creates a delegation to a pool.
#[uniffi::export]
pub fn encode_output_create_delegation(
    pool_id: String,
    owner_address: String,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_output_create_delegation(&pool_id, &owner_address, network.into())?.encode())
}

/// Builds a delegation-staking output.
#[uniffi::export]
pub fn encode_output_delegate_staking(
    amount: String,
    delegation_id: String,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_output_delegate_staking(
        amount_from_atoms(&amount)?,
        &delegation_id,
        network.into(),
    )?
    .encode())
}

/// Builds a stake pool creation output. `pool_data` is the blob produced by
/// [`encodeStakePoolData`](crate::encode_stake_pool_data).
#[uniffi::export]
pub fn encode_output_create_stake_pool(
    pool_id: String,
    pool_data: Vec<u8>,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let pool_data = StakePoolData::decode_all(&mut &pool_data[..]).map_err(decoding)?;
    Ok(crypto::encode_output_create_stake_pool(&pool_id, pool_data, network.into())?.encode())
}

/// Builds a `ProduceBlockFromStake` output.
#[uniffi::export]
pub fn encode_output_produce_block_from_stake(
    pool_id: String,
    staker: String,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_output_produce_block_from_stake(&pool_id, &staker, network.into())?.encode())
}

/// Builds a data deposit output.
#[uniffi::export]
pub fn encode_output_data_deposit(data: Vec<u8>) -> Vec<u8> {
    crypto::encode_output_data_deposit(&data).encode()
}

/// Builds a fungible token issuance output.
///
/// `totalSupply` `Fixed` requires `supplyAmount`; `Lockable` and
/// `Unlimited` ignore it.
#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub fn encode_output_issue_fungible_token(
    authority: String,
    token_ticker: String,
    metadata_uri: String,
    number_of_decimals: u8,
    total_supply: TotalSupply,
    supply_amount: Option<String>,
    is_token_freezable: FreezableToken,
    current_block_height: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let _ = current_block_height; // accepted for wasm-parity; unused upstream
    let total_supply = match total_supply {
        TotalSupply::Lockable => TokenTotalSupply::Lockable,
        TotalSupply::Unlimited => TokenTotalSupply::Unlimited,
        TotalSupply::Fixed => TokenTotalSupply::Fixed(
            supply_amount
                .ok_or(BindingsError::FixedTotalSupplyWithoutAmount)
                .and_then(|atoms| amount_from_atoms(&atoms))?,
        ),
    };
    Ok(crypto::encode_output_issue_fungible_token(
        &authority,
        &token_ticker,
        &metadata_uri,
        number_of_decimals,
        total_supply,
        is_token_freezable.into(),
        network.into(),
    )?
    .encode())
}

/// Builds an NFT issuance output.
#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub fn encode_output_issue_nft(
    token_id: String,
    authority: String,
    name: String,
    ticker: String,
    description: String,
    media_hash: Vec<u8>,
    creator: Option<Vec<u8>>,
    media_uri: Option<String>,
    icon_uri: Option<String>,
    additional_metadata_uri: Option<String>,
    current_block_height: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let _ = current_block_height; // accepted for wasm-parity; unused upstream
    let creator = creator
        .map(|blob| PublicKey::decode_all(&mut &blob[..]).map_err(decoding))
        .transpose()?;
    Ok(crypto::encode_output_issue_nft(
        &token_id,
        &authority,
        &name,
        &ticker,
        &description,
        &media_hash,
        creator,
        media_uri.as_deref(),
        icon_uri.as_deref(),
        additional_metadata_uri.as_deref(),
        network.into(),
    )?
    .encode())
}

/// Builds a hash-timelock-contract output. `refundTimelock` is a blob
/// produced by one of the `encodeLock*` functions.
#[uniffi::export]
pub fn encode_output_htlc(
    amount: String,
    token_id: Option<String>,
    secret_hash: String,
    spend_address: String,
    refund_address: String,
    refund_timelock: Vec<u8>,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let refund_timelock =
        OutputTimeLock::decode_all(&mut &refund_timelock[..]).map_err(decoding)?;
    Ok(crypto::encode_output_htlc(
        amount_from_atoms(&amount)?,
        token_id.as_deref(),
        &secret_hash,
        &spend_address,
        &refund_address,
        refund_timelock,
        network.into(),
    )?
    .encode())
}

/// Builds an order creation output.
#[uniffi::export]
pub fn encode_create_order_output(
    ask_amount: String,
    ask_token_id: Option<String>,
    give_amount: String,
    give_token_id: Option<String>,
    conclude_address: String,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_create_order_output(
        amount_from_atoms(&ask_amount)?,
        ask_token_id.as_deref(),
        amount_from_atoms(&give_amount)?,
        give_token_id.as_deref(),
        &conclude_address,
        network.into(),
    )?
    .encode())
}
