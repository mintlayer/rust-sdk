// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Transaction assembly, decoding and inspection.

use ml_common as common;
use ml_serialization as serialization;

use common::chain::signature::inputsig::InputWitness;
use common::chain::{
    Destination, OutPointSourceId, SignedTransaction, Transaction, TxInput, TxOutput,
    partially_signed_transaction::{
        PartiallySignedTransaction, PartiallySignedTransactionConsistencyCheck,
        make_sighash_input_commitments_at_height,
    },
};
use common::primitives::{BlockHeight, H256, Idable};
use common::size_estimation::{
    input_signature_size_from_destination, outputs_encoded_size,
    tx_size_with_num_inputs_and_outputs,
};
use serialization::json_encoded::JsonEncoded;
use serialization::{Decode, DecodeAll, Encode};
use std::str::FromStr;

use super::{Error, Network, SourceId, TxAdditionalInfo, chain_config, parse_addressable};

/// Builds a [`OutPointSourceId`] from raw hash bytes.
///
/// Given the bytes of a transaction id or block id and where the output
/// comes from, this produces the source id used by UTXO inputs.
pub fn encode_outpoint_source_id(id: H256, source: SourceId) -> OutPointSourceId {
    match source {
        SourceId::Transaction => OutPointSourceId::Transaction(id.into()),
        SourceId::BlockReward => OutPointSourceId::BlockReward(id.into()),
    }
}

/// Builds a transaction from inputs and outputs.
///
/// `flags` are the transaction flags (currently always zero).
pub fn encode_transaction(
    inputs: Vec<TxInput>,
    outputs: Vec<TxOutput>,
    flags: u64,
) -> Result<Transaction, Error> {
    Transaction::new(u128::from(flags), inputs, outputs)
        .map_err(|error| Error::TransactionCreation(error.to_string()))
}

/// Returns the lowercase hex transaction id of a transaction.
pub fn transaction_id(transaction: &Transaction) -> String {
    format!("{:x}", transaction.get_id())
}

/// Decodes a transaction, requiring the bytes to contain exactly one
/// transaction.
pub fn decode_transaction(bytes: &[u8]) -> Result<Transaction, Error> {
    Transaction::decode_all(&mut &bytes[..]).map_err(Error::from)
}

/// Decodes a transaction, ignoring any trailing bytes.
///
/// This tolerates signed transaction bytes, whose witness list is appended
/// after the transaction.
pub fn decode_transaction_lenient(bytes: &[u8]) -> Result<Transaction, Error> {
    Transaction::decode(&mut &bytes[..]).map_err(Error::from)
}

/// Assembles a signed transaction from a transaction and its witnesses.
pub fn encode_signed_transaction(
    transaction: Transaction,
    signatures: Vec<InputWitness>,
) -> Result<SignedTransaction, Error> {
    SignedTransaction::new(transaction, signatures)
        .map_err(|error| Error::SignedTransactionCreation(error.to_string()))
}

/// Builds a partially signed transaction.
///
/// Every per-input list must have exactly one entry per transaction input;
/// `None` marks an absent entry. `additional_info` must cover every
/// `ProduceBlockFromStake`, `FillOrder` and `ConcludeOrder` input.
#[allow(clippy::too_many_arguments)]
pub fn encode_partially_signed_transaction(
    transaction: Transaction,
    signatures: Vec<Option<InputWitness>>,
    input_utxos: Vec<Option<TxOutput>>,
    input_destinations: Vec<Option<Destination>>,
    htlc_secrets: Vec<Option<common::chain::htlc::HtlcSecret>>,
    additional_info: TxAdditionalInfo,
    _network: Network,
) -> Result<PartiallySignedTransaction, Error> {
    PartiallySignedTransaction::new(
        transaction,
        signatures,
        input_utxos,
        input_destinations,
        Some(htlc_secrets),
        additional_info,
        PartiallySignedTransactionConsistencyCheck::WithAdditionalInfo,
    )
    .map_err(|error| Error::PartiallySignedTransactionCreation(error.to_string()))
}

/// Decodes a transaction into a JSON representation with bech32 addresses.
pub fn decode_signed_transaction_to_json(
    transaction: &[u8],
    network: Network,
) -> Result<serde_json::Value, Error> {
    let tx = SignedTransaction::decode_all(&mut &transaction[..]).map_err(Error::from)?;
    json_with_addresses(&tx, network)
}

/// Decodes a partially signed transaction into JSON with bech32 addresses.
pub fn decode_partially_signed_transaction_to_json(
    transaction: &[u8],
    network: Network,
) -> Result<serde_json::Value, Error> {
    let ptx = PartiallySignedTransaction::decode_all(&mut &transaction[..]).map_err(Error::from)?;
    json_with_addresses(&ptx, network)
}

/// Estimates the byte size of a signed transaction.
///
/// `input_destinations` holds the spending address of each input; script
/// hash and multisig destinations are not supported. HTLC inputs are assumed
/// to be regular UTXO inputs.
pub fn estimate_transaction_size(
    inputs: &[TxInput],
    input_destinations: &[&str],
    outputs: &[TxOutput],
    network: Network,
) -> Result<usize, Error> {
    let mut total_size =
        tx_size_with_num_inputs_and_outputs(outputs.len(), input_destinations.len())
            .map_err(|error| Error::SizeEstimation(error.to_string()))?
            + outputs_encoded_size(outputs)
            + inputs.encode().len();

    for destination in input_destinations {
        let destination = parse_addressable(network, destination)?;
        let signature_size = input_signature_size_from_destination(&destination, None, None)
            .map_err(|error| Error::SizeEstimation(error.to_string()))?;
        total_size += signature_size;
    }

    Ok(total_size)
}

/// Extracts the HTLC pre-image secret from a signed HTLC spend.
pub fn extract_htlc_secret(
    signed_tx: &SignedTransaction,
    htlc_outpoint_source_id: OutPointSourceId,
    htlc_output_index: u32,
) -> Result<common::chain::htlc::HtlcSecret, Error> {
    let htlc_utxo_outpoint =
        common::chain::UtxoOutPoint::new(htlc_outpoint_source_id, htlc_output_index);

    let htlc_position = signed_tx
        .transaction()
        .inputs()
        .iter()
        .position(
            |input| matches!(input, TxInput::Utxo(outpoint) if *outpoint == htlc_utxo_outpoint),
        )
        .ok_or(Error::NoInputOutpointFound)?;

    let witness = signed_tx.signatures().get(htlc_position).ok_or(Error::InvalidWitnessCount)?;

    let (htlc_spend, _) = super::signing::extract_htlc_spend(witness)?;
    match htlc_spend {
        common::chain::signature::inputsig::authorize_hashed_timelock_contract_spend::AuthorizedHashedTimelockContractSpend::Spend(secret, _) => {
            Ok(secret)
        }
        common::chain::signature::inputsig::authorize_hashed_timelock_contract_spend::AuthorizedHashedTimelockContractSpend::Refund(_) => {
            Err(Error::UnexpectedHtlcSpendType)
        }
    }
}

pub(crate) fn make_input_commitments<'a>(
    transaction: &Transaction,
    input_utxos: &'a [Option<TxOutput>],
    additional_info: &TxAdditionalInfo,
    network: Network,
    current_block_height: u64,
) -> Result<
    Vec<
        common::chain::transaction::signature::sighash::input_commitments::SighashInputCommitment<
            'a,
        >,
    >,
    Error,
> {
    make_sighash_input_commitments_at_height(
        transaction.inputs(),
        input_utxos,
        additional_info,
        chain_config(network),
        BlockHeight::new(current_block_height),
    )
    .map_err(|error| Error::Sighash(error.to_string()))
}

pub(crate) fn json_with_addresses<T: serde::Serialize>(
    object: &T,
    network: Network,
) -> Result<serde_json::Value, Error> {
    let json = JsonEncoded::new(object).to_string();
    let json = common::address::dehexify::dehexify_all_addresses(chain_config(network), &json);
    serde_json::from_str(&json).map_err(|error| Error::Json(error.to_string()))
}

/// Parses a hex-encoded hash into an [`H256`].
pub(crate) fn h256_from_str(hex: &str) -> Result<H256, Error> {
    H256::from_str(hex).map_err(|_| Error::InvalidTransactionId(hex.to_owned()))
}
