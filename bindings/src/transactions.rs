// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Transaction assembly, ids, decoding and size estimation.

use mintlayer_sdk::crypto::types::{
    Decode, DecodeAll, Encode, H256, HtlcSecret, InputWitness, OutPointSourceId, SignedTransaction,
    Transaction, TxOutput,
};

use super::error::{BindingsError, decoding};
use super::network::Network;
use super::types::{
    SourceId, TxAdditionalInfo, additional_info_from, decode_list, decode_optional,
};
use mintlayer_sdk::crypto;

/// Encodes an outpoint source id (the first half of a UTXO reference).
/// `id` is the 32-byte transaction id or block reward id.
#[uniffi::export]
pub fn encode_outpoint_source_id(id: Vec<u8>, source: SourceId) -> Result<Vec<u8>, BindingsError> {
    if id.len() != 32 {
        return Err(BindingsError::Decoding {
            detail: "outpoint source id must be exactly 32 bytes".to_owned(),
        });
    }
    let outpoint = crypto::encode_outpoint_source_id(H256::from_slice(&id), source.into());
    Ok(outpoint.encode())
}

/// Assembles an unsigned transaction from encoded inputs and outputs.
/// `flags` carries the transaction flags bitfield.
#[uniffi::export]
pub fn encode_transaction(
    inputs: Vec<Vec<u8>>,
    outputs: Vec<Vec<u8>>,
    flags: u64,
) -> Result<Vec<u8>, BindingsError> {
    let inputs = decode_list(inputs, "input")?;
    let outputs = decode_list(outputs, "output")?;
    let transaction = crypto::encode_transaction(inputs, outputs, flags)?;
    Ok(transaction.encode())
}

/// Returns the hex-encoded id of a transaction.
///
/// With `strictByteSize` the bytes must exactly match one `Transaction`
/// object; otherwise trailing bytes (e.g. the signature list of a
/// `SignedTransaction`) are tolerated and ignored.
#[uniffi::export]
pub fn get_transaction_id(
    transaction: Vec<u8>,
    strict_byte_size: bool,
) -> Result<String, BindingsError> {
    let tx = if strict_byte_size {
        crypto::decode_transaction(&transaction)?
    } else {
        crypto::decode_transaction_lenient(&transaction)?
    };
    Ok(crypto::transaction_id(&tx))
}

/// Decodes an unsigned transaction into a JSON representation.
///
/// The bytes must exactly match one `Transaction` object; trailing bytes
/// (e.g. the witness list of a signed transaction) are rejected — decode
/// with [`decodeSignedTransactionToJson`](crate::decode_signed_transaction_to_json)
/// for signed transactions.
#[uniffi::export]
pub fn decode_transaction_to_json(
    transaction: Vec<u8>,
    network: Network,
) -> Result<String, BindingsError> {
    let json = crypto::decode_unsigned_transaction_to_json(&transaction, network.into())?;
    Ok(json.to_string())
}

/// Decodes a signed transaction into a JSON representation.
#[uniffi::export]
pub fn decode_signed_transaction_to_json(
    transaction: Vec<u8>,
    network: Network,
) -> Result<String, BindingsError> {
    let json = crypto::decode_signed_transaction_to_json(&transaction, network.into())?;
    Ok(json.to_string())
}

/// Decodes a partially signed transaction into a JSON representation.
#[uniffi::export]
pub fn decode_partially_signed_transaction_to_json(
    transaction: Vec<u8>,
    network: Network,
) -> Result<String, BindingsError> {
    let json = crypto::decode_partially_signed_transaction_to_json(&transaction, network.into())?;
    Ok(json.to_string())
}

/// Assembles a signed transaction from a transaction and its witness list.
#[uniffi::export]
pub fn encode_signed_transaction(
    transaction: Vec<u8>,
    signatures: Vec<Vec<u8>>,
) -> Result<Vec<u8>, BindingsError> {
    let transaction = Transaction::decode_all(&mut &transaction[..]).map_err(decoding)?;
    let signatures = decode_list(signatures, "signature")?;
    let signed = crypto::encode_signed_transaction(transaction, signatures)?;
    Ok(signed.encode())
}

/// Assembles a partially signed transaction.
///
/// `signatures`, `inputUtxos`, `inputDestinations` and `htlcSecrets` each
/// have one optional entry per transaction input (same position): the
/// witness for the input, its UTXO, the address that must sign it and —
/// for HTLC spends — the revealed secret.
#[uniffi::export]
#[allow(clippy::too_many_arguments)]
pub fn encode_partially_signed_transaction(
    transaction: Vec<u8>,
    signatures: Vec<Option<Vec<u8>>>,
    input_utxos: Vec<Option<Vec<u8>>>,
    input_destinations: Vec<Option<String>>,
    htlc_secrets: Vec<Option<Vec<u8>>>,
    additional_info: TxAdditionalInfo,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let transaction = Transaction::decode_all(&mut &transaction[..]).map_err(decoding)?;
    let signatures: Vec<Option<InputWitness>> = signatures
        .into_iter()
        .map(|blob| decode_optional(blob, "signature"))
        .collect::<Result<_, _>>()?;
    let input_utxos: Vec<Option<TxOutput>> = input_utxos
        .into_iter()
        .map(|blob| decode_optional(blob, "input utxo"))
        .collect::<Result<_, _>>()?;
    let input_destinations = input_destinations
        .into_iter()
        .map(|address| {
            address
                .map(|address| {
                    crypto::encode_destination(&address, network.into())
                        .map_err(BindingsError::from)
                })
                .transpose()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let htlc_secrets: Vec<Option<HtlcSecret>> = htlc_secrets
        .into_iter()
        .map(|blob| decode_optional(blob, "htlc secret"))
        .collect::<Result<_, _>>()?;
    let additional_info = additional_info_from(network.into(), &additional_info)?;
    let partial = crypto::encode_partially_signed_transaction(
        transaction,
        signatures,
        input_utxos,
        input_destinations,
        htlc_secrets,
        additional_info,
        network.into(),
    )?;
    Ok(partial.encode())
}

/// Estimates the encoded size of a transaction, in bytes.
///
/// `inputDestinations` holds the signing address of each input.
#[uniffi::export]
pub fn estimate_transaction_size(
    inputs: Vec<Vec<u8>>,
    input_destinations: Vec<String>,
    outputs: Vec<Vec<u8>>,
    network: Network,
) -> Result<u64, BindingsError> {
    let inputs = decode_list(inputs, "input")?;
    let outputs = decode_list(outputs, "output")?;
    let destinations: Vec<&str> = input_destinations.iter().map(String::as_str).collect();
    let size = crypto::estimate_transaction_size(&inputs, &destinations, &outputs, network.into())?;
    u64::try_from(size).map_err(|_| BindingsError::SizeEstimation {
        detail: "size exceeds u64".to_owned(),
    })
}

/// Extracts the revealed HTLC pre-image secret from a signed HTLC spend.
///
/// `htlcOutpointSourceId` is the blob produced by
/// [`encodeOutpointSourceId`](crate::encode_outpoint_source_id) for the
/// HTLC output the transaction is spending.
#[uniffi::export]
pub fn extract_htlc_secret(
    signed_tx: Vec<u8>,
    strict_byte_size: bool,
    htlc_outpoint_source_id: Vec<u8>,
    htlc_output_index: u32,
) -> Result<Vec<u8>, BindingsError> {
    let signed_tx = if strict_byte_size {
        SignedTransaction::decode_all(&mut &signed_tx[..]).map_err(decoding)?
    } else {
        SignedTransaction::decode(&mut &signed_tx[..]).map_err(decoding)?
    };
    let outpoint_source_id =
        OutPointSourceId::decode_all(&mut &htlc_outpoint_source_id[..]).map_err(decoding)?;
    let secret = crypto::extract_htlc_secret(&signed_tx, outpoint_source_id, htlc_output_index)?;
    Ok(secret.consume().to_vec())
}
