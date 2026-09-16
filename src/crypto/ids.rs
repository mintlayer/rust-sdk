// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Derivation of token, pool, delegation and order ids from transaction
//! inputs.

use ml_common as common;

use common::address::Address;
use common::chain::{TxInput, make_delegation_id, make_order_id, make_pool_id, make_token_id};
use common::primitives::BlockHeight;

use super::{Error, Network, chain_config};

/// Returns the bech32 token id produced by issuing a token with these inputs.
///
/// The id scheme depends on the network upgrade active at
/// `current_block_height`.
pub fn get_token_id(
    inputs: &[TxInput],
    current_block_height: u64,
    network: Network,
) -> Result<String, Error> {
    let token_id = make_token_id(
        chain_config(network),
        BlockHeight::new(current_block_height),
        inputs,
    )
    .map_err(|error| Error::TransactionCreation(error.to_string()))?;
    Ok(token_address(token_id, network))
}

/// Returns the bech32 order id produced by creating an order with these
/// inputs.
pub fn get_order_id(inputs: &[TxInput], network: Network) -> Result<String, Error> {
    let order_id =
        make_order_id(inputs).map_err(|error| Error::TransactionCreation(error.to_string()))?;
    Ok(token_address(order_id, network))
}

/// Returns the bech32 delegation id produced by creating a delegation with
/// these inputs.
pub fn get_delegation_id(inputs: &[TxInput], network: Network) -> Result<String, Error> {
    let delegation_id = make_delegation_id(inputs)
        .map_err(|error| Error::TransactionCreation(error.to_string()))?;
    Ok(token_address(delegation_id, network))
}

/// Returns the bech32 pool id produced by creating a stake pool with these
/// inputs.
pub fn get_pool_id(inputs: &[TxInput], network: Network) -> Result<String, Error> {
    let pool_id =
        make_pool_id(inputs).map_err(|error| Error::TransactionCreation(error.to_string()))?;
    Ok(token_address(pool_id, network))
}

fn token_address<T: common::address::traits::Addressable>(object: T, network: Network) -> String {
    Address::new(chain_config(network), object)
        .expect("id address creation must not fail")
        .to_string()
}
