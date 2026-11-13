// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Signed transaction intents: the login-style message flow that proves
//! control over every input destination of a transaction.

use mintlayer_sdk::crypto::types::Encode;

use super::error::BindingsError;
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
