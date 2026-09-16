// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Cryptography and transaction-building tests, mirroring the go-sdk
//! `wasm/client_test.go` suite and the wasm-wrappers test vectors.

use mintlayer_sdk::crypto::types::{
    DecodeAll, Encode, H256, InputWitness, PrivateKey, Transaction, TxInput, TxOutput,
};
use mintlayer_sdk::crypto::{
    Amount, Error, IsTokenUnfreezable, Network, SigHashType, SourceId, TxAdditionalInfo,
    data_deposit_fee, decode_partially_signed_transaction_to_json,
    decode_signed_transaction_to_json, decode_transaction, decode_transaction_lenient,
    effective_pool_balance, encode_destination, encode_input_for_change_token_authority,
    encode_input_for_change_token_metadata_uri, encode_input_for_conclude_order,
    encode_input_for_fill_order, encode_input_for_freeze_order, encode_input_for_freeze_token,
    encode_input_for_lock_token_supply, encode_input_for_mint_tokens,
    encode_input_for_unfreeze_token, encode_input_for_unmint_tokens, encode_input_for_utxo,
    encode_input_for_withdraw_from_delegation, encode_lock_for_block_count,
    encode_lock_for_seconds, encode_lock_until_height, encode_lock_until_time,
    encode_outpoint_source_id, encode_output_create_stake_pool, encode_output_htlc,
    encode_output_issue_nft, encode_output_transfer, encode_partially_signed_transaction,
    encode_signed_transaction, encode_signed_transaction_intent, encode_stake_pool_data,
    encode_transaction, encode_witness, encode_witness_no_signature, estimate_transaction_size,
    fungible_token_issuance_fee, get_delegation_id, get_order_id, get_pool_id, get_token_id,
    make_default_account_privkey, make_private_key, make_receiving_address,
    make_transaction_intent_message_to_sign, nft_issuance_fee, pubkey_to_pubkeyhash_address,
    public_key_from_private_key, sign_challenge, sign_message_for_spending,
    staking_pool_spend_maturity_block_count, token_change_authority_fee, token_freeze_fee,
    token_supply_change_fee, transaction_id, verify_challenge, verify_signature_for_spending,
    verify_transaction_intent,
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

/// A fixed Schnorr private key (a public test vector, identical to
/// `MAINNET_RECEIVING_0`), so that `witness_roundtrip_on_fake_tx` exercises a
/// deterministic key instead of a randomly generated one. Schnorr signing
/// itself still draws fresh auxiliary randomness per signature, so only the
/// deterministic properties of the witness can be asserted.
const FIXED_SIGNING_PRIVKEY: &str =
    "00b88adfb44da2c1fd5f12f7996bd147f45bd0b8917fa8842d4c901b965d5dad1f";
const TX_HEX: &str = "0100040000ff5d9a94390ee97208d31aa5c3b5ddbd8df9d308069df2ebf5283f7ce3e4261401000000080340f9924e4da0af7dc8c5be71a9c9e05962c7bf4ef96127fde7a7b4e1469e48620f0080e03779c31102000365807e3b4147cb978b78715e60606092f89dc769586e98456850bd3b449c87b400203015e9ef9fc142569e0f966bc0188464fa712a841e14002e0fe952a076a26c01e539c5f0ceba927ab8f8f55f274af739ce4eef3700000b00204aa9d10100000b409e4c355d010199e4ec3a5b176140ef9cd58c7d3579fdb0ecb21a";
const TX_SIGNED_HEX: &str = "0100040000ff5d9a94390ee97208d31aa5c3b5ddbd8df9d308069df2ebf5283f7ce3e4261401000000080340f9924e4da0af7dc8c5be71a9c9e05962c7bf4ef96127fde7a7b4e1469e48620f0080e03779c31102000365807e3b4147cb978b78715e60606092f89dc769586e98456850bd3b449c87b400203015e9ef9fc142569e0f966bc0188464fa712a841e14002e0fe952a076a26c01e539c5f0ceba927ab8f8f55f274af739ce4eef3700000b00204aa9d10100000b409e4c355d010199e4ec3a5b176140ef9cd58c7d3579fdb0ecb21a0401018d010002eddd003bfb6333123e682abe6923da1d38faa4f0e0d9e2ee42d5aa46c152a34800a749a30c8c9c33696ce407fc145ebc9824e17b778d0d9ccc8129be52f37b74160e60f6689ac2f481071e1a63d9cf0f6eab84c2703b5e9f229cd8188ce092edd4";

fn fake_utxo_transaction(private_key: PrivateKey) -> (PrivateKey, String, Transaction, TxOutput) {
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
    let (_, _, transaction, _) = fake_utxo_transaction(make_private_key());

    let tx_id = transaction_id(&transaction);
    assert_eq!(tx_id.len(), 64);
    assert!(
        tx_id.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "transaction id must be lowercase hex, got {tx_id}"
    );
}

#[test]
fn timelocks_encode() {
    // Exact SCALE encodings of the OutputTimeLock variants.
    assert_eq!(
        encode_lock_for_block_count(100).encode(),
        hex::decode("029101").unwrap(),
        "ForBlockCount(100): variant 0x02 + compact(100) = 0xa1 0x01"
    );
    assert_eq!(
        encode_lock_for_seconds(86_400).encode(),
        hex::decode("0302460500").unwrap(),
        "ForSeconds(86400): variant 0x03 + compact(86400) = 0x01 0x46 0x05 0x00"
    );
    assert_eq!(
        encode_lock_until_height(500_000).encode(),
        hex::decode("0082841e00").unwrap(),
        "UntilHeight(500000): variant 0x00 + compact(500000) = 0x82 0x84 0x1e 0x00"
    );
    assert_eq!(
        encode_lock_until_time(1_700_000_000).encode(),
        hex::decode("010300f15365").unwrap(),
        "UntilTime(1700000000): variant 0x01 + compact(1700000000) = 0x03 0x00 0xf1 0x53 0x65"
    );
}

#[test]
fn fees_match_consensus_values() {
    let height = 500_000u64;
    let network = Network::Mainnet;

    // Consensus fee schedule at Mainnet height 500_000, pinned in atoms.
    assert_eq!(
        fungible_token_issuance_fee(height, network).into_atoms(),
        10_000_000_000_000u128,
        "fungible token issuance fee"
    );
    assert_eq!(
        nft_issuance_fee(height, network).into_atoms(),
        500_000_000_000u128,
        "NFT issuance fee"
    );
    assert_eq!(
        data_deposit_fee(height, network).into_atoms(),
        2_000_000_000_000u128,
        "data deposit fee"
    );
    assert_eq!(
        token_supply_change_fee(height, network).into_atoms(),
        5_000_000_000_000u128,
        "token supply change fee"
    );
    assert_eq!(
        token_freeze_fee(height, network).into_atoms(),
        5_000_000_000_000u128,
        "token freeze fee"
    );
    assert_eq!(
        token_change_authority_fee(height, network).into_atoms(),
        2_000_000_000_000u128,
        "token change authority fee"
    );
}

#[test]
fn staking_helpers() {
    // Post-fork Mainnet maturity is exactly 7200 blocks.
    let maturity = staking_pool_spend_maturity_block_count(500_000, Network::Mainnet);
    assert_eq!(
        maturity, 7200,
        "staking pool spend maturity at Mainnet height 500_000"
    );

    let pledge = Amount::from_atoms(1_000_000_000_000_000);
    let pool_balance = Amount::from_atoms(10_000_000_000_000_000);
    let effective = effective_pool_balance(Network::Mainnet, pledge, pool_balance)
        .expect("effective pool balance must be computable");
    assert_eq!(
        effective.into_atoms(),
        9_309_784_157_607_562u128,
        "pledge-capped effective balance"
    );
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
    // Schnorr signing derives fresh auxiliary randomness for every signature,
    // so the signature bytes cannot be hex-pinned. Instead the test anchors on
    // a FIXED private key and asserts every deterministic property of the
    // produced witness.
    let fixed_key = <PrivateKey as DecodeAll>::decode_all(
        &mut &hex::decode(FIXED_SIGNING_PRIVKEY).unwrap()[..],
    )
    .expect("fixed private key must decode");
    assert_eq!(
        hex::encode(fixed_key.encode()),
        FIXED_SIGNING_PRIVKEY,
        "fixed private key must survive a decode/encode roundtrip"
    );

    let (private_key, address, transaction, output) = fake_utxo_transaction(fixed_key);

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

    // (a) The produced witness must decode back into an InputWitness.
    let decoded_witness =
        InputWitness::decode_all(&mut &witness.encode()[..]).expect("witness must decode");
    // (b) It must be a standard signature authorizing every sighash mode.
    let InputWitness::Standard(signature) = &decoded_witness else {
        panic!("witness must be a standard input signature, got {decoded_witness:?}");
    };
    assert_eq!(signature.sighash_type(), SigHashType::all());

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
    assert_eq!(
        pool_id, "mpool1zte8hxywgpaqw4xalxpj2kxmuj83tprw2f7rgcgweytmv45kfzyqduwah5",
        "pool id derived from an all-zero-hash outpoint"
    );
}

#[test]
fn get_token_id_from_inputs() {
    let source_id = encode_outpoint_source_id(H256::from_slice(&[0u8; 32]), SourceId::Transaction);
    let fake_input = encode_input_for_utxo(source_id, 0);

    let token_id = get_token_id(&[fake_input], 500_000, Network::Mainnet).unwrap();
    assert_eq!(
        token_id, "mmltk1ht59xvv2sdxz28txuwryy55yl5qq9tf9657kvnqulfy9a2g3csssee4n2n",
        "token id derived from an all-zero-hash outpoint at height 500_000"
    );
}

/// A mainnet VRF public key, taken from mintlayer-core's own address test
/// vectors (`common/src/address/hexified.rs`), so that stake-pool creation can
/// be exercised without deriving a VRF key (the SDK exposes no helper for it).
const MAINNET_VRF_PUBLIC_KEY: &str =
    "mvrfpk1qqyxcl4tc6y9amf2vmv6sgu8x5jwqlxawx73vhgemkduag9c8ku57m03mze";

#[test]
fn input_constructors_encode() {
    let network = Network::Mainnet;

    // A delegation id derived from a UTXO outpoint over hash [7u8; 32].
    let delegation_source =
        encode_outpoint_source_id(H256::from_slice(&[7u8; 32]), SourceId::Transaction);
    let delegation_id =
        get_delegation_id(&[encode_input_for_utxo(delegation_source, 0)], network).unwrap();

    // The token id pinned by `get_token_id_from_inputs`.
    let token_source =
        encode_outpoint_source_id(H256::from_slice(&[0u8; 32]), SourceId::Transaction);
    let token_id =
        get_token_id(&[encode_input_for_utxo(token_source, 0)], 500_000, network).unwrap();

    let fresh_authority =
        pubkey_to_pubkeyhash_address(&public_key_from_private_key(&make_private_key()), network);

    // Type alias to avoid clippy::type_complexity on the cases vector. The
    // trait object bound is tied to 'a so the closures may borrow locals.
    type InputCase<'a> = (&'a str, Box<dyn Fn() -> Result<TxInput, Error> + 'a>);

    let cases: Vec<InputCase> = vec![
        (
            "withdraw_from_delegation",
            Box::new(|| {
                encode_input_for_withdraw_from_delegation(
                    &delegation_id,
                    Amount::from_atoms(1),
                    0,
                    network,
                )
            }),
        ),
        (
            "mint_tokens",
            Box::new(|| encode_input_for_mint_tokens(&token_id, Amount::from_atoms(1), 0, network)),
        ),
        (
            "unmint_tokens",
            Box::new(|| encode_input_for_unmint_tokens(&token_id, 0, network)),
        ),
        (
            "lock_token_supply",
            Box::new(|| encode_input_for_lock_token_supply(&token_id, 0, network)),
        ),
        (
            "freeze_token",
            Box::new(|| {
                encode_input_for_freeze_token(&token_id, IsTokenUnfreezable::Yes, 0, network)
            }),
        ),
        (
            "unfreeze_token",
            Box::new(|| encode_input_for_unfreeze_token(&token_id, 0, network)),
        ),
        (
            "change_token_authority",
            Box::new(|| {
                encode_input_for_change_token_authority(&token_id, &fresh_authority, 0, network)
            }),
        ),
        (
            "change_token_metadata_uri",
            Box::new(|| {
                encode_input_for_change_token_metadata_uri(
                    &token_id,
                    "https://example.com/metadata",
                    0,
                    network,
                )
            }),
        ),
    ];

    for (name, build) in cases {
        let input = build().unwrap_or_else(|error| panic!("{name} must encode: {error}"));
        assert!(
            !input.encode().is_empty(),
            "{name} produced an empty encoding"
        );
    }
}

#[test]
fn fill_order_fork_versions() {
    let network = Network::Mainnet;
    let source = encode_outpoint_source_id(H256::from_slice(&[0u8; 32]), SourceId::Transaction);
    let order_id = get_order_id(&[encode_input_for_utxo(source, 0)], network).unwrap();
    let destination =
        pubkey_to_pubkeyhash_address(&public_key_from_private_key(&make_private_key()), network);

    // Mainnet orders V1 activates at height 517_700; straddle it. Pre-fork
    // inputs are V0 AccountCommand, post-fork they are V1 OrderAccountCommand.
    let pre_fork = encode_input_for_fill_order(
        &order_id,
        Amount::from_atoms(1),
        &destination,
        0,
        500_000,
        network,
    )
    .expect("fill order must encode before the orders V1 fork");
    let post_fork = encode_input_for_fill_order(
        &order_id,
        Amount::from_atoms(1),
        &destination,
        0,
        5_000_000,
        network,
    )
    .expect("fill order must encode after the orders V1 fork");
    assert_ne!(
        pre_fork.encode(),
        post_fork.encode(),
        "fill-order encoding must change across the orders V1 fork"
    );

    let conclude_pre = encode_input_for_conclude_order(&order_id, 0, 500_000, network)
        .expect("conclude order must encode before the orders V1 fork");
    let conclude_post = encode_input_for_conclude_order(&order_id, 0, 5_000_000, network)
        .expect("conclude order must encode after the orders V1 fork");
    assert_ne!(
        conclude_pre.encode(),
        conclude_post.encode(),
        "conclude-order encoding must change across the orders V1 fork"
    );

    // Freezing an order only exists once orders V1 is active.
    match encode_input_for_freeze_order(&order_id, 500_000, network) {
        Err(Error::OrdersV1NotActivated) => {}
        other => panic!("expected OrdersV1NotActivated before the fork, got {other:?}"),
    }
    let freeze_post = encode_input_for_freeze_order(&order_id, 5_000_000, network)
        .expect("freeze order must encode after the orders V1 fork");
    assert!(!freeze_post.encode().is_empty());
}

#[test]
fn intents_roundtrip() {
    let message = make_transaction_intent_message_to_sign("transfer", EXPECTED_TX_ID)
        .expect("intent message must be produced");
    assert!(!message.is_empty());

    let signed = encode_signed_transaction_intent(&message, vec![vec![1u8; 64]])
        .expect("signed intent must be assembled");

    // An empty destination list can never verify: the intent carries one
    // signature but there is no input destination to check it against.
    let result = verify_transaction_intent(&message, &signed.encode(), &[], Network::Mainnet);
    assert!(
        result.is_err(),
        "verification with no destinations must fail"
    );
}

#[test]
fn partially_signed_transaction_roundtrip() {
    let (_, address, transaction, output) = fake_utxo_transaction(make_private_key());
    let destination = encode_destination(&address, Network::Mainnet).unwrap();

    let ptx = encode_partially_signed_transaction(
        transaction,
        vec![None],
        vec![Some(output)],
        vec![Some(destination)],
        vec![None],
        TxAdditionalInfo::new(),
        Network::Mainnet,
    )
    .expect("partially signed transaction must be assembled");

    let json = decode_partially_signed_transaction_to_json(&ptx.encode(), Network::Mainnet)
        .expect("partially signed transaction must decode to JSON");
    assert_eq!(
        json.get("type").and_then(|value| value.as_str()),
        Some("V1"),
        "partially signed transaction JSON must carry the V1 tag"
    );
    assert!(
        json.get("tx").and_then(|value| value.as_object()).is_some(),
        "partially signed transaction JSON must contain the transaction"
    );
}

#[test]
fn htlc_and_special_outputs_encode() {
    let network = Network::Mainnet;
    let key = make_private_key();
    let address = pubkey_to_pubkeyhash_address(&public_key_from_private_key(&key), network);

    // HTLC output whose secret hash is the SHA-256 hash of the empty string,
    // truncated to the 20-byte HtlcSecretHash length the chain enforces
    // (a full 32-byte hex string is rejected as "invalid length").
    let htlc = encode_output_htlc(
        Amount::from_atoms(1),
        None,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4",
        &address,
        &address,
        encode_lock_until_height(1_000_000),
        network,
    )
    .expect("HTLC output must encode");
    assert!(!htlc.encode().is_empty());

    // NFT issuance on top of the pinned token id. "TCK" satisfies the
    // 1..=12-byte alphanumeric ticker rule and the 32-byte media hash the
    // minimum hash length.
    let token_source =
        encode_outpoint_source_id(H256::from_slice(&[0u8; 32]), SourceId::Transaction);
    let token_id =
        get_token_id(&[encode_input_for_utxo(token_source, 0)], 500_000, network).unwrap();
    let nft = encode_output_issue_nft(
        &token_id, &address, "name", "TCK", "desc", &[1u8; 32], None, None, None, None, network,
    )
    .expect("NFT issuance output must encode");
    assert!(!nft.encode().is_empty());

    // Stake pool creation using a mainnet VRF public key from mintlayer-core's
    // address test vectors.
    let pool_source =
        encode_outpoint_source_id(H256::from_slice(&[0u8; 32]), SourceId::Transaction);
    let pool_id = get_pool_id(&[encode_input_for_utxo(pool_source, 0)], network).unwrap();
    let pool_data = encode_stake_pool_data(
        Amount::from_atoms(1_000_000_000_000),
        &address,
        MAINNET_VRF_PUBLIC_KEY,
        &address,
        100,
        Amount::from_atoms(1_000),
        network,
    )
    .expect("stake pool data must encode");
    let pool = encode_output_create_stake_pool(&pool_id, pool_data, network)
        .expect("create-stake-pool output must encode");
    assert!(!pool.encode().is_empty());
}
