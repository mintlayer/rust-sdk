// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Transaction input constructors. Each function returns the runtime blob
//! of one `TxInput`; concatenate the blobs for
//! [`encodeTransaction`](crate::encode_transaction).

use mintlayer_sdk::crypto::types::{DecodeAll, Encode, OutPointSourceId};

use super::error::{BindingsError, decoding};
use super::network::Network;
use super::types::{TokenUnfreezable, amount_from_atoms};
use mintlayer_sdk::crypto;

/// Builds a UTXO input. `outpointSourceId` is the blob produced by
/// [`encodeOutpointSourceId`](crate::encode_outpoint_source_id).
#[uniffi::export]
pub fn encode_input_for_utxo(
    outpoint_source_id: Vec<u8>,
    output_index: u32,
) -> Result<Vec<u8>, BindingsError> {
    let outpoint_source_id =
        OutPointSourceId::decode_all(&mut &outpoint_source_id[..]).map_err(decoding)?;
    Ok(crypto::encode_input_for_utxo(outpoint_source_id, output_index).encode())
}

/// Builds an input that withdraws from a delegation. The nonce must be in
/// sequence with the other account spendings of the delegation owner.
#[uniffi::export]
pub fn encode_input_for_withdraw_from_delegation(
    delegation_id: String,
    amount: String,
    nonce: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_input_for_withdraw_from_delegation(
        &delegation_id,
        amount_from_atoms(&amount)?,
        nonce,
        network.into(),
    )?
    .encode())
}

/// Builds an input that mints tokens (authority only).
#[uniffi::export]
pub fn encode_input_for_mint_tokens(
    token_id: String,
    amount: String,
    nonce: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_input_for_mint_tokens(
        &token_id,
        amount_from_atoms(&amount)?,
        nonce,
        network.into(),
    )?
    .encode())
}

/// Builds an input that unmints tokens (authority only).
#[uniffi::export]
pub fn encode_input_for_unmint_tokens(
    token_id: String,
    nonce: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_input_for_unmint_tokens(&token_id, nonce, network.into())?.encode())
}

/// Builds an input that locks a token's total supply (authority only).
#[uniffi::export]
pub fn encode_input_for_lock_token_supply(
    token_id: String,
    nonce: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_input_for_lock_token_supply(&token_id, nonce, network.into())?.encode())
}

/// Builds an input that freezes a token (authority only).
#[uniffi::export]
pub fn encode_input_for_freeze_token(
    token_id: String,
    is_token_unfreezable: TokenUnfreezable,
    nonce: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_input_for_freeze_token(
        &token_id,
        is_token_unfreezable.into(),
        nonce,
        network.into(),
    )?
    .encode())
}

/// Builds an input that unfreezes a token (authority only).
#[uniffi::export]
pub fn encode_input_for_unfreeze_token(
    token_id: String,
    nonce: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_input_for_unfreeze_token(&token_id, nonce, network.into())?.encode())
}

/// Builds an input that changes a token's authority (authority only).
#[uniffi::export]
pub fn encode_input_for_change_token_authority(
    token_id: String,
    new_authority: String,
    nonce: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_input_for_change_token_authority(
        &token_id,
        &new_authority,
        nonce,
        network.into(),
    )?
    .encode())
}

/// Builds an input that changes a token's metadata URI (authority only).
#[uniffi::export]
pub fn encode_input_for_change_token_metadata_uri(
    token_id: String,
    new_metadata_uri: String,
    nonce: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_input_for_change_token_metadata_uri(
        &token_id,
        &new_metadata_uri,
        nonce,
        network.into(),
    )?
    .encode())
}

/// Builds an order-fill input. The nonce must be in sequence with the
/// other account spendings of the filler.
#[uniffi::export]
pub fn encode_input_for_fill_order(
    order_id: String,
    fill_amount: String,
    destination: String,
    nonce: u64,
    current_block_height: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_input_for_fill_order(
        &order_id,
        amount_from_atoms(&fill_amount)?,
        &destination,
        nonce,
        current_block_height,
        network.into(),
    )?
    .encode())
}

/// Builds an order-freeze input (order authority only).
#[uniffi::export]
pub fn encode_input_for_freeze_order(
    order_id: String,
    current_block_height: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(
        crypto::encode_input_for_freeze_order(&order_id, current_block_height, network.into())?
            .encode(),
    )
}

/// Builds an order-conclude input (order creator only).
#[uniffi::export]
pub fn encode_input_for_conclude_order(
    order_id: String,
    nonce: u64,
    current_block_height: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_input_for_conclude_order(
        &order_id,
        nonce,
        current_block_height,
        network.into(),
    )?
    .encode())
}
