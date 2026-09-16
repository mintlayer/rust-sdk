// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Signed transaction intents.

use ml_common as common;
use ml_serialization as serialization;

use common::chain::SignedTransactionIntent;
use common::primitives::Id;
use serialization::DecodeAll;

use super::{Error, Network, parse_addressable, transactions::h256_from_str};

/// Returns the message that has to be signed for a transaction intent.
///
/// Every input destination of the transaction must sign this message with
/// [`sign_challenge`](super::sign_challenge).
pub fn make_transaction_intent_message_to_sign(
    intent: &str,
    transaction_id_hex: &str,
) -> Result<String, Error> {
    let hash = h256_from_str(transaction_id_hex)?;
    let transaction_id = Id::<common::chain::Transaction>::new(hash);
    Ok(SignedTransactionIntent::get_message_to_sign(
        intent,
        &transaction_id,
    ))
}

/// Assembles a signed transaction intent.
///
/// `signatures` holds one challenge signature per transaction input, in
/// input order, produced by
/// [`sign_challenge`](super::sign_challenge).
pub fn encode_signed_transaction_intent(
    signed_message: &str,
    signatures: Vec<Vec<u8>>,
) -> Result<SignedTransactionIntent, Error> {
    Ok(SignedTransactionIntent::from_components_unchecked(
        signed_message.to_owned(),
        signatures,
    ))
}

/// Verifies a signed transaction intent.
///
/// `input_destinations` holds the address of each transaction input
/// destination; public key and public key hash addresses are treated
/// interchangeably.
pub fn verify_transaction_intent(
    expected_signed_message: &str,
    encoded_signed_intent: &[u8],
    input_destinations: &[&str],
    network: Network,
) -> Result<(), Error> {
    let signed_intent = SignedTransactionIntent::decode_all(&mut &encoded_signed_intent[..])
        .map_err(Error::from)?;

    let destinations = input_destinations
        .iter()
        .map(|address| parse_addressable::<common::chain::Destination>(network, address))
        .collect::<Result<Vec<_>, Error>>()?;

    signed_intent
        .verify(
            super::chain_config(network),
            &destinations,
            expected_signed_message,
        )
        .map_err(|error| Error::IntentVerification(error.to_string()))
}
