// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! The error type surfaced to the foreign-language bindings.

use mintlayer_sdk::crypto;

/// Errors produced by the wallet primitive operations.
///
/// Every fallible exported function returns one of these variants; no error
/// crosses the FFI boundary as a panic or an opaque string.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum BindingsError {
    /// The BIP39 mnemonic could not be parsed.
    #[error("invalid mnemonic: {detail}")]
    InvalidMnemonic {
        /// Why the mnemonic is invalid.
        detail: String,
    },
    /// A runtime-format blob could not be decoded.
    #[error("decoding failed: {detail}")]
    Decoding {
        /// Why decoding failed.
        detail: String,
    },
    /// A bech32 address could not be parsed for the given network.
    #[error("failed to parse address `{address}`: {detail}")]
    AddressParse {
        /// The offending address.
        address: String,
        /// Why the address is invalid.
        detail: String,
    },
    /// A key index has its most significant bit set, which is not allowed.
    #[error("key index must not have the most significant bit set")]
    InvalidKeyIndex,
    /// A fixed total supply was requested without supplying the cap.
    #[error("fixed total supply requires a supply amount")]
    FixedTotalSupplyWithoutAmount,
    /// The margin ratio is out of the valid range.
    #[error("invalid margin ratio per thousand: {ratio}")]
    InvalidMarginRatio {
        /// The rejected ratio.
        ratio: u16,
    },
    /// A multisig challenge requires at least one required signature.
    #[error("multisig requires at least one signature")]
    ZeroMultisigRequiredSignatures,
    /// The NFT creator field is not a valid public key.
    #[error("invalid NFT creator public key: {detail}")]
    InvalidNftCreatorPublicKey {
        /// The rejected value.
        detail: String,
    },
    /// Token issuance parameters failed chain validation.
    #[error("invalid token parameters: {detail}")]
    InvalidTokenParameters {
        /// Why the parameters are invalid.
        detail: String,
    },
    /// The transaction could not be constructed from its inputs and outputs.
    #[error("failed to create transaction: {detail}")]
    TransactionCreation {
        /// Why construction failed.
        detail: String,
    },
    /// The signed transaction could not be constructed.
    #[error("failed to create signed transaction: {detail}")]
    SignedTransactionCreation {
        /// Why construction failed.
        detail: String,
    },
    /// The partially signed transaction failed its consistency check.
    #[error("failed to create partially signed transaction: {detail}")]
    PartiallySignedTransactionCreation {
        /// Why construction failed.
        detail: String,
    },
    /// An input witness could not be produced.
    #[error("failed to sign input: {detail}")]
    InputSigning {
        /// Why signing failed.
        detail: String,
    },
    /// Key generation or derivation failed.
    #[error("failed to derive key: {detail}")]
    KeyDerivation {
        /// Why derivation failed.
        detail: String,
    },
    /// The sighash could not be calculated.
    #[error("failed to calculate sighash: {detail}")]
    Sighash {
        /// Why the calculation failed.
        detail: String,
    },
    /// A message could not be signed.
    #[error("failed to sign detail: {detail}")]
    MessageSigning {
        /// Why signing failed.
        detail: String,
    },
    /// A signature could not be verified.
    #[error("signature verification failed: {detail}")]
    SignatureVerification {
        /// Why verification failed.
        detail: String,
    },
    /// The HTLC secret hash is malformed.
    #[error("invalid HTLC secret hash: {detail}")]
    InvalidHtlcSecretHash {
        /// The rejected value.
        detail: String,
    },
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
    #[error("failed to estimate transaction size: {detail}")]
    SizeEstimation {
        /// Why the estimation failed.
        detail: String,
    },
    /// The effective pool balance could not be calculated.
    #[error("failed to calculate effective pool balance: {detail}")]
    EffectiveBalance {
        /// Why the calculation failed.
        detail: String,
    },
    /// Order freezing requires the orders V1 fork to be active at the height.
    #[error("orders V1 is not active at the specified height")]
    OrdersV1NotActivated,
    /// The transaction intent message is not valid UTF-8.
    #[error("transaction intent message is not a valid UTF-8 string")]
    IntentMessageNotUtf8,
    /// An amount string could not be parsed as a 128-bit atom count.
    #[error("invalid amount: {detail}")]
    InvalidAmount {
        /// The rejected value.
        detail: String,
    },
    /// The transaction id is not a valid hex string.
    #[error("invalid transaction id: {detail}")]
    InvalidTransactionId {
        /// The rejected value.
        detail: String,
    },
    /// The signed intent could not be verified.
    #[error("transaction intent verification failed: {detail}")]
    IntentVerification {
        /// Why verification failed.
        detail: String,
    },
    /// A JSON representation could not be produced.
    #[error("failed to produce JSON: {detail}")]
    Json {
        /// Why the production failed.
        detail: String,
    },
}

/// Maximum length of caller-supplied strings echoed in error payloads, so
/// oversized garbage cannot inflate host-side logs and crash reports.
fn clamp(value: String) -> String {
    const MAX: usize = 512;
    // Control characters (including newlines and ANSI escapes), the bidi
    // and format marks used for visual spoofing, and the Unicode line
    // separators are dropped so echoed garbage cannot forge or spoof
    // host-side log entries.
    const SPOOFING: [char; 22] = [
        '\u{00AD}', '\u{061C}', '\u{180E}', '\u{200B}', '\u{200C}', '\u{200D}', '\u{200E}',
        '\u{200F}', '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}', '\u{2060}',
        '\u{2061}', '\u{2062}', '\u{2063}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
        '\u{FEFF}',
    ];
    let sanitized: String = value
        .chars()
        .filter(|c| {
            !c.is_control()
                && !SPOOFING.contains(c)
                && !('\u{E0000}'..='\u{E007F}').contains(c)
                && *c != '\u{2028}'
                && *c != '\u{2029}'
        })
        .collect();
    if sanitized.len() <= MAX {
        return sanitized;
    }
    // Truncate on a char boundary: byte 512 may fall inside a multi-byte
    // character, and slicing there would panic on attacker-controlled input.
    let mut end = MAX;
    while !sanitized.is_char_boundary(end) {
        end -= 1;
    }
    sanitized[..end].to_owned()
}

impl From<crypto::Error> for BindingsError {
    fn from(error: crypto::Error) -> Self {
        match error {
            crypto::Error::InvalidMnemonic(e) => Self::InvalidMnemonic {
                detail: clamp(e.to_string()),
            },
            crypto::Error::Decoding(e) => Self::Decoding {
                detail: clamp(e.to_string()),
            },
            crypto::Error::AddressParse { address, message } => Self::AddressParse {
                address: clamp(address),
                detail: clamp(message),
            },
            crypto::Error::InvalidKeyIndex => Self::InvalidKeyIndex,
            crypto::Error::FixedTotalSupplyWithoutAmount => Self::FixedTotalSupplyWithoutAmount,
            crypto::Error::InvalidMarginRatio(ratio) => Self::InvalidMarginRatio { ratio },
            crypto::Error::ZeroMultisigRequiredSignatures => Self::ZeroMultisigRequiredSignatures,
            crypto::Error::InvalidNftCreatorPublicKey(message) => {
                Self::InvalidNftCreatorPublicKey {
                    detail: clamp(message),
                }
            }
            crypto::Error::InvalidTokenParameters(message) => Self::InvalidTokenParameters {
                detail: clamp(message),
            },
            crypto::Error::TransactionCreation(message) => Self::TransactionCreation {
                detail: clamp(message),
            },
            crypto::Error::SignedTransactionCreation(message) => Self::SignedTransactionCreation {
                detail: clamp(message),
            },
            crypto::Error::PartiallySignedTransactionCreation(message) => {
                Self::PartiallySignedTransactionCreation {
                    detail: clamp(message),
                }
            }
            crypto::Error::InputSigning(message) => Self::InputSigning {
                detail: clamp(message),
            },
            crypto::Error::KeyDerivation(message) => Self::KeyDerivation {
                detail: clamp(message),
            },
            crypto::Error::Sighash(message) => Self::Sighash {
                detail: clamp(message),
            },
            crypto::Error::MessageSigning(message) => Self::MessageSigning {
                detail: clamp(message),
            },
            crypto::Error::SignatureVerification(message) => Self::SignatureVerification {
                detail: clamp(message),
            },
            crypto::Error::InvalidHtlcSecretHash(message) => Self::InvalidHtlcSecretHash {
                detail: clamp(message),
            },
            crypto::Error::UnexpectedHtlcSpendType => Self::UnexpectedHtlcSpendType,
            crypto::Error::NoInputOutpointFound => Self::NoInputOutpointFound,
            crypto::Error::InvalidWitnessCount => Self::InvalidWitnessCount,
            crypto::Error::SizeEstimation(message) => Self::SizeEstimation {
                detail: clamp(message),
            },
            crypto::Error::EffectiveBalance(message) => Self::EffectiveBalance {
                detail: clamp(message),
            },
            crypto::Error::OrdersV1NotActivated => Self::OrdersV1NotActivated,
            crypto::Error::IntentMessageNotUtf8 => Self::IntentMessageNotUtf8,
            crypto::Error::InvalidTransactionId(message) => Self::InvalidTransactionId {
                detail: clamp(message),
            },
            crypto::Error::IntentVerification(message) => Self::IntentVerification {
                detail: clamp(message),
            },
            crypto::Error::Json(message) => Self::Json {
                detail: clamp(message),
            },
        }
    }
}

/// Maps a runtime-format decoding failure to [`BindingsError::Decoding`].
pub(crate) fn decoding<E: std::fmt::Display>(error: E) -> BindingsError {
    BindingsError::Decoding {
        detail: error.to_string(),
    }
}
