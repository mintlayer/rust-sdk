// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Cryptography and transaction-building tests, mirroring the go-sdk
//! `wasm/client_test.go` suite and the wasm-wrappers test vectors.

use mintlayer_sdk::crypto::types::{Encode, H256, PrivateKey, Transaction, TxOutput};
use mintlayer_sdk::crypto::{
    Amount, Network, SigHashType, SourceId, TxAdditionalInfo, data_deposit_fee,
    decode_signed_transaction_to_json, decode_transaction, decode_transaction_lenient,
    effective_pool_balance, encode_input_for_utxo, encode_lock_for_block_count,
    encode_lock_for_seconds, encode_lock_until_height, encode_lock_until_time,
    encode_outpoint_source_id, encode_output_transfer, encode_signed_transaction,
    encode_transaction, encode_witness, encode_witness_no_signature, estimate_transaction_size,
    fungible_token_issuance_fee, get_pool_id, make_default_account_privkey, make_private_key,
    make_receiving_address, nft_issuance_fee, pubkey_to_pubkeyhash_address,
    public_key_from_private_key, sign_challenge, sign_message_for_spending,
    staking_pool_spend_maturity_block_count, token_change_authority_fee, token_freeze_fee,
    token_supply_change_fee, transaction_id, verify_challenge, verify_signature_for_spending,
};

const MNEMONIC: &str = "walk exile faculty near leg neutral license matrix maple invite cupboard hat opinion excess coffee leopard latin regret document core limb crew dizzy movie";

const MAINNET_ACCOUNT_PRIVKEY: &str = "00038000002c80004d4c80000000261ee699496924546a94266597d15e3c081d8aa3b99ccefec2453418fd4e58720134b4486bdb7e70bc23933483a0cb10ac17bc104fd3c42758a4a777d71fda2d";
const MAINNET_RECEIVING_0: &str =
    "00b88adfb44da2c1fd5f12f7996bd147f45bd0b8917fa8842d4c901b965d5dad1f";
const MAINNET_RECEIVING_1: &str =
    "0022b76360c53d567d5130a7de421576c0ec1b745485c01793dbed534d489c017e";
const TESTNET_ACCOUNT_PRIVKEY: &str = "00038000002c80000001800000008fe13ec65ee469346b060206efaebafec23e2c06b5288a5e446aeab3854f13c5bfbc80385eda560749b9601f5f5bd92ed56caf243ecb4f57c97fcb3bf13ad0e6";
const TESTNET_RECEIVING_0: &str =
    "00f42c0e96b4ee90ed64c57948216d7a4773d59969a080ac45f639ee624481590d";
const TESTNET_RECEIVING_1: &str =
    "00114be4d2511116792ca87760973ba299ad94a5a8ddb3ab491ecfa7e62d613745";

const EXPECTED_TX_ID: &str = "35a7938c2a2aad5ae324e7d0536de245bf9e439169aa3c16f1492be117e5d0e0";
const TX_HEX: &str = "0100040000ff5d9a94390ee97208d31aa5c3b5ddbd8df9d308069df2ebf5283f7ce3e4261401000000080340f9924e4da0af7dc8c5be71a9c9e05962c7bf4ef96127fde7a7b4e1469e48620f0080e03779c31102000365807e3b4147cb978b78715e60606092f89dc769586e98456850bd3b449c87b400203015e9ef9fc142569e0f966bc0188464fa712a841e14002e0fe952a076a26c01e539c5f0ceba927ab8f8f55f274af739ce4eef3700000b00204aa9d10100000b409e4c355d010199e4ec3a5b176140ef9cd58c7d3579fdb0ecb21a";
const TX_SIGNED_HEX: &str = "0100040000ff5d9a94390ee97208d31aa5c3b5ddbd8df9d308069df2ebf5283f7ce3e4261401000000080340f9924e4da0af7dc8c5be71a9c9e05962c7bf4ef96127fde7a7b4e1469e48620f0080e03779c31102000365807e3b4147cb978b78715e60606092f89dc769586e98456850bd3b449c87b400203015e9ef9fc142569e0f966bc0188464fa712a841e14002e0fe952a076a26c01e539c5f0ceba927ab8f8f55f274af739ce4eef3700000b00204aa9d10100000b409e4c355d010199e4ec3a5b176140ef9cd58c7d3579fdb0ecb21a0401018d010002eddd003bfb6333123e682abe6923da1d38faa4f0e0d9e2ee42d5aa46c152a34800a749a30c8c9c33696ce407fc145ebc9824e17b778d0d9ccc8129be52f37b74160e60f6689ac2f481071e1a63d9cf0f6eab84c2703b5e9f229cd8188ce092edd4";

fn fake_utxo_transaction() -> (PrivateKey, String, Transaction, TxOutput) {
    let private_key = make_private_key();
    let public_key = public_key_from_private_key(&private_key);
    let address = pubkey_to_pubkeyhash_address(&public_key, Network::Mainnet);
    let output = encode_output_transfer(
        Amount::from_atoms(100_000_000_000),
        &address,
        Network::Mainnet,
    )
    .unwrap();
    let source_id = encode_outpoint_source_id(H256::from_slice(&[0u8; 32]), SourceId::Transaction);
    let input = encode_input_for_utxo(source_id, 0);
    let transaction = encode_transaction(vec![input], vec![output.clone()], 0).unwrap();
    (private_key, address, transaction, output)
}

#[test]
fn make_private_key_is_random() {
    let key_1 = make_private_key();
    let key_2 = make_private_key();

    let encoded_1 = key_1.encode();
    let encoded_2 = key_2.encode();

    assert_eq!(encoded_1.len(), 33);
    assert_eq!(encoded_2.len(), 33);
    assert_ne!(encoded_1, encoded_2, "two consecutive keys are identical");
}

#[test]
fn default_account_privkey_legacy_vectors() {
    for passphrase in [None, Some("")] {
        let mainnet_account =
            make_default_account_privkey(MNEMONIC, Network::Mainnet, passphrase).unwrap();
        let testnet_account =
            make_default_account_privkey(MNEMONIC, Network::Testnet, passphrase).unwrap();

        assert_eq!(
            hex::encode(mainnet_account.encode()),
            MAINNET_ACCOUNT_PRIVKEY,
            "mainnet account key, passphrase {passphrase:?}"
        );
        assert_eq!(
            hex::encode(testnet_account.encode()),
            TESTNET_ACCOUNT_PRIVKEY,
            "testnet account key, passphrase {passphrase:?}"
        );
    }

    let mainnet_account = make_default_account_privkey(MNEMONIC, Network::Mainnet, None).unwrap();
    let testnet_account = make_default_account_privkey(MNEMONIC, Network::Testnet, None).unwrap();

    for (index, expected) in [(0u32, MAINNET_RECEIVING_0), (1, MAINNET_RECEIVING_1)] {
        let receiving = make_receiving_address(&mainnet_account, index).unwrap();
        assert_eq!(
            hex::encode(receiving.encode()),
            expected,
            "mainnet receiving {index}"
        );
    }
    for (index, expected) in [(0u32, TESTNET_RECEIVING_0), (1, TESTNET_RECEIVING_1)] {
        let receiving = make_receiving_address(&testnet_account, index).unwrap();
        assert_eq!(
            hex::encode(receiving.encode()),
            expected,
            "testnet receiving {index}"
        );
    }
}

#[test]
fn different_passphrases_produce_different_keys() {
    let passphrases = [None, Some("passphrase-1"), Some("passphrase-2")];

    let mut account_keys = Vec::new();
    let mut receiving_addresses = Vec::new();
    for passphrase in passphrases {
        let account = make_default_account_privkey(MNEMONIC, Network::Mainnet, passphrase).unwrap();
        account_keys.push(hex::encode(account.encode()));

        let receiving = make_receiving_address(&account, 0).unwrap();
        let public_key = public_key_from_private_key(&receiving);
        receiving_addresses.push(pubkey_to_pubkeyhash_address(&public_key, Network::Mainnet));
    }

    for i in 0..passphrases.len() {
        for j in (i + 1)..passphrases.len() {
            assert_ne!(
                account_keys[i], account_keys[j],
                "account keys {i} and {j} match"
            );
            assert_ne!(
                receiving_addresses[i], receiving_addresses[j],
                "receiving addresses {i} and {j} match"
            );
        }
    }
}

#[test]
fn transaction_get_id_vector() {
    let tx_bytes = hex::decode(TX_HEX).unwrap();
    let signed_bytes = hex::decode(TX_SIGNED_HEX).unwrap();

    let transaction = decode_transaction(&tx_bytes).unwrap();
    assert_eq!(transaction_id(&transaction), EXPECTED_TX_ID);

    let lenient = decode_transaction_lenient(&signed_bytes).unwrap();
    assert_eq!(transaction_id(&lenient), EXPECTED_TX_ID);

    assert!(
        decode_transaction(&signed_bytes).is_err(),
        "strict decoding must reject signed transaction bytes"
    );
}

#[test]
fn encode_transaction_roundtrip() {
    let (_, _, transaction, _) = fake_utxo_transaction();

    let tx_id = transaction_id(&transaction);
    assert_eq!(tx_id.len(), 64);
    assert!(
        tx_id.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "transaction id must be lowercase hex, got {tx_id}"
    );
}

#[test]
fn timelocks_encode() {
    let locks = [
        encode_lock_for_block_count(100),
        encode_lock_for_seconds(86_400),
        encode_lock_until_time(1_700_000_000),
        encode_lock_until_height(500_000),
    ];
    for lock in locks {
        assert!(!lock.encode().is_empty());
    }
}

#[test]
fn fees_are_non_zero() {
    let height = 500_000u64;
    let network = Network::Mainnet;

    assert!(fungible_token_issuance_fee(height, network).into_atoms() > 0);
    assert!(nft_issuance_fee(height, network).into_atoms() > 0);
    assert!(data_deposit_fee(height, network).into_atoms() > 0);
    assert!(token_supply_change_fee(height, network).into_atoms() > 0);
    assert!(token_freeze_fee(height, network).into_atoms() > 0);
    assert!(token_change_authority_fee(height, network).into_atoms() > 0);
}

#[test]
fn staking_helpers() {
    let maturity = staking_pool_spend_maturity_block_count(500_000, Network::Mainnet);
    assert!(maturity > 0);

    let pledge = Amount::from_atoms(1_000_000_000_000_000);
    let pool_balance = Amount::from_atoms(10_000_000_000_000_000);
    let effective = effective_pool_balance(Network::Mainnet, pledge, pool_balance).unwrap();
    assert!(effective.into_atoms() > 0);
}

#[test]
fn sign_challenge_roundtrip() {
    let key = make_private_key();
    let public_key = public_key_from_private_key(&key);
    let address = pubkey_to_pubkeyhash_address(&public_key, Network::Mainnet);

    let message = b"hello mintlayer";
    let signature = sign_challenge(&key, message).unwrap();

    let verified = verify_challenge(&address, Network::Mainnet, &signature, message).unwrap();
    assert!(verified);

    let tampered_message = b"hello mintlayex";
    let result = verify_challenge(&address, Network::Mainnet, &signature, tampered_message);
    assert!(matches!(result, Err(_) | Ok(false)));
}

#[test]
fn sign_message_for_spending_roundtrip() {
    let key = make_private_key();
    let public_key = public_key_from_private_key(&key);

    let message = b"spending message test";
    let signature = sign_message_for_spending(&key, message).unwrap();

    assert!(verify_signature_for_spending(
        &public_key,
        &signature,
        message
    ));

    let tampered_message = b"spending message tesu";
    assert!(!verify_signature_for_spending(
        &public_key,
        &signature,
        tampered_message
    ));

    let wrong_key = make_private_key();
    let wrong_public_key = public_key_from_private_key(&wrong_key);
    assert!(!verify_signature_for_spending(
        &wrong_public_key,
        &signature,
        message
    ));
}

#[test]
fn encode_witness_no_signature_encodes() {
    let witness = encode_witness_no_signature();
    assert!(!witness.encode().is_empty());
}

#[test]
fn witness_roundtrip_on_fake_tx() {
    let (private_key, address, transaction, output) = fake_utxo_transaction();

    let witness = encode_witness(
        SigHashType::all(),
        &private_key,
        &address,
        &transaction,
        &[Some(output)],
        0,
        &TxAdditionalInfo::new(),
        0,
        Network::Mainnet,
    )
    .unwrap();

    let signed = encode_signed_transaction(transaction, vec![witness]).unwrap();
    let json = decode_signed_transaction_to_json(&signed.encode(), Network::Mainnet).unwrap();

    let decoded_transaction = json
        .get("transaction")
        .expect("signed transaction JSON must contain the transaction");
    let decoded_v1 = decoded_transaction
        .get("V1")
        .expect("transaction JSON must contain the V1 variant");
    assert!(
        decoded_v1.get("inputs").and_then(|value| value.as_array()).is_some(),
        "decoded JSON must contain transaction inputs"
    );
    assert!(
        decoded_v1.get("outputs").and_then(|value| value.as_array()).is_some(),
        "decoded JSON must contain transaction outputs"
    );
}

#[test]
fn estimate_transaction_size_length_mismatch() {
    let result = estimate_transaction_size(&[], &["mtc1qfoo"], &[], Network::Mainnet);
    assert!(result.is_err());
}

#[test]
fn get_pool_id_from_inputs() {
    let source_id = encode_outpoint_source_id(H256::from_slice(&[0u8; 32]), SourceId::Transaction);
    let fake_input = encode_input_for_utxo(source_id, 0);

    let pool_id = get_pool_id(&[fake_input], Network::Mainnet).unwrap();
    assert!(pool_id.starts_with("mpool"), "unexpected pool id {pool_id}");
}
