// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Input witness production and message signing.

use ml_common as common;
use ml_crypto as crypto;
use ml_randomness as randomness;

use common::chain::Destination;
use common::chain::Transaction;
use common::chain::classic_multisig::ClassicMultisigChallenge;
use common::chain::htlc::HtlcSecret;
use common::chain::signature::{
    inputsig::{
        InputWitness,
        arbitrary_message::{ArbitraryMessageSignature, produce_message_challenge},
        authorize_hashed_timelock_contract_spend::AuthorizedHashedTimelockContractSpend,
        classical_multisig::authorize_classical_multisig::{
            AuthorizedClassicalMultisigSpend, sign_classical_multisig_spending,
        },
        htlc::{
            produce_uniparty_signature_for_htlc_refunding,
            produce_uniparty_signature_for_htlc_spending,
        },
        standard_signature::StandardInputSignature,
    },
    sighash::signature_hash,
};
use crypto::key::{PrivateKey, PublicKey, Signature};
use ml_serialization as serialization;
use serialization::Encode;

use super::{
    Error, Network, SigHashType, TxAdditionalInfo, parse_addressable,
    transactions::make_input_commitments,
};

/// Signs one input of a transaction and returns the input witness.
///
/// `input_owner_destination` is the address that can spend the input.
/// `input_utxos` must hold one entry per transaction input: `None` for
/// non-UTXO inputs, `Some(output)` otherwise. `additional_info` must cover
/// every `ProduceBlockFromStake`, `FillOrder` and `ConcludeOrder` input of
/// the transaction, not just the input being signed.
#[allow(clippy::too_many_arguments)]
pub fn encode_witness(
    sighash_type: SigHashType,
    private_key: &PrivateKey,
    input_owner_destination: &str,
    transaction: &Transaction,
    input_utxos: &[Option<common::chain::TxOutput>],
    input_index: usize,
    additional_info: &TxAdditionalInfo,
    current_block_height: u64,
    network: Network,
) -> Result<InputWitness, Error> {
    let destination = parse_addressable(network, input_owner_destination)?;
    let input_commitments = make_input_commitments(
        transaction,
        input_utxos,
        additional_info,
        network,
        current_block_height,
    )?;

    StandardInputSignature::produce_uniparty_signature_for_input(
        private_key,
        sighash_type,
        destination,
        transaction,
        &input_commitments,
        input_index,
        &mut randomness::make_true_rng(),
    )
    .map(InputWitness::Standard)
    .map_err(|error| Error::InputSigning(error.to_string()))
}

/// Returns the empty witness used for unsigned inputs such as fill-order.
pub fn encode_witness_no_signature() -> InputWitness {
    InputWitness::NoSignature(None)
}

/// Signs an HTLC input for spending, revealing the secret.
///
/// See [`encode_witness`] for the remaining parameters.
#[allow(clippy::too_many_arguments)]
pub fn encode_witness_htlc_spend(
    sighash_type: SigHashType,
    private_key: &PrivateKey,
    input_owner_destination: &str,
    transaction: &Transaction,
    input_utxos: &[Option<common::chain::TxOutput>],
    input_index: usize,
    secret: HtlcSecret,
    additional_info: &TxAdditionalInfo,
    current_block_height: u64,
    network: Network,
) -> Result<InputWitness, Error> {
    let destination = parse_addressable(network, input_owner_destination)?;
    let input_commitments = make_input_commitments(
        transaction,
        input_utxos,
        additional_info,
        network,
        current_block_height,
    )?;

    produce_uniparty_signature_for_htlc_spending(
        private_key,
        sighash_type,
        destination,
        transaction,
        &input_commitments,
        input_index,
        secret,
        &mut randomness::make_true_rng(),
    )
    .map(InputWitness::Standard)
    .map_err(|error| Error::InputSigning(error.to_string()))
}

/// Signs an HTLC refund input for a single-sig refund address.
///
/// See [`encode_witness`] for the remaining parameters.
#[allow(clippy::too_many_arguments)]
pub fn encode_witness_htlc_refund_single_sig(
    sighash_type: SigHashType,
    private_key: &PrivateKey,
    input_owner_destination: &str,
    transaction: &Transaction,
    input_utxos: &[Option<common::chain::TxOutput>],
    input_index: usize,
    additional_info: &TxAdditionalInfo,
    current_block_height: u64,
    network: Network,
) -> Result<InputWitness, Error> {
    let destination = parse_addressable(network, input_owner_destination)?;
    let input_commitments = make_input_commitments(
        transaction,
        input_utxos,
        additional_info,
        network,
        current_block_height,
    )?;

    produce_uniparty_signature_for_htlc_refunding(
        private_key,
        sighash_type,
        destination,
        transaction,
        &input_commitments,
        input_index,
        &mut randomness::make_true_rng(),
    )
    .map(InputWitness::Standard)
    .map_err(|error| Error::InputSigning(error.to_string()))
}

/// Adds one signature to a multisig HTLC refund witness.
///
/// `key_index` is the index of the public key, inside `challenge`, that
/// corresponds to `private_key`. `input_witness` is `None` for the first
/// signer or the witness produced by a previous call, so that several
/// signers can build the witness cumulatively.
#[allow(clippy::too_many_arguments)]
pub fn encode_witness_htlc_refund_multisig(
    sighash_type: SigHashType,
    private_key: &PrivateKey,
    key_index: u8,
    input_witness: Option<&InputWitness>,
    challenge: &ClassicMultisigChallenge,
    transaction: &Transaction,
    input_utxos: &[Option<common::chain::TxOutput>],
    input_index: usize,
    additional_info: &TxAdditionalInfo,
    current_block_height: u64,
    network: Network,
) -> Result<InputWitness, Error> {
    let input_commitments = make_input_commitments(
        transaction,
        input_utxos,
        additional_info,
        network,
        current_block_height,
    )?;

    let sighash = signature_hash(sighash_type, transaction, &input_commitments, input_index)
        .map_err(|error| Error::Sighash(error.to_string()))?;

    let authorization = match input_witness {
        Some(witness) => {
            let (htlc_spend, _) = extract_htlc_spend(witness)?;
            match htlc_spend {
                AuthorizedHashedTimelockContractSpend::Spend(_, _) => {
                    return Err(Error::UnexpectedHtlcSpendType);
                }
                AuthorizedHashedTimelockContractSpend::Refund(raw_signature) => {
                    AuthorizedClassicalMultisigSpend::from_data(&raw_signature)
                        .map_err(|error| Error::InputSigning(error.to_string()))?
                }
            }
        }
        None => AuthorizedClassicalMultisigSpend::new_empty(challenge.clone()),
    };

    let authorization = sign_classical_multisig_spending(
        super::chain_config(network),
        key_index,
        private_key,
        challenge,
        &sighash,
        authorization,
        &mut randomness::make_true_rng(),
    )
    .map_err(|error| Error::InputSigning(error.to_string()))?
    .take();

    let raw_signature = AuthorizedHashedTimelockContractSpend::Refund(authorization.encode());
    Ok(InputWitness::Standard(StandardInputSignature::new(
        sighash_type,
        raw_signature.encode(),
    )))
}

/// Signs an arbitrary message with a private key, producing a witness
/// signature for spending requests.
pub fn sign_message_for_spending(
    private_key: &PrivateKey,
    message: &[u8],
) -> Result<Signature, Error> {
    private_key
        .sign_message(message, &mut randomness::make_true_rng())
        .map_err(|error| Error::MessageSigning(error.to_string()))
}

/// Verifies a spending signature against a public key and message.
pub fn verify_signature_for_spending(
    public_key: &PublicKey,
    signature: &Signature,
    message: &[u8],
) -> bool {
    public_key.verify_message(signature, message)
}

/// Signs a challenge message with a private key.
///
/// The result can be verified with
/// [`verify_challenge`](super::verify_challenge) against the pubkeyhash
/// address of the signing key.
pub fn sign_challenge(private_key: &PrivateKey, message: &[u8]) -> Result<Vec<u8>, Error> {
    ArbitraryMessageSignature::produce_uniparty_signature_as_pub_key_hash_spending(
        private_key,
        message,
        &mut randomness::make_true_rng(),
    )
    .map(|signature| signature.into_raw())
    .map_err(|error| Error::MessageSigning(error.to_string()))
}

/// Verifies a challenge signature against a pubkeyhash address.
///
/// Returns `true` on success; a failed verification yields an error.
pub fn verify_challenge(
    address: &str,
    network: Network,
    signed_challenge: &[u8],
    message: &[u8],
) -> Result<bool, Error> {
    let destination = parse_addressable::<Destination>(network, address)?;
    let message_challenge = produce_message_challenge(message);
    let signature = ArbitraryMessageSignature::from_data(signed_challenge.to_vec());
    signature
        .verify_signature(
            super::chain_config(network),
            &destination,
            &message_challenge,
        )
        .map_err(|error| Error::SignatureVerification(error.to_string()))?;
    Ok(true)
}

pub(crate) fn extract_htlc_spend(
    witness: &InputWitness,
) -> Result<(AuthorizedHashedTimelockContractSpend, SigHashType), Error> {
    match witness {
        InputWitness::NoSignature(_) => Err(Error::UnexpectedHtlcSpendType),
        InputWitness::Standard(signature) => Ok((
            AuthorizedHashedTimelockContractSpend::from_data(signature.raw_signature())
                .map_err(|error| Error::InputSigning(error.to_string()))?,
            signature.sighash_type(),
        )),
    }
}
