// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Signed transaction intents: the login-style message flow that proves
//! control over every input destination of a transaction.

use mintlayer_sdk::crypto::types::{DecodeAll, Encode, PrivateKey};

use super::error::{BindingsError, decoding};
use super::network::Network;
use mintlayer_sdk::crypto;

/// Returns the message that every input destination of the transaction
/// must sign, as bytes.
///
/// Each destination signs the message with
/// [`signChallenge`](crate::sign_challenge).
#[uniffi::export]
pub fn make_transaction_intent_message_to_sign(
    intent: String,
    transaction_id: String,
) -> Result<Vec<u8>, BindingsError> {
    let message = crypto::make_transaction_intent_message_to_sign(&intent, &transaction_id)?;
    Ok(message.into_bytes())
}

/// Assembles a signed transaction intent from the signed message and one
/// challenge signature per input destination, in input order.
#[uniffi::export]
pub fn encode_signed_transaction_intent(
    signed_message: Vec<u8>,
    signatures: Vec<Vec<u8>>,
) -> Result<Vec<u8>, BindingsError> {
    let signed_message =
        String::from_utf8(signed_message).map_err(|_| BindingsError::IntentMessageNotUtf8)?;
    let intent = crypto::encode_signed_transaction_intent(&signed_message, signatures)?;
    Ok(intent.encode())
}

/// Verifies a signed transaction intent.
///
/// `inputDestinations` holds the bech32 address of each input destination;
/// public key and public key hash addresses are treated interchangeably.
/// Returns `true` on success; a failed verification is an error.
#[uniffi::export]
pub fn verify_transaction_intent(
    expected_signed_message: Vec<u8>,
    encoded_signed_intent: Vec<u8>,
    input_destinations: Vec<String>,
    network: Network,
) -> Result<bool, BindingsError> {
    let expected_signed_message = String::from_utf8(expected_signed_message)
        .map_err(|_| BindingsError::IntentMessageNotUtf8)?;
    let destinations: Vec<&str> = input_destinations.iter().map(String::as_str).collect();
    crypto::verify_transaction_intent(
        &expected_signed_message,
        &encoded_signed_intent,
        &destinations,
        network.into(),
    )?;
    Ok(true)
}

/// Convenience wrapper over the three intent steps: computes the
/// transaction id from the signed transaction, builds the message to
/// sign, signs it with every private key in input order, encodes the
/// intent, and self-verifies against the input destinations before
/// returning the encoded bytes the bridge API accepts as
/// `deposit_transactions[].intent`.
#[uniffi::export]
pub fn sign_transaction_intent(
    intent: String,
    signed_transaction: Vec<u8>,
    private_keys: Vec<Vec<u8>>,
    input_destinations: Vec<String>,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    if private_keys.len() != input_destinations.len() {
        return Err(BindingsError::IntentKeyDestinationCountMismatch {
            keys: private_keys.len() as u32,
            destinations: input_destinations.len() as u32,
        });
    }
    let transaction_id = crypto::transaction_id(&crypto::decode_transaction(&signed_transaction)?);
    let message = crypto::make_transaction_intent_message_to_sign(&intent, &transaction_id)?;
    let mut signatures = Vec::with_capacity(private_keys.len());
    for key in &private_keys {
        let key = PrivateKey::decode_all(&mut &key[..]).map_err(decoding)?;
        signatures.push(crypto::sign_challenge(&key, message.as_bytes())?);
    }
    let signed_intent = crypto::encode_signed_transaction_intent(&message, signatures)?;
    let encoded = signed_intent.encode();
    let destinations: Vec<&str> = input_destinations.iter().map(String::as_str).collect();
    crypto::verify_transaction_intent(&message, &encoded, &destinations, network.into())?;
    Ok(encoded)
}
