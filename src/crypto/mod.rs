// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Native cryptography and transaction building backed by
//! [mintlayer-core](https://github.com/mintlayer/mintlayer-core).
//!
//! The functions mirror the go-sdk `wasm` sub-client operation for operation,
//! but exchange typed values ([`Transaction`], [`TxOutput`], [`PrivateKey`],
//! [`Amount`], ...) instead of opaque byte arrays. Every type participates in
//! the SCALE encoding, so [`Encode`] and [`DecodeAll`] give byte-level access
//! when needed.
//!
//! # Fork sensitivity
//!
//! Some functions take a `current_block_height` parameter because their
//! result depends on whether a hard fork has been activated. The value should
//! be the height of the block into which the transaction is supposed to be
//! included; a wallet that cannot predict it should use "the current tip
//! height plus one" and refuse to operate near a fork height.

mod addresses;
mod error;
mod fees;
mod ids;
mod inputs;
mod intents;
mod keys;
mod outputs;
mod signing;
mod staking;
mod timelocks;
mod transactions;

use std::sync::OnceLock;

use ml_common as common;

pub use addresses::{
    encode_destination, encode_multisig_challenge, multisig_challenge_to_address,
    pubkey_to_pubkeyhash_address,
};
pub use error::Error;
pub use fees::{
    data_deposit_fee, fungible_token_issuance_fee, nft_issuance_fee, token_change_authority_fee,
    token_freeze_fee, token_supply_change_fee,
};
pub use ids::{get_delegation_id, get_order_id, get_pool_id, get_token_id};
pub use inputs::{
    encode_input_for_change_token_authority, encode_input_for_change_token_metadata_uri,
    encode_input_for_conclude_order, encode_input_for_fill_order, encode_input_for_freeze_order,
    encode_input_for_freeze_token, encode_input_for_lock_token_supply,
    encode_input_for_mint_tokens, encode_input_for_unfreeze_token, encode_input_for_unmint_tokens,
    encode_input_for_utxo, encode_input_for_withdraw_from_delegation,
};
pub use intents::{
    encode_signed_transaction_intent, make_transaction_intent_message_to_sign,
    verify_transaction_intent,
};
pub use keys::{
    extended_public_key_from_extended_private_key, make_change_address,
    make_change_address_public_key, make_default_account_privkey, make_private_key,
    make_receiving_address, make_receiving_address_public_key, public_key_from_private_key,
};
pub use outputs::{
    encode_create_order_output, encode_output_coin_burn, encode_output_create_delegation,
    encode_output_create_stake_pool, encode_output_data_deposit, encode_output_delegate_staking,
    encode_output_htlc, encode_output_issue_fungible_token, encode_output_issue_nft,
    encode_output_lock_then_transfer, encode_output_produce_block_from_stake,
    encode_output_token_burn, encode_output_token_lock_then_transfer, encode_output_token_transfer,
    encode_output_transfer,
};
pub use signing::{
    encode_witness, encode_witness_htlc_refund_multisig, encode_witness_htlc_refund_single_sig,
    encode_witness_htlc_spend, encode_witness_no_signature, sign_challenge,
    sign_message_for_spending, verify_challenge, verify_signature_for_spending,
};
pub use staking::{
    effective_pool_balance, encode_stake_pool_data, staking_pool_spend_maturity_block_count,
};
pub use timelocks::{
    encode_lock_for_block_count, encode_lock_for_seconds, encode_lock_until_height,
    encode_lock_until_time,
};
pub use transactions::{
    decode_partially_signed_transaction_to_json, decode_signed_transaction_to_json,
    decode_transaction, decode_transaction_lenient, encode_outpoint_source_id,
    encode_partially_signed_transaction, encode_signed_transaction, encode_transaction,
    estimate_transaction_size, extract_htlc_secret, transaction_id,
};

pub use common::chain::signature::sighash::sighashtype::SigHashType;
pub use common::chain::tokens::{IsTokenFreezable, IsTokenUnfreezable, TokenTotalSupply};

/// The coin or token amount in atoms, the smallest indivisible unit
/// (1 ML = 100,000,000,000 atoms).
pub use common::primitives::Amount;

/// Additional per-input information required for sighash calculation after
/// the sighash commitments fork.
pub use common::chain::partially_signed_transaction::TxAdditionalInfo;

/// Key chain and transaction primitives re-exported from mintlayer-core, so
/// that SDK users do not need a direct dependency on it.
pub mod types {
    pub use ml_common::chain::signature::inputsig::InputWitness;
    pub use ml_common::chain::{
        Destination, OutPointSourceId, SignedTransaction, Transaction, TxInput, TxOutput,
        classic_multisig::ClassicMultisigChallenge, htlc::HtlcSecret, stakelock::StakePoolData,
        timelock::OutputTimeLock,
    };
    pub use ml_common::primitives::H256;
    pub use ml_crypto::key::{
        PrivateKey, PublicKey, Signature,
        extended::{ExtendedPrivateKey, ExtendedPublicKey},
    };
    pub use ml_serialization::{DecodeAll, Encode};
}

use common::chain::config::{Builder, ChainConfig, ChainType};

/// The network for which an operation is performed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Network {
    /// Production network.
    #[default]
    Mainnet,
    /// Public test network.
    Testnet,
    /// Local regression-testing network.
    Regtest,
    /// Signet.
    Signet,
}

impl From<Network> for ChainType {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => Self::Mainnet,
            Network::Testnet => Self::Testnet,
            Network::Regtest => Self::Regtest,
            Network::Signet => Self::Signet,
        }
    }
}

/// Whether a token can be frozen.
///
/// Alias of [`IsTokenFreezable`] kept for parity with the go-sdk
/// `FreezableToken` constants.
pub type FreezableToken = IsTokenFreezable;

/// Whether a token can be unfrozen once frozen.
///
/// Alias of [`IsTokenUnfreezable`] kept for parity with the go-sdk
/// `TokenUnfreezable` constants.
pub type TokenUnfreezable = IsTokenUnfreezable;

/// Where a transaction output comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    /// The output of a transaction.
    Transaction,
    /// The reward of a block.
    BlockReward,
}

/// Returns the cached [`ChainConfig`] for a network.
pub(crate) fn chain_config(network: Network) -> &'static ChainConfig {
    static CONFIGS: [OnceLock<ChainConfig>; 4] = [const { OnceLock::new() }; 4];
    CONFIGS[network as usize].get_or_init(|| Builder::new(ChainType::from(network)).build())
}

/// Parses a bech32 address into an addressable chain object.
pub(crate) fn parse_addressable<T: common::address::traits::Addressable>(
    network: Network,
    address: &str,
) -> Result<T, Error> {
    common::address::Address::from_string(chain_config(network), address)
        .map_err(|error| Error::AddressParse {
            address: address.to_owned(),
            message: error.to_string(),
        })
        .map(common::address::Address::into_object)
}
