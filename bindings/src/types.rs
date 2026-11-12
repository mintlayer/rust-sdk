// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Foreign-language types shared across the exported surface: the signature
//! hash selector, token supply variants, freezability flags and the
//! additional-info records used when signing witnesses and assembling
//! partially signed transactions.
//!
//! Amounts cross the boundary as decimal atom strings because 128-bit atom
//! counts do not fit into Kotlin's `Long` or Swift's `Int`.

use mintlayer_sdk::crypto;
use mintlayer_sdk::crypto::TxAdditionalInfo as ChainTxAdditionalInfo;
use mintlayer_sdk::crypto::types::{DecodeAll, OrderId, OutputValue, PoolId, TokenId};
use mintlayer_sdk::crypto::{
    Amount, IsTokenFreezable, IsTokenUnfreezable, OrderAdditionalInfo, PoolAdditionalInfo,
    SigHashType,
};

use super::error::BindingsError;

/// The part of the transaction that will be committed in the signature
/// (Mintlayer's equivalent of Bitcoin's sighash flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SignatureHashType {
    /// Sign everything.
    All,
    /// Sign nothing.
    None,
    /// Sign only this input.
    Single,
    /// Sign only this input, at any position.
    AnyoneCanPay,
}

/// Converts the sighash selector into the chain type.
///
/// The chain type is a validated u8 newtype, so the conversion is fallible
/// and must not go through an infallible `From`.
pub(crate) fn sighash_type_from(value: SignatureHashType) -> Result<SigHashType, BindingsError> {
    let raw = match value {
        SignatureHashType::All => SigHashType::ALL,
        SignatureHashType::None => SigHashType::NONE,
        SignatureHashType::Single => SigHashType::SINGLE,
        SignatureHashType::AnyoneCanPay => SigHashType::ANYONECANPAY,
    };
    SigHashType::try_from(raw)
        .map_err(|_| super::error::decoding("invalid sighash type".to_owned()))
}

/// Whether a UTXO comes from a transaction output or a block reward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SourceId {
    /// The output of a transaction.
    Transaction,
    /// The reward of a block.
    BlockReward,
}

impl From<SourceId> for crypto::SourceId {
    fn from(value: SourceId) -> Self {
        match value {
            SourceId::Transaction => Self::Transaction,
            SourceId::BlockReward => Self::BlockReward,
        }
    }
}

/// The supply policy of a token, set at issuance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TotalSupply {
    /// Can be issued with no limit, then locked once to a fixed supply.
    Lockable,
    /// Unlimited supply, bounded only by the 128-bit amount type.
    Unlimited,
    /// The total supply is fixed at issuance; requires `supplyAmount`.
    Fixed,
}

/// Whether a token can be frozen by its authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FreezableToken {
    No,
    Yes,
}

impl From<FreezableToken> for IsTokenFreezable {
    fn from(value: FreezableToken) -> Self {
        match value {
            FreezableToken::No => Self::No,
            FreezableToken::Yes => Self::Yes,
        }
    }
}

/// Whether a frozen token can be unfrozen by its authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TokenUnfreezable {
    No,
    Yes,
}

impl From<TokenUnfreezable> for IsTokenUnfreezable {
    fn from(value: TokenUnfreezable) -> Self {
        match value {
            TokenUnfreezable::No => Self::No,
            TokenUnfreezable::Yes => Self::Yes,
        }
    }
}

/// An amount of coins or of a specific token.
///
/// `atoms` is a decimal string of the amount in atoms, the smallest
/// indivisible unit (1 ML = 100,000,000,000 atoms).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SimpleCurrencyAmount {
    /// An amount of the native coin.
    Coins {
        /// Amount in atoms, as a decimal string.
        atoms: String,
    },
    /// An amount of a token.
    TokenAmount {
        /// The token id, as a bech32 address string.
        token_id: String,
        /// Amount in atoms, as a decimal string.
        atoms: String,
    },
}

/// Additional information for one pool, keyed by its bech32 pool id.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PoolAdditionalInfoEntry {
    /// The pool id, as a bech32 address string.
    pub id: String,
    /// The pool's staker balance in atoms, as a decimal string.
    pub staker_balance: String,
}

/// Additional information for one order, keyed by its bech32 order id.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OrderAdditionalInfoEntry {
    /// The order id, as a bech32 address string.
    pub id: String,
    /// The amount the order initially asked for.
    pub initially_asked: SimpleCurrencyAmount,
    /// The amount the order was initially given.
    pub initially_given: SimpleCurrencyAmount,
    /// The order's current ask balance in atoms, as a decimal string.
    pub ask_balance: String,
    /// The order's current give balance in atoms, as a decimal string.
    pub give_balance: String,
}

/// Additional per-input information required for sighash calculation after
/// the sighash commitments fork.
///
/// It must contain: for each `ProduceBlockFromStake` input of the
/// transaction, the pool info of the referenced pool; for each `FillOrder`
/// and `ConcludeOrder` input, the order info of the referenced order.
/// Entries not referenced by any input are harmless.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TxAdditionalInfo {
    /// Pool info entries, one per referenced pool.
    pub pool_info: Vec<PoolAdditionalInfoEntry>,
    /// Order info entries, one per referenced order.
    pub order_info: Vec<OrderAdditionalInfoEntry>,
}

/// Parses a decimal atom string into an amount.
pub(crate) fn amount_from_atoms(atoms: &str) -> Result<Amount, BindingsError> {
    // Reject non-digit inputs (a leading `+` or whitespace would parse via
    // `u128::from_str` but is not accepted by the reference implementation).
    if atoms.is_empty() || !atoms.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(BindingsError::InvalidAmount {
            message: "amount must be a decimal digit string".to_owned(),
        });
    }
    atoms
        .parse::<u128>()
        .map(Amount::from_atoms)
        .map_err(|error| BindingsError::InvalidAmount {
            message: error.to_string(),
        })
}

/// Formats an amount as a decimal atom string.
pub(crate) fn amount_to_atoms(amount: Amount) -> String {
    amount.into_atoms().to_string()
}

/// Decodes a list of runtime blobs into typed objects.
pub(crate) fn decode_list<T: DecodeAll>(
    blobs: Vec<Vec<u8>>,
    what: &'static str,
) -> Result<Vec<T>, BindingsError> {
    blobs
        .into_iter()
        .map(|blob| {
            T::decode_all(&mut &blob[..])
                .map_err(|error| super::error::decoding(format!("{what}: {error}")))
        })
        .collect()
}

/// Converts the FFI additional-info record into the chain type.
pub(crate) fn additional_info_from(
    network: crypto::Network,
    info: &TxAdditionalInfo,
) -> Result<ChainTxAdditionalInfo, BindingsError> {
    let mut converted = ChainTxAdditionalInfo::new();
    for entry in &info.pool_info {
        let pool_id: PoolId = crypto::parse_addressable(network, &entry.id)?;
        converted.add_pool_info(
            pool_id,
            PoolAdditionalInfo {
                staker_balance: amount_from_atoms(&entry.staker_balance)?,
            },
        );
    }
    for entry in &info.order_info {
        let order_id = crypto::parse_addressable::<OrderId>(network, &entry.id)?;
        converted.add_order_info(
            order_id,
            OrderAdditionalInfo {
                initially_asked: output_value_from(&entry.initially_asked, network)?,
                initially_given: output_value_from(&entry.initially_given, network)?,
                ask_balance: amount_from_atoms(&entry.ask_balance)?,
                give_balance: amount_from_atoms(&entry.give_balance)?,
            },
        );
    }
    Ok(converted)
}

/// Converts a coins-or-tokens selector into an output value.
fn output_value_from(
    amount: &SimpleCurrencyAmount,
    network: crypto::Network,
) -> Result<OutputValue, BindingsError> {
    match amount {
        SimpleCurrencyAmount::Coins { atoms } => Ok(OutputValue::Coin(amount_from_atoms(atoms)?)),
        SimpleCurrencyAmount::TokenAmount { token_id, atoms } => Ok(OutputValue::TokenV1(
            crypto::parse_addressable::<TokenId>(network, token_id)?,
            amount_from_atoms(atoms)?,
        )),
    }
}

/// Decodes an optional runtime blob into an optional typed object.
pub(crate) fn decode_optional<T: DecodeAll>(
    blob: Option<Vec<u8>>,
    what: &'static str,
) -> Result<Option<T>, BindingsError> {
    blob.map(|bytes| {
        T::decode_all(&mut &bytes[..])
            .map_err(|error| super::error::decoding(format!("{what}: {error}")))
    })
    .transpose()
}
