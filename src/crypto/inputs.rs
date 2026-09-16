// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Transaction input constructors.

use ml_common as common;

use common::chain::{
    AccountCommand, AccountNonce, AccountOutPoint, AccountSpending, OrderAccountCommand,
    OrdersVersion, OutPointSourceId, TxInput, UtxoOutPoint,
};
use common::primitives::{Amount, BlockHeight};

use super::{Error, IsTokenUnfreezable, Network, parse_addressable};

/// Builds a UTXO spend input.
#[must_use]
pub fn encode_input_for_utxo(outpoint_source_id: OutPointSourceId, output_index: u32) -> TxInput {
    TxInput::Utxo(UtxoOutPoint::new(outpoint_source_id, output_index))
}

/// Builds an input that withdraws from a delegation.
///
/// The nonce must be in sequence with the other account spendings of the
/// delegation owner.
pub fn encode_input_for_withdraw_from_delegation(
    delegation_id: &str,
    amount: Amount,
    nonce: u64,
    network: Network,
) -> Result<TxInput, Error> {
    let delegation_id = parse_addressable(network, delegation_id)?;
    Ok(TxInput::Account(AccountOutPoint::new(
        AccountNonce::new(nonce),
        AccountSpending::DelegationBalance(delegation_id, amount),
    )))
}

/// Builds a mint-tokens input.
pub fn encode_input_for_mint_tokens(
    token_id: &str,
    amount: Amount,
    nonce: u64,
    network: Network,
) -> Result<TxInput, Error> {
    let token_id = parse_addressable(network, token_id)?;
    Ok(TxInput::AccountCommand(
        AccountNonce::new(nonce),
        AccountCommand::MintTokens(token_id, amount),
    ))
}

/// Builds an unmint-tokens input.
pub fn encode_input_for_unmint_tokens(
    token_id: &str,
    nonce: u64,
    network: Network,
) -> Result<TxInput, Error> {
    let token_id = parse_addressable(network, token_id)?;
    Ok(TxInput::AccountCommand(
        AccountNonce::new(nonce),
        AccountCommand::UnmintTokens(token_id),
    ))
}

/// Builds a lock-token-supply input.
pub fn encode_input_for_lock_token_supply(
    token_id: &str,
    nonce: u64,
    network: Network,
) -> Result<TxInput, Error> {
    let token_id = parse_addressable(network, token_id)?;
    Ok(TxInput::AccountCommand(
        AccountNonce::new(nonce),
        AccountCommand::LockTokenSupply(token_id),
    ))
}

/// Builds a freeze-token input.
pub fn encode_input_for_freeze_token(
    token_id: &str,
    is_token_unfreezable: IsTokenUnfreezable,
    nonce: u64,
    network: Network,
) -> Result<TxInput, Error> {
    let token_id = parse_addressable(network, token_id)?;
    Ok(TxInput::AccountCommand(
        AccountNonce::new(nonce),
        AccountCommand::FreezeToken(token_id, is_token_unfreezable),
    ))
}

/// Builds an unfreeze-token input.
pub fn encode_input_for_unfreeze_token(
    token_id: &str,
    nonce: u64,
    network: Network,
) -> Result<TxInput, Error> {
    let token_id = parse_addressable(network, token_id)?;
    Ok(TxInput::AccountCommand(
        AccountNonce::new(nonce),
        AccountCommand::UnfreezeToken(token_id),
    ))
}

/// Builds a change-token-authority input.
pub fn encode_input_for_change_token_authority(
    token_id: &str,
    new_authority: &str,
    nonce: u64,
    network: Network,
) -> Result<TxInput, Error> {
    let token_id = parse_addressable(network, token_id)?;
    let new_authority = parse_addressable(network, new_authority)?;
    Ok(TxInput::AccountCommand(
        AccountNonce::new(nonce),
        AccountCommand::ChangeTokenAuthority(token_id, new_authority),
    ))
}

/// Builds a change-token-metadata-URI input.
pub fn encode_input_for_change_token_metadata_uri(
    token_id: &str,
    new_metadata_uri: &str,
    nonce: u64,
    network: Network,
) -> Result<TxInput, Error> {
    let token_id = parse_addressable(network, token_id)?;
    Ok(TxInput::AccountCommand(
        AccountNonce::new(nonce),
        AccountCommand::ChangeTokenMetadataUri(token_id, new_metadata_uri.as_bytes().to_vec()),
    ))
}

/// Builds a fill-order input.
///
/// Fill-order inputs must not be signed; use
/// [`encode_witness_no_signature`](super::encode_witness_no_signature) for
/// them. Before the orders V1 fork the nonce is significant; once the fork
/// is active at `current_block_height`, both the `nonce` and the
/// `destination` are ignored (in V1 the destination is derived from the
/// order's transaction outputs).
pub fn encode_input_for_fill_order(
    order_id: &str,
    fill_amount: Amount,
    destination: &str,
    nonce: u64,
    current_block_height: u64,
    network: Network,
) -> Result<TxInput, Error> {
    let order_id = parse_addressable(network, order_id)?;
    let destination = parse_addressable(network, destination)?;
    match orders_version(network, current_block_height)? {
        OrdersVersion::V0 => Ok(TxInput::AccountCommand(
            AccountNonce::new(nonce),
            AccountCommand::FillOrder(order_id, fill_amount, destination),
        )),
        OrdersVersion::V1 => Ok(TxInput::OrderAccountCommand(
            OrderAccountCommand::FillOrder(order_id, fill_amount),
        )),
    }
}

/// Builds a freeze-order input (orders V1 only).
pub fn encode_input_for_freeze_order(
    order_id: &str,
    current_block_height: u64,
    network: Network,
) -> Result<TxInput, Error> {
    let order_id = parse_addressable(network, order_id)?;
    match orders_version(network, current_block_height)? {
        OrdersVersion::V0 => Err(Error::OrdersV1NotActivated),
        OrdersVersion::V1 => Ok(TxInput::OrderAccountCommand(
            OrderAccountCommand::FreezeOrder(order_id),
        )),
    }
}

/// Builds a conclude-order input.
///
/// Before the orders V1 fork the nonce is significant; afterwards it is
/// ignored.
pub fn encode_input_for_conclude_order(
    order_id: &str,
    nonce: u64,
    current_block_height: u64,
    network: Network,
) -> Result<TxInput, Error> {
    let order_id = parse_addressable(network, order_id)?;
    match orders_version(network, current_block_height)? {
        OrdersVersion::V0 => Ok(TxInput::AccountCommand(
            AccountNonce::new(nonce),
            AccountCommand::ConcludeOrder(order_id),
        )),
        OrdersVersion::V1 => Ok(TxInput::OrderAccountCommand(
            OrderAccountCommand::ConcludeOrder(order_id),
        )),
    }
}

fn orders_version(network: Network, current_block_height: u64) -> Result<OrdersVersion, Error> {
    Ok(super::chain_config(network)
        .chainstate_upgrades()
        .version_at_height(BlockHeight::new(current_block_height))
        .1
        .orders_version())
}
