// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Error type for the [`crypto`](crate::crypto) module.

/// Errors produced by the cryptography and transaction-building operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The BIP39 mnemonic could not be parsed.
    #[error("invalid mnemonic: {0}")]
    InvalidMnemonic(#[from] bip39::Error),
    /// A SCALE-encoded object could not be decoded.
    #[error("decoding failed: {0}")]
    Decoding(#[from] ml_serialization::Error),
    /// A bech32 address could not be parsed for the given network.
    #[error("failed to parse address `{address}`: {message}")]
    AddressParse {
        /// The offending address.
        address: String,
        /// The reason the address is invalid.
        message: String,
    },
    /// The key index has its most significant bit set, which is not allowed.
    #[error("key index must not have the most significant bit set")]
    InvalidKeyIndex,
    /// A fixed total supply was requested without supplying the cap.
    #[error("fixed total supply requires a supply amount")]
    FixedTotalSupplyWithoutAmount,
    /// The margin ratio is out of the valid range.
    #[error("invalid margin ratio per thousand: {0}")]
    InvalidMarginRatio(u16),
    /// Multisig challenges require at least one required signature.
    #[error("multisig requires at least one signature")]
    ZeroMultisigRequiredSignatures,
    /// The NFT creator field is not a valid public key.
    #[error("invalid NFT creator public key: {0}")]
    InvalidNftCreatorPublicKey(String),
    /// Token issuance parameters failed chain validation.
    #[error("invalid token parameters: {0}")]
    InvalidTokenParameters(String),
    /// The transaction could not be constructed from its inputs and outputs.
    #[error("failed to create transaction: {0}")]
    TransactionCreation(String),
    /// The signed transaction could not be constructed.
    #[error("failed to create signed transaction: {0}")]
    SignedTransactionCreation(String),
    /// The partially signed transaction failed its consistency check.
    #[error("failed to create partially signed transaction: {0}")]
    PartiallySignedTransactionCreation(String),
    /// An input witness could not be produced.
    #[error("failed to sign input: {0}")]
    InputSigning(String),
    /// The sighash could not be calculated.
    #[error("failed to calculate sighash: {0}")]
    Sighash(String),
    /// A message could not be signed.
    #[error("failed to sign message: {0}")]
    MessageSigning(String),
    /// A signature could not be verified.
    #[error("signature verification failed: {0}")]
    SignatureVerification(String),
    /// The HTLC secret hash is malformed.
    #[error("invalid HTLC secret hash: {0}")]
    InvalidHtlcSecretHash(String),
    /// The witness is not an HTLC spend of the expected variant.
    #[error("unexpected HTLC spend type")]
    UnexpectedHtlcSpendType,
    /// The transaction contains no input matching the given outpoint.
    #[error("no input found for the given outpoint")]
    NoInputOutpointFound,
    /// The transaction has fewer witnesses than the requested input index.
    #[error("no witness found for the given input index")]
    InvalidWitnessCount,
    /// The transaction size could not be estimated.
    #[error("failed to estimate transaction size: {0}")]
    SizeEstimation(String),
    /// The effective pool balance could not be calculated.
    #[error("failed to calculate effective pool balance: {0}")]
    EffectiveBalance(String),
    /// Order freezing requires the orders V1 fork to be active.
    #[error("orders V1 is not active at the specified height")]
    OrdersV1NotActivated,
    /// The intent message is not valid UTF-8.
    #[error("transaction intent message is not a valid UTF-8 string")]
    IntentMessageNotUtf8,
    /// The transaction id is not a valid hex string.
    #[error("invalid transaction id: {0}")]
    InvalidTransactionId(String),
    /// The signed intent could not be verified.
    #[error("transaction intent verification failed: {0}")]
    IntentVerification(String),
    /// A JSON representation could not be produced.
    #[error("failed to produce JSON: {0}")]
    Json(String),
}
