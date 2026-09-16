// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Derive a key from a mnemonic, fetch spendable UTXOs from the indexer,
//! build and sign a transaction natively, and submit it to the indexer.
//!
//! Usage:
//! ```text
//! cargo run --example send-coins --features crypto,indexer -- \
//!     --mnemonic "abandon abandon ... about" \
//!     --to mtc1q... \
//!     --amount 100000000000 \
//!     --indexer http://127.0.0.1:3000 \
//!     --key-index 0 \
//!     --network testnet
//! ```
//!
//! # Fee semantics
//!
//! The example sweeps every spendable Transfer/Coin UTXO of the derived
//! address and emits a single output of `--amount` with no change output.
//! In Mintlayer the fee is implicitly `sum(inputs) - sum(outputs)` and goes
//! to the block producer, so `--amount` must be strictly less than the
//! swept total: the difference becomes the transaction fee (a zero-fee
//! transaction is rejected by the mempool). The example prints the implicit
//! fee before submitting. Production code should instead add a change
//! output sized from
//! [`estimate_transaction_size`](mintlayer_sdk::crypto::estimate_transaction_size)
//! and a fee rate fetched with the `feerate` endpoints.
//!
//! # Secret handling
//!
//! Passing the mnemonic as a command-line argument exposes it in shell
//! history and the process list. For real funds, read it from an
//! environment variable or an interactive prompt instead; the mnemonic never
//! leaves this process (only the signed transaction hex is submitted).
//! Timelocked (`LockThenTransfer`) outputs are skipped.

use std::str::FromStr;

use mintlayer_sdk::crypto::types::*;
use mintlayer_sdk::crypto::{self, Amount, Network, SigHashType, SourceId, TxAdditionalInfo};
use mintlayer_sdk::indexer::Client as IndexerClient;

struct ParsedArgs {
    mnemonic: String,
    to: String,
    amount: String,
    indexer: String,
    key_index: u32,
    network: Network,
}

fn parse_args() -> Result<ParsedArgs, String> {
    let mut parsed = ParsedArgs {
        mnemonic: String::new(),
        to: String::new(),
        amount: String::new(),
        indexer: "http://127.0.0.1:3000".to_owned(),
        key_index: 0,
        network: Network::Testnet,
    };

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--mnemonic" => parsed.mnemonic = value,
            "--to" => parsed.to = value,
            "--amount" => parsed.amount = value,
            "--indexer" => parsed.indexer = value,
            "--key-index" => {
                parsed.key_index = value.parse().map_err(|_| "invalid --key-index")?;
            }
            "--network" => {
                parsed.network = match value.as_str() {
                    "mainnet" => Network::Mainnet,
                    "testnet" => Network::Testnet,
                    "regtest" => Network::Regtest,
                    "signet" => Network::Signet,
                    _ => {
                        return Err(
                            "unknown network (use mainnet|testnet|regtest|signet)".to_owned()
                        );
                    }
                };
            }
            _ => return Err("unknown flag".to_owned()),
        }
    }

    if parsed.mnemonic.is_empty() || parsed.to.is_empty() || parsed.amount.is_empty() {
        return Err("required flags: --mnemonic, --to, --amount".to_owned());
    }
    Ok(parsed)
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args = parse_args()?;
    let network = args.network;

    let account_key = crypto::make_default_account_privkey(&args.mnemonic, network, None)
        .map_err(|error| format!("invalid mnemonic: {error}"))?;
    let spend_key = crypto::make_receiving_address(&account_key, args.key_index)
        .map_err(|error| format!("key derivation failed: {error}"))?;
    let public_key = crypto::public_key_from_private_key(&spend_key);
    let from_address = crypto::pubkey_to_pubkeyhash_address(&public_key, network);
    println!("sending from {from_address}");

    let indexer = IndexerClient::new(&args.indexer);

    let tip = indexer
        .tip()
        .await
        .map_err(|error| format!("failed to fetch chain tip: {error}"))?;
    let inclusion_height = tip.block_height.checked_add(1).ok_or("chain tip overflow")?;
    println!("predicting inclusion at height {inclusion_height}");

    let utxos = indexer
        .spendable_utxos(&from_address)
        .await
        .map_err(|error| format!("failed to fetch UTXOs: {error}"))?;
    if utxos.is_empty() {
        return Err(format!("no spendable UTXOs at {from_address}"));
    }

    let mut inputs: Vec<TxInput> = Vec::new();
    let mut input_utxos: Vec<Option<TxOutput>> = Vec::new();
    let mut total = 0u128;

    for utxo in &utxos {
        let Some(atoms) = transfer_coin_atoms(&utxo.output) else {
            println!("skipping non Transfer/Coin output");
            continue;
        };
        let source_id_bytes = hex::decode(&utxo.outpoint.source_id)
            .map_err(|error| format!("invalid source id hex: {error}"))?;
        if source_id_bytes.len() != 32 {
            return Err("source id is not 32 bytes".to_owned());
        }
        let hash = H256::from_slice(&source_id_bytes);
        let outpoint_source_id = crypto::encode_outpoint_source_id(hash, SourceId::Transaction);
        inputs.push(crypto::encode_input_for_utxo(
            outpoint_source_id,
            utxo.outpoint.index,
        ));

        let destination =
            crypto::encode_destination(&from_address, network).map_err(|e| e.to_string())?;
        input_utxos.push(Some(TxOutput::Transfer(
            OutputValue::Coin(Amount::from_atoms(atoms)),
            destination,
        )));
        total = total.checked_add(atoms).ok_or_else(|| "UTXO total overflow".to_owned())?;
    }

    let amount = u128::from_str(&args.amount).map_err(|_| "invalid --amount")?;
    if amount >= total {
        return Err(format!(
            "this example sweeps the full balance with no change output, \
             so --amount must be strictly less than the swept total \
             ({total}): the difference becomes the transaction fee and a \
             zero-fee transaction is rejected by the mempool"
        ));
    }
    let fee = total - amount;
    println!("implicit fee (paid to the block producer): {fee} atoms");

    let outputs = vec![
        crypto::encode_output_transfer(Amount::from_atoms(amount), &args.to, network)
            .map_err(|error| error.to_string())?,
    ];

    let transaction = crypto::encode_transaction(inputs.clone(), outputs, 0)
        .map_err(|error| error.to_string())?;
    let tx_id = crypto::transaction_id(&transaction);
    println!("unsigned transaction {tx_id}");

    let mut witnesses = Vec::new();
    for (index, _) in inputs.iter().enumerate() {
        let witness = crypto::encode_witness(
            SigHashType::all(),
            &spend_key,
            &from_address,
            &transaction,
            &input_utxos,
            index,
            &TxAdditionalInfo::new(),
            inclusion_height,
            network,
        )
        .map_err(|error| format!("signing failed: {error}"))?;
        witnesses.push(witness);
    }

    let signed = crypto::encode_signed_transaction(transaction, witnesses)
        .map_err(|error| error.to_string())?;
    let signed_hex = hex::encode(signed.encode());

    let submitted = indexer
        .submit_transaction(&signed_hex)
        .await
        .map_err(|error| format!("submission failed: {error}"))?;
    println!("submitted transaction {submitted}");
    Ok(())
}

fn transfer_coin_atoms(output: &serde_json::Value) -> Option<u128> {
    if output.get("type")?.as_str()? != "Transfer" {
        return None;
    }
    let value = output.get("value")?;
    if value.get("type")?.as_str()? != "Coin" {
        return None;
    }
    let atoms = value.get("amount")?.get("atoms")?.as_str()?;
    atoms.parse().ok()
}
