// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Transaction output constructors.

use ml_common as common;
use ml_crypto as crypto;

use common::chain::{
    OrderData, TxOutput,
    htlc::{HashedTimelockContract, HtlcSecretHash},
    output_value::OutputValue,
    stakelock::StakePoolData,
    timelock::OutputTimeLock,
    tokens::{
        IsTokenFreezable, Metadata, NftIssuance, NftIssuanceV0, TokenCreator, TokenIssuance,
        TokenIssuanceV1, TokenTotalSupply,
    },
};
use common::primitives::Amount;
use crypto::key::PublicKey;
use std::str::FromStr;

use super::{Error, Network, parse_addressable};

/// Builds a coin transfer output.
pub fn encode_output_transfer(
    amount: Amount,
    address: &str,
    network: Network,
) -> Result<TxOutput, Error> {
    let destination = parse_addressable(network, address)?;
    Ok(TxOutput::Transfer(OutputValue::Coin(amount), destination))
}

/// Builds a token transfer output.
pub fn encode_output_token_transfer(
    amount: Amount,
    address: &str,
    token_id: &str,
    network: Network,
) -> Result<TxOutput, Error> {
    let destination = parse_addressable(network, address)?;
    let token_id = parse_addressable(network, token_id)?;
    Ok(TxOutput::Transfer(
        OutputValue::TokenV1(token_id, amount),
        destination,
    ))
}

/// Builds a timelocked coin transfer output.
pub fn encode_output_lock_then_transfer(
    amount: Amount,
    address: &str,
    lock: OutputTimeLock,
    network: Network,
) -> Result<TxOutput, Error> {
    let destination = parse_addressable(network, address)?;
    Ok(TxOutput::LockThenTransfer(
        OutputValue::Coin(amount),
        destination,
        lock,
    ))
}

/// Builds a timelocked token transfer output.
pub fn encode_output_token_lock_then_transfer(
    amount: Amount,
    address: &str,
    token_id: &str,
    lock: OutputTimeLock,
    network: Network,
) -> Result<TxOutput, Error> {
    let destination = parse_addressable(network, address)?;
    let token_id = parse_addressable(network, token_id)?;
    Ok(TxOutput::LockThenTransfer(
        OutputValue::TokenV1(token_id, amount),
        destination,
        lock,
    ))
}

/// Builds a coin burn output.
pub fn encode_output_coin_burn(amount: Amount) -> TxOutput {
    TxOutput::Burn(OutputValue::Coin(amount))
}

/// Builds a token burn output.
pub fn encode_output_token_burn(
    amount: Amount,
    token_id: &str,
    network: Network,
) -> Result<TxOutput, Error> {
    let token_id = parse_addressable(network, token_id)?;
    Ok(TxOutput::Burn(OutputValue::TokenV1(token_id, amount)))
}

/// Builds an output that creates a delegation to a pool.
pub fn encode_output_create_delegation(
    pool_id: &str,
    owner_address: &str,
    network: Network,
) -> Result<TxOutput, Error> {
    let owner = parse_addressable(network, owner_address)?;
    let pool_id = parse_addressable(network, pool_id)?;
    Ok(TxOutput::CreateDelegationId(owner, pool_id))
}

/// Builds an output that delegates coins to a delegation.
pub fn encode_output_delegate_staking(
    amount: Amount,
    delegation_id: &str,
    network: Network,
) -> Result<TxOutput, Error> {
    let delegation_id = parse_addressable(network, delegation_id)?;
    Ok(TxOutput::DelegateStaking(amount, delegation_id))
}

/// Builds an output that creates a stake pool.
///
/// The pool id must be derived from the hash of the first transaction input
/// (see [`super::get_pool_id`]).
pub fn encode_output_create_stake_pool(
    pool_id: &str,
    pool_data: StakePoolData,
    network: Network,
) -> Result<TxOutput, Error> {
    let pool_id = parse_addressable(network, pool_id)?;
    Ok(TxOutput::CreateStakePool(pool_id, Box::new(pool_data)))
}

/// Builds the output emitted when producing a block via a pool.
pub fn encode_output_produce_block_from_stake(
    pool_id: &str,
    staker: &str,
    network: Network,
) -> Result<TxOutput, Error> {
    let staker = parse_addressable(network, staker)?;
    let pool_id = parse_addressable(network, pool_id)?;
    Ok(TxOutput::ProduceBlockFromStake(staker, pool_id))
}

/// Builds an arbitrary data deposit output.
pub fn encode_output_data_deposit(data: &[u8]) -> TxOutput {
    TxOutput::DataDeposit(data.to_vec())
}

/// Builds a fungible token issuance output.
pub fn encode_output_issue_fungible_token(
    authority: &str,
    token_ticker: &str,
    metadata_uri: &str,
    number_of_decimals: u8,
    total_supply: TokenTotalSupply,
    is_token_freezable: IsTokenFreezable,
    network: Network,
) -> Result<TxOutput, Error> {
    let authority = parse_addressable(network, authority)?;
    let token_issuance = TokenIssuance::V1(TokenIssuanceV1 {
        authority,
        token_ticker: token_ticker.as_bytes().to_vec(),
        metadata_uri: metadata_uri.as_bytes().to_vec(),
        number_of_decimals,
        total_supply,
        is_freezable: is_token_freezable,
    });

    ml_tx_verifier::check_tokens_issuance(super::chain_config(network), &token_issuance)
        .map_err(|error| Error::InvalidTokenParameters(error.to_string()))?;

    Ok(TxOutput::IssueFungibleToken(Box::new(token_issuance)))
}

/// Builds an NFT issuance output.
#[allow(clippy::too_many_arguments)]
pub fn encode_output_issue_nft(
    token_id: &str,
    authority: &str,
    name: &str,
    ticker: &str,
    description: &str,
    media_hash: &[u8],
    creator: Option<PublicKey>,
    media_uri: Option<&str>,
    icon_uri: Option<&str>,
    additional_metadata_uri: Option<&str>,
    network: Network,
) -> Result<TxOutput, Error> {
    let token_id = parse_addressable(network, token_id)?;
    let authority = parse_addressable(network, authority)?;
    let creator = creator.map(|public_key| TokenCreator { public_key });

    let nft_issuance = NftIssuanceV0 {
        metadata: Metadata {
            creator,
            name: name.as_bytes().to_vec(),
            description: description.as_bytes().to_vec(),
            ticker: ticker.as_bytes().to_vec(),
            icon_uri: into_data_or_no_vec(icon_uri),
            additional_metadata_uri: into_data_or_no_vec(additional_metadata_uri),
            media_uri: into_data_or_no_vec(media_uri),
            media_hash: media_hash.to_vec(),
        },
    };

    ml_tx_verifier::check_nft_issuance_data(super::chain_config(network), &nft_issuance)
        .map_err(|error| Error::InvalidTokenParameters(error.to_string()))?;

    Ok(TxOutput::IssueNft(
        token_id,
        Box::new(NftIssuance::V0(nft_issuance)),
        authority,
    ))
}

/// Builds an HTLC output; `token_id` selects a token HTLC instead of coins.
pub fn encode_output_htlc(
    amount: Amount,
    token_id: Option<&str>,
    secret_hash: &str,
    spend_address: &str,
    refund_address: &str,
    refund_timelock: OutputTimeLock,
    network: Network,
) -> Result<TxOutput, Error> {
    let output_value = output_value(network, amount, token_id)?;
    let secret_hash = HtlcSecretHash::from_str(secret_hash)
        .map_err(|error| Error::InvalidHtlcSecretHash(error.to_string()))?;
    let spend_key = parse_addressable(network, spend_address)?;
    let refund_key = parse_addressable(network, refund_address)?;

    let htlc = HashedTimelockContract {
        secret_hash,
        spend_key,
        refund_timelock,
        refund_key,
    };
    Ok(TxOutput::Htlc(output_value, Box::new(htlc)))
}

/// Builds a DEX order creation output; `None` token ids mean coins.
pub fn encode_create_order_output(
    ask_amount: Amount,
    ask_token_id: Option<&str>,
    give_amount: Amount,
    give_token_id: Option<&str>,
    conclude_address: &str,
    network: Network,
) -> Result<TxOutput, Error> {
    let ask = output_value(network, ask_amount, ask_token_id)?;
    let give = output_value(network, give_amount, give_token_id)?;
    let conclude_key = parse_addressable(network, conclude_address)?;
    Ok(TxOutput::CreateOrder(Box::new(OrderData::new(
        conclude_key,
        ask,
        give,
    ))))
}

fn output_value(
    network: Network,
    amount: Amount,
    token_id: Option<&str>,
) -> Result<OutputValue, Error> {
    match token_id {
        Some(token_id) => Ok(OutputValue::TokenV1(
            parse_addressable(network, token_id)?,
            amount,
        )),
        None => Ok(OutputValue::Coin(amount)),
    }
}

fn into_data_or_no_vec(
    value: Option<&str>,
) -> ml_serialization::extras::non_empty_vec::DataOrNoVec<u8> {
    value.map(|value| value.as_bytes().to_vec()).into()
}
