// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Message signing and transaction input witnesses.

use mintlayer_sdk::crypto::types::{
    ClassicMultisigChallenge, DecodeAll, Encode, HtlcSecret, InputWitness, PrivateKey, PublicKey,
    Signature, Transaction,
};

use super::error::{BindingsError, decoding};
use super::network::Network;
use super::types::{
    SignatureHashType, TxAdditionalInfo, additional_info_from, decode_optional, sighash_type_from,
};
use mintlayer_sdk::crypto;

/// Signs a message with a private key, for spending purposes (e.g. input
/// witnesses).
#[uniffi::export]
pub fn sign_message_for_spending(
    private_key: Vec<u8>,
    message: Vec<u8>,
) -> Result<Vec<u8>, BindingsError> {
    let key = PrivateKey::decode_all(&mut &private_key[..]).map_err(decoding)?;
    Ok(crypto::sign_message_for_spending(&key, &message)?.encode())
}

/// Verifies a spending signature against a public key and message.
#[uniffi::export]
pub fn verify_signature_for_spending(
    public_key: Vec<u8>,
    signature: Vec<u8>,
    message: Vec<u8>,
) -> Result<bool, BindingsError> {
    let key = PublicKey::decode_all(&mut &public_key[..]).map_err(decoding)?;
    let signature = Signature::decode_all(&mut &signature[..]).map_err(decoding)?;
    Ok(crypto::verify_signature_for_spending(
        &key, &signature, &message,
    ))
}

/// Signs an arbitrary message as a uniparty pubkey-hash challenge
/// (login/authentication style signatures).
#[uniffi::export]
pub fn sign_challenge(private_key: Vec<u8>, message: Vec<u8>) -> Result<Vec<u8>, BindingsError> {
    let key = PrivateKey::decode_all(&mut &private_key[..]).map_err(decoding)?;
    crypto::sign_challenge(&key, &message).map_err(BindingsError::from)
}

/// Verifies a challenge signature against a bech32 address.
///
/// Returns `true` on success; a failed verification is an error.
#[uniffi::export]
pub fn verify_challenge(
    address: String,
    network: Network,
    signed_challenge: Vec<u8>,
    message: Vec<u8>,
) -> Result<bool, BindingsError> {
    crypto::verify_challenge(&address, network.into(), &signed_challenge, &message)?;
    Ok(true)
}

/// Encodes a no-signature witness for an input that requires none.
#[uniffi::export]
pub fn encode_witness_no_signature() -> Vec<u8> {
    crypto::encode_witness_no_signature().encode()
}

/// Signs one input of the transaction and encodes the witness.
///
/// `inputUtxos` has one entry per transaction input: `null` for non-UTXO
/// inputs, otherwise the encoded output created by the corresponding
/// `encodeOutput*` function. `additionalInfo` must cover every
/// `ProduceBlockFromStake`, `FillOrder` and `ConcludeOrder` input of the
/// transaction (not just this one).
#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub fn encode_witness(
    sighash_type: SignatureHashType,
    private_key: Vec<u8>,
    input_owner_destination: String,
    transaction: Vec<u8>,
    input_utxos: Vec<Option<Vec<u8>>>,
    input_index: u32,
    additional_info: TxAdditionalInfo,
    current_block_height: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let key = PrivateKey::decode_all(&mut &private_key[..]).map_err(decoding)?;
    let transaction = Transaction::decode_all(&mut &transaction[..]).map_err(decoding)?;
    let input_utxos: Vec<Option<mintlayer_sdk::crypto::types::TxOutput>> = input_utxos
        .into_iter()
        .map(|blob| decode_optional(blob, "input utxo"))
        .collect::<Result<_, _>>()?;
    let additional_info = additional_info_from(network.into(), &additional_info)?;
    let witness = crypto::encode_witness(
        sighash_type_from(sighash_type)?,
        &key,
        &input_owner_destination,
        &transaction,
        &input_utxos,
        input_index as usize,
        &additional_info,
        current_block_height,
        network.into(),
    )?;
    Ok(witness.encode())
}

/// Signs an HTLC input for spending, revealing the secret.
///
/// See [`encodeWitness`](crate::encode_witness) for the remaining
/// parameters.
#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub fn encode_witness_htlc_spend(
    sighash_type: SignatureHashType,
    private_key: Vec<u8>,
    input_owner_destination: String,
    transaction: Vec<u8>,
    input_utxos: Vec<Option<Vec<u8>>>,
    input_index: u32,
    secret: Vec<u8>,
    additional_info: TxAdditionalInfo,
    current_block_height: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let key = PrivateKey::decode_all(&mut &private_key[..]).map_err(decoding)?;
    let transaction = Transaction::decode_all(&mut &transaction[..]).map_err(decoding)?;
    let input_utxos: Vec<Option<mintlayer_sdk::crypto::types::TxOutput>> = input_utxos
        .into_iter()
        .map(|blob| decode_optional(blob, "input utxo"))
        .collect::<Result<_, _>>()?;
    let secret = HtlcSecret::decode_all(&mut &secret[..]).map_err(decoding)?;
    let additional_info = additional_info_from(network.into(), &additional_info)?;
    let witness = crypto::encode_witness_htlc_spend(
        sighash_type_from(sighash_type)?,
        &key,
        &input_owner_destination,
        &transaction,
        &input_utxos,
        input_index as usize,
        secret,
        &additional_info,
        current_block_height,
        network.into(),
    )?;
    Ok(witness.encode())
}

/// Signs an HTLC refund input (single signature).
#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub fn encode_witness_htlc_refund_single_sig(
    sighash_type: SignatureHashType,
    private_key: Vec<u8>,
    input_owner_destination: String,
    transaction: Vec<u8>,
    input_utxos: Vec<Option<Vec<u8>>>,
    input_index: u32,
    additional_info: TxAdditionalInfo,
    current_block_height: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let key = PrivateKey::decode_all(&mut &private_key[..]).map_err(decoding)?;
    let transaction = Transaction::decode_all(&mut &transaction[..]).map_err(decoding)?;
    let input_utxos: Vec<Option<mintlayer_sdk::crypto::types::TxOutput>> = input_utxos
        .into_iter()
        .map(|blob| decode_optional(blob, "input utxo"))
        .collect::<Result<_, _>>()?;
    let additional_info = additional_info_from(network.into(), &additional_info)?;
    let witness = crypto::encode_witness_htlc_refund_single_sig(
        sighash_type_from(sighash_type)?,
        &key,
        &input_owner_destination,
        &transaction,
        &input_utxos,
        input_index as usize,
        &additional_info,
        current_block_height,
        network.into(),
    )?;
    Ok(witness.encode())
}

/// Signs an HTLC refund input on behalf of a multisig challenge.
///
/// `inputWitness` is the (optional) partial witness of the preceding
/// signer — `null`, or an empty array for the first signer;
/// `multisigChallenge` is the blob produced by
/// [`encodeMultisigChallenge`](crate::encode_multisig_challenge).
#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub fn encode_witness_htlc_refund_multisig(
    sighash_type: SignatureHashType,
    private_key: Vec<u8>,
    key_index: u8,
    input_witness: Option<Vec<u8>>,
    multisig_challenge: Vec<u8>,
    transaction: Vec<u8>,
    input_utxos: Vec<Option<Vec<u8>>>,
    input_index: u32,
    additional_info: TxAdditionalInfo,
    current_block_height: u64,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let key = PrivateKey::decode_all(&mut &private_key[..]).map_err(decoding)?;
    // An empty blob is the wasm contract's "first signer" form.
    let input_witness = decode_optional::<InputWitness>(
        input_witness.filter(|blob| !blob.is_empty()),
        "input witness",
    )?;
    let challenge =
        ClassicMultisigChallenge::decode_all(&mut &multisig_challenge[..]).map_err(decoding)?;
    let transaction = Transaction::decode_all(&mut &transaction[..]).map_err(decoding)?;
    let input_utxos: Vec<Option<mintlayer_sdk::crypto::types::TxOutput>> = input_utxos
        .into_iter()
        .map(|blob| decode_optional(blob, "input utxo"))
        .collect::<Result<_, _>>()?;
    let additional_info = additional_info_from(network.into(), &additional_info)?;
    let witness = crypto::encode_witness_htlc_refund_multisig(
        sighash_type_from(sighash_type)?,
        &key,
        key_index,
        input_witness.as_ref(),
        &challenge,
        &transaction,
        &input_utxos,
        input_index as usize,
        &additional_info,
        current_block_height,
        network.into(),
    )?;
    Ok(witness.encode())
}
