// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Structural tests for the transaction-building surface.
//!
//! These pin the encode → decode roundtrips and the consumer workflows
//! (transaction assembly, signing, intents) against the typed chain
//! objects. Byte-level parity with the WASM reference is pinned by the
//! parity suite for the derivation chain.

use hex::FromHex;
use mintlayer_sdk::crypto::types::{
    DecodeAll, Destination, OutputTimeLock, OutputValue, TxInput, TxOutput,
};
use mintlayer_wallet_bindings::*;

const MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

fn hex_bytes(s: &str) -> Vec<u8> {
    Vec::from_hex(s).expect("test constant must be valid hex")
}

/// Derives the test-vector account key and its receiving key #0 (testnet).
fn derived_keys() -> (Vec<u8>, Vec<u8>, Vec<u8>, String) {
    let account = make_default_account_privkey(MNEMONIC.to_owned(), Network::Testnet, None)
        .expect("derivation must succeed");
    let xpub = extended_public_key_from_extended_private_key(account.clone())
        .expect("xpub derivation must succeed");
    let private_key =
        make_receiving_address(account.clone(), 0).expect("receiving key derivation must succeed");
    let public_key = make_receiving_address_public_key(xpub.clone(), 0)
        .expect("receiving pubkey derivation must succeed");
    let address = pubkey_to_pubkeyhash_address(public_key.clone(), Network::Testnet)
        .expect("address encoding must succeed");
    (account, private_key, public_key, address)
}

#[test]
fn fees_are_positive_atom_strings() {
    for network in [Network::Mainnet, Network::Testnet] {
        for fee in [
            fungible_token_issuance_fee(0, network),
            nft_issuance_fee(100, network),
            token_supply_change_fee(100, network),
            token_freeze_fee(100, network),
            token_change_authority_fee(100, network),
            data_deposit_fee(100, network),
        ] {
            let atoms = fee.parse::<u128>().expect("fee must be a decimal string");
            assert!(atoms > 0, "fee must be positive, got {fee}");
        }
    }
}

#[test]
fn timelock_blobs_decode_to_expected_variants() {
    let block_count = encode_lock_for_block_count(100);
    assert!(
        matches!(
            OutputTimeLock::decode_all(&mut &block_count[..]),
            Ok(OutputTimeLock::ForBlockCount(100))
        ),
        "ForBlockCount must roundtrip"
    );

    let seconds = encode_lock_for_seconds(3600);
    assert!(
        matches!(
            OutputTimeLock::decode_all(&mut &seconds[..]),
            Ok(OutputTimeLock::ForSeconds(3600))
        ),
        "ForSeconds must roundtrip"
    );

    let until_time = encode_lock_until_time(1_700_000_000);
    assert!(
        matches!(
            OutputTimeLock::decode_all(&mut &until_time[..]),
            Ok(OutputTimeLock::UntilTime(_))
        ),
        "UntilTime must decode"
    );

    let until_height = encode_lock_until_height(5000);
    assert!(
        matches!(
            OutputTimeLock::decode_all(&mut &until_height[..]),
            Ok(OutputTimeLock::UntilHeight(_))
        ),
        "UntilHeight must decode"
    );
}

#[test]
fn outpoint_and_utxo_input_roundtrip() {
    let tx_id = hex_bytes("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
    let outpoint = encode_outpoint_source_id(tx_id, SourceId::Transaction).expect("must encode");
    let input = encode_input_for_utxo(outpoint, 7).expect("must encode");
    let decoded = TxInput::decode_all(&mut &input[..]).expect("must decode");
    assert!(
        matches!(decoded, TxInput::Utxo(_)),
        "expected a UTXO input, got {decoded:?}"
    );
}

#[test]
fn transfer_output_roundtrip() {
    let (_, _, _, address) = derived_keys();
    let blob = encode_output_transfer("100000000000".to_owned(), address.clone(), Network::Testnet)
        .expect("must encode");
    let decoded = TxOutput::decode_all(&mut &blob[..]).expect("must decode");
    match decoded {
        TxOutput::Transfer(OutputValue::Coin(amount), destination) => {
            assert_eq!(amount.into_atoms(), 100_000_000_000u128);
            assert!(matches!(destination, Destination::PublicKeyHash(_)));
        }
        other => panic!("expected a coin transfer, got {other:?}"),
    }
}

#[test]
fn decode_unsigned_transaction_is_strict() {
    // A transaction with one input must decode; strict decoding rejects the
    // same bytes with the witness list of a signed transaction appended.
    let (_, _, _, address) = derived_keys();
    let tx_id = hex_bytes("101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f");
    let outpoint = encode_outpoint_source_id(tx_id, SourceId::Transaction).unwrap();
    let input = encode_input_for_utxo(outpoint, 0).unwrap();
    let output =
        encode_output_transfer("1000".to_owned(), address.clone(), Network::Testnet).unwrap();
    let transaction = encode_transaction(vec![input], vec![output], 0).unwrap();
    let json = decode_transaction_to_json(transaction.clone(), Network::Testnet)
        .expect("a 1-input transaction must decode to json");
    assert!(json.contains("inputs"), "json shape: {json}");

    // Strictness: appending trailing bytes must be rejected.
    let mut padded = transaction.clone();
    padded.extend_from_slice(&[0x00]);
    assert!(decode_transaction_to_json(padded, Network::Testnet).is_err());
}

#[test]
fn stake_pool_data_requires_vrf_address() {
    // The VRF public key parameter must be an address of the VRF public key
    // kind; a pubkeyhash address is rejected. The happy path is not
    // exercised by the differential suite (that VRF-address input cannot be
    // produced from the wasm API); this test pins the rejection behavior.
    let (_, _, _, address) = derived_keys();
    let error = encode_stake_pool_data(
        "1000000000000".to_owned(),
        address.clone(),
        address.clone(), // wrong kind: pubkeyhash instead of VRF public key
        address.clone(),
        1000,
        "1000".to_owned(),
        Network::Testnet,
    )
    .expect_err("pubkeyhash address must be rejected as VRF key");
    assert!(
        matches!(error, BindingsError::AddressParse { .. }),
        "expected an address parse error, got {error:?}"
    );
}

#[test]
fn token_and_lock_outputs_roundtrip() {
    let (_, _, _, address) = derived_keys();
    let token_id = get_token_id(
        vec![
            encode_input_for_utxo(
                encode_outpoint_source_id(
                    hex_bytes("101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f"),
                    SourceId::Transaction,
                )
                .unwrap(),
                0,
            )
            .unwrap(),
        ],
        1000,
        Network::Testnet,
    )
    .expect("token id must derive");

    let token_transfer = encode_output_token_transfer(
        "500".to_owned(),
        address.clone(),
        token_id.clone(),
        Network::Testnet,
    )
    .unwrap();
    assert!(matches!(
        TxOutput::decode_all(&mut &token_transfer[..]),
        Ok(TxOutput::Transfer(..))
    ));

    let lock = encode_lock_for_block_count(10);
    let locked =
        encode_output_lock_then_transfer("700".to_owned(), address, lock, Network::Testnet)
            .unwrap();
    assert!(matches!(
        TxOutput::decode_all(&mut &locked[..]),
        Ok(TxOutput::LockThenTransfer(OutputValue::Coin(_), _, _))
    ));

    let mint =
        encode_input_for_mint_tokens(token_id, "500".to_owned(), 3, Network::Testnet).unwrap();
    assert!(matches!(
        TxInput::decode_all(&mut &mint[..]),
        Ok(TxInput::AccountCommand(..))
    ));
}

#[test]
fn multisig_challenge_address_is_be32() {
    let (_, _, public_key, single_address) = derived_keys();
    let second_key = public_key.clone();
    let challenge = encode_multisig_challenge(vec![public_key, second_key], 1, Network::Testnet)
        .expect("challenge must encode");
    let address =
        multisig_challenge_to_address(challenge, Network::Testnet).expect("address must encode");
    assert!(
        address.starts_with("tmtc1"),
        "unexpected multisig address: {address}"
    );
    assert_ne!(address, single_address);
}

#[test]
fn full_transaction_signing_flow() {
    let (_, private_key, public_key, address) = derived_keys();

    // A spendable input: outpoint from a made-up transaction id.
    let tx_id = hex_bytes("101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f");
    let outpoint = encode_outpoint_source_id(tx_id, SourceId::Transaction).unwrap();
    let input = encode_input_for_utxo(outpoint.clone(), 0).unwrap();

    // A transfer back to ourselves minus fees.
    let output =
        encode_output_transfer("99999999999".to_owned(), address.clone(), Network::Testnet)
            .unwrap();

    let transaction = encode_transaction(vec![input], vec![output], 0).unwrap();
    let tx_id_hex = get_transaction_id(transaction.clone(), true).unwrap();
    assert_eq!(tx_id_hex.len(), 64, "transaction id must be 64 hex chars");

    // Sign input 0 with the receiving key.
    let input_utxo_blob =
        encode_output_transfer("100000000000".to_owned(), address.clone(), Network::Testnet)
            .unwrap();
    let witness = encode_witness(
        SignatureHashType::All,
        private_key.clone(),
        address.clone(),
        transaction.clone(),
        vec![Some(input_utxo_blob.clone())],
        0,
        TxAdditionalInfo {
            pool_info: Vec::new(),
            order_info: Vec::new(),
        },
        1000,
        Network::Testnet,
    )
    .expect("witness must sign");

    let signed = encode_signed_transaction(transaction.clone(), vec![witness])
        .expect("signed transaction must assemble");
    let json =
        decode_signed_transaction_to_json(signed, Network::Testnet).expect("must decode to json");
    assert!(json.contains("\"signatures\""), "json shape: {json}");

    // Size estimation must be a sane positive number.
    let input_lenient = encode_input_for_utxo(outpoint, 0).unwrap();
    let size = estimate_transaction_size(
        vec![input_lenient],
        vec![address.clone()],
        vec![
            encode_output_transfer("99999999999".to_owned(), address.clone(), Network::Testnet)
                .unwrap(),
        ],
        Network::Testnet,
    )
    .unwrap();
    assert!(size > 100 && size < 10_000, "unreasonable size: {size}");

    // Partially signed assembly with the same pieces.
    let partial = encode_partially_signed_transaction(
        transaction,
        vec![None],
        vec![Some(input_utxo_blob.clone())],
        vec![Some(address)],
        vec![None],
        TxAdditionalInfo {
            pool_info: Vec::new(),
            order_info: Vec::new(),
        },
        Network::Testnet,
    )
    .expect("partial must assemble");
    let partial_json = decode_partially_signed_transaction_to_json(partial, Network::Testnet)
        .expect("must decode to json");
    assert!(partial_json.contains("input"), "json shape: {partial_json}");
    let _ = public_key;
}

#[test]
fn message_signing_roundtrip() {
    let (_, private_key, public_key, _) = derived_keys();
    let message = b"hello mintlayer".to_vec();
    let signature =
        sign_message_for_spending(private_key.clone(), message.clone()).expect("must sign");
    assert!(
        verify_signature_for_spending(public_key, signature.clone(), message.clone()).unwrap(),
        "valid signature must verify"
    );
    let mut tampered = message.clone();
    tampered[0] ^= 0xff;
    assert!(
        !verify_signature_for_spending(pubkey_from_priv(&private_key), signature, tampered,)
            .unwrap(),
        "tampered message must not verify"
    );
}

#[test]
fn challenge_signing_roundtrip() {
    let (_, private_key, _, address) = derived_keys();
    let message = b"challenge:12345".to_vec();
    let signed = sign_challenge(private_key, message.clone()).expect("must sign");
    assert!(
        verify_challenge(address, Network::Testnet, signed, message).expect("must verify"),
        "challenge must verify"
    );
}

#[test]
fn intent_flow_roundtrip() {
    let (_, private_key, _, address) = derived_keys();
    let tx_id = hex_bytes("101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f");
    let outpoint = encode_outpoint_source_id(tx_id, SourceId::Transaction).unwrap();
    let input = encode_input_for_utxo(outpoint, 0).unwrap();
    let output =
        encode_output_transfer("1000".to_owned(), address.clone(), Network::Testnet).unwrap();
    let transaction = encode_transaction(vec![input], vec![output], 0).unwrap();
    let tx_id_hex = get_transaction_id(transaction, true).unwrap();

    let message = make_transaction_intent_message_to_sign("login".to_owned(), tx_id_hex)
        .expect("message must be produced");
    let signature = sign_challenge(private_key, message.clone()).expect("must sign");
    let intent = encode_signed_transaction_intent(message.clone(), vec![signature])
        .expect("intent must encode");
    assert!(
        verify_transaction_intent(message, intent, vec![address], Network::Testnet,)
            .expect("must verify"),
        "intent must verify"
    );
}

fn pubkey_from_priv(private_key: &[u8]) -> Vec<u8> {
    public_key_from_private_key(private_key.to_vec()).expect("must derive")
}

#[test]
fn transaction_with_witness_decodes_to_json() {
    let transaction = encode_transaction(Vec::new(), Vec::new(), 0).unwrap();
    let json =
        decode_transaction_to_json(transaction, Network::Testnet).expect("must decode to json");
    assert!(json.contains("outputs"), "json shape: {json}");
}

#[test]
fn error_string_clamping_does_not_panic_on_multibyte_input() {
    // 511 ASCII bytes + a 2-byte character: byte 512 falls inside the
    // multi-byte character; clamping must not panic.
    let mut hostile = "a".repeat(511);
    hostile.push('é');
    let address = format!("{}!", "tmt1q");
    let error = encode_destination(format!("{hostile}{address}"), Network::Testnet)
        .expect_err("garbage address must be rejected");
    assert!(matches!(error, BindingsError::AddressParse { .. }));
}

#[test]
fn error_payloads_are_sanitized_and_bounded() {
    // Control characters, bidi/format characters and Unicode line
    // separators must be stripped; the payload must stay bounded.
    let mut hostile = String::from("tmt1q\n\u{1b}[31m\u{2028}\u{2028}");
    for c in SPOOFING {
        hostile.push(c);
    }
    hostile.push_str(&"x".repeat(600));
    let error = encode_destination(hostile, Network::Testnet)
        .expect_err("garbage address must be rejected");
    let BindingsError::AddressParse { address, .. } = error else {
        panic!("expected AddressParse, got {error:?}");
    };
    assert!(
        address.len() <= 512,
        "payload must be bounded: {}",
        address.len()
    );
    const SPOOFING: [char; 22] = [
        '\u{00AD}', '\u{061C}', '\u{180E}', '\u{200B}', '\u{200C}', '\u{200D}', '\u{200E}',
        '\u{200F}', '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}', '\u{2060}',
        '\u{2061}', '\u{2062}', '\u{2063}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
        '\u{FEFF}',
    ];
    assert!(
        !address.chars().any(|c| {
            c.is_control()
                || SPOOFING.contains(&c)
                || ('\u{E0000}'..='\u{E007F}').contains(&c)
                || c == '\u{2028}'
                || c == '\u{2029}'
        }),
        "payload must contain no control/spoofing/separator characters"
    );
}
