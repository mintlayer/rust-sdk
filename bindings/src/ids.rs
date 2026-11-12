// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Derivation of token, pool, delegation and order ids from the inputs of
//! the transaction that creates them. Ids are returned as bech32 address
//! strings.

use mintlayer_sdk::crypto::types::{DecodeAll, TxInput};

use super::error::{BindingsError, decoding};
use super::network::Network;
use mintlayer_sdk::crypto;

/// Decodes a list of runtime input blobs into typed inputs.
pub(crate) fn decode_inputs(blobs: Vec<Vec<u8>>) -> Result<Vec<TxInput>, BindingsError> {
    blobs
        .into_iter()
        .map(|blob| TxInput::decode_all(&mut &blob[..]).map_err(decoding))
        .collect()
}

/// Returns the bech32 token id produced by issuing a token with these
/// inputs. The id scheme depends on the network upgrade active at
/// `current_block_height`.
#[uniffi::export]
pub fn get_token_id(
    inputs: Vec<Vec<u8>>,
    current_block_height: u64,
    network: Network,
) -> Result<String, BindingsError> {
    let inputs = decode_inputs(inputs)?;
    Ok(crypto::get_token_id(
        &inputs,
        current_block_height,
        network.into(),
    )?)
}

/// Returns the bech32 order id produced by creating an order with these
/// inputs.
#[uniffi::export]
pub fn get_order_id(inputs: Vec<Vec<u8>>, network: Network) -> Result<String, BindingsError> {
    let inputs = decode_inputs(inputs)?;
    Ok(crypto::get_order_id(&inputs, network.into())?)
}

/// Returns the bech32 delegation id produced by creating a delegation with
/// these inputs.
#[uniffi::export]
pub fn get_delegation_id(inputs: Vec<Vec<u8>>, network: Network) -> Result<String, BindingsError> {
    let inputs = decode_inputs(inputs)?;
    Ok(crypto::get_delegation_id(&inputs, network.into())?)
}

/// Returns the bech32 pool id produced by creating a stake pool with these
/// inputs.
#[uniffi::export]
pub fn get_pool_id(inputs: Vec<Vec<u8>>, network: Network) -> Result<String, BindingsError> {
    let inputs = decode_inputs(inputs)?;
    Ok(crypto::get_pool_id(&inputs, network.into())?)
}
