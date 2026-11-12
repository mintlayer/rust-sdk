// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Parity tests against the reference implementation.
//!
//! The constants below are the outputs of `@mintlayer/wasm-lib` 1.4.0 (built
//! from mintlayer-core) for the public BIP39 test vector
//! `"abandon" × 23 + "art"` (24 words, checksum valid — public test vector,
//! no secret material). The bindings must reproduce them byte-for-byte.

use hex::FromHex;
use mintlayer_wallet_bindings::{
    ExtendedPrivateKeyData, Network, extended_private_key_decode, extended_private_key_encode,
    extended_public_key_from_extended_private_key, make_change_address,
    make_change_address_public_key, make_default_account_privkey, make_receiving_address,
    make_receiving_address_public_key, pubkey_to_pubkeyhash_address, public_key_from_private_key,
};

/// The public BIP39 test vector (23 × "abandon" + "art").
const MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

fn hex_bytes(s: &str) -> Vec<u8> {
    Vec::from_hex(s).expect("test constant must be valid hex")
}

fn hex_str(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

/// Runtime blob of the default account extended private key, as produced by
/// the wasm bindings (network-parameterized).
const XPRIV: [(&str, &str); 2] = [
    (
        "testnet",
        "00038000002c800000018000000012ed17c3e72223beb38ea98ec1317581f02458f02b0ecf4abe02c6490eca170a900d967bc0c84b9c2da3b55f7db9f76cbdc0469176a76d01141af2a705b4353a",
    ),
    (
        "mainnet",
        "00038000002c80004d4c800000005280d67624d70241021ee99da728121cb69e2356e7776cbc360fc25dedc35d8c3ee4db2b274aa3fac7024333d45f916ac113cd63c47e9444d39d46827dce1a24",
    ),
];

/// Runtime blob of the account extended public key.
const XPUB: [(&str, &str); 2] = [
    (
        "testnet",
        "00038000002c800000018000000012ed17c3e72223beb38ea98ec1317581f02458f02b0ecf4abe02c6490eca170a03b9385a6fbee0caa6e79407f83b0053e2d864b6c7a8f32ce61c3f495b5dd5d027",
    ),
    (
        "mainnet",
        "00038000002c80004d4c800000005280d67624d70241021ee99da728121cb69e2356e7776cbc360fc25dedc35d8c026c3ac93c48d49fa446a13c846b1ef04255e1526018ea445949078d96947e7454",
    ),
];

/// Receiving public key (key index 0), as a runtime blob.
const RECEIVING_PUBKEY: [(&str, &str); 2] = [
    (
        "testnet",
        "00035a6f28253f35e2ff1ab0b086b8c87f731159661157e5be2ca98188369625dc32",
    ),
    (
        "mainnet",
        "00030db84115b4aa824edfefeb4998d3e5e9131188719cb960540010e20f29777a05",
    ),
];

/// bech32 address of the receiving public key (key index 0).
const ADDRESS: [(&str, &str); 2] = [
    ("testnet", "tmt1qyxqfq37rra32sq6zhayqcpjwh4trc07mcfardwt"),
    ("mainnet", "mtc1q82twmwwxa3l2pe26gf0zgghu69jtez3qq2ajqyl"),
];

/// Acceptance vectors in the normalized `levels || point || chain code`
/// form, as documented by the consumers of the bindings.
const NORMALIZED_XPUB: [(&str, &str); 2] = [
    (
        "testnet",
        "8000002c800000018000000003b9385a6fbee0caa6e79407f83b0053e2d864b6c7a8f32ce61c3f495b5dd5d02712ed17c3e72223beb38ea98ec1317581f02458f02b0ecf4abe02c6490eca170a",
    ),
    (
        "mainnet",
        "8000002c80004d4c80000000026c3ac93c48d49fa446a13c846b1ef04255e1526018ea445949078d96947e74545280d67624d70241021ee99da728121cb69e2356e7776cbc360fc25dedc35d8c",
    ),
];

fn network(name: &str) -> Network {
    match name {
        "testnet" => Network::Testnet,
        "mainnet" => Network::Mainnet,
        other => panic!("unknown network in test table: {other}"),
    }
}

#[test]
fn default_account_privkey_matches_reference() {
    for (name, expected) in XPRIV {
        let blob = make_default_account_privkey(MNEMONIC.to_owned(), network(name), None)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(hex_str(&blob), expected, "{name}: xpriv blob mismatch");
    }
}

#[test]
fn extended_public_key_matches_reference() {
    for (name, xpriv) in XPRIV {
        let xpub = extended_public_key_from_extended_private_key(hex_bytes(xpriv))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let expected = XPUB.iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(hex_str(&xpub), expected, "{name}: xpub blob mismatch");
    }
}

#[test]
fn receiving_public_key_matches_reference() {
    for (name, xpub) in XPUB {
        let pubkey = make_receiving_address_public_key(hex_bytes(xpub), 0)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let expected = RECEIVING_PUBKEY.iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(
            hex_str(&pubkey),
            expected,
            "{name}: receiving pubkey mismatch"
        );
    }
}

#[test]
fn address_matches_reference() {
    for (name, xpub) in XPUB {
        let pubkey = make_receiving_address_public_key(hex_bytes(xpub), 0)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let address = pubkey_to_pubkeyhash_address(pubkey, network(name))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let expected = ADDRESS.iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(address, expected, "{name}: address mismatch");
    }
}

#[test]
fn extended_public_key_matches_normalized_format() {
    // The normalized form is `levels || compressed point || chain code`;
    // the runtime blob is `[holder][path size][levels][chain code][point]`.
    for (name, xpub) in XPUB {
        let blob = hex_bytes(xpub);
        let expected = NORMALIZED_XPUB.iter().find(|(n, _)| *n == name).unwrap().1;
        let levels = &blob[2..14];
        let chain_code = &blob[14..46];
        let point = &blob[46..79];
        let mut normalized = Vec::with_capacity(12 + 33 + 32);
        normalized.extend_from_slice(levels);
        normalized.extend_from_slice(point);
        normalized.extend_from_slice(chain_code);
        assert_eq!(
            hex_str(&normalized),
            expected,
            "{name}: normalized xpub mismatch"
        );
    }
}

#[test]
fn extended_private_key_decode_exposes_reference_parts() {
    for (name, xpriv) in XPRIV {
        let decoded =
            extended_private_key_decode(hex_bytes(xpriv)).unwrap_or_else(|e| panic!("{name}: {e}"));
        let expected_levels = match name {
            "testnet" => vec![0x8000_002c, 0x8000_0001, 0x8000_0000],
            "mainnet" => vec![0x8000_002c, 0x8000_4d4c, 0x8000_0000],
            other => panic!("unknown network in test table: {other}"),
        };
        assert_eq!(decoded.derivation_path, expected_levels, "{name}: path");
        assert_eq!(decoded.chain_code.len(), 32, "{name}: chain code length");
        // Plain-key runtime blobs are [holder byte][33-byte compressed point].
        assert_eq!(
            decoded.public_key.len(),
            34,
            "{name}: public key blob length"
        );
        // The point inside the record's public key blob must match the
        // reference xpub's embedded point.
        let xpub = XPUB.iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(
            &decoded.public_key[1..34],
            &hex_bytes(xpub)[46..79],
            "{name}: record public key"
        );
    }
}

#[test]
fn extended_private_key_encode_roundtrips() {
    for (name, xpriv) in XPRIV {
        let decoded =
            extended_private_key_decode(hex_bytes(xpriv)).unwrap_or_else(|e| panic!("{name}: {e}"));
        let reencoded =
            extended_private_key_encode(decoded).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            hex_str(&reencoded),
            xpriv,
            "{name}: encode roundtrip mismatch"
        );
    }
}

#[test]
fn extended_private_key_encode_rejects_inconsistent_parts() {
    let mut decoded =
        extended_private_key_decode(hex_bytes(XPRIV[0].1)).expect("decode must succeed");
    decoded.public_key = vec![0u8; 33];
    let error =
        extended_private_key_encode(decoded).expect_err("inconsistent parts must be rejected");
    assert!(
        matches!(
            error,
            mintlayer_wallet_bindings::BindingsError::Decoding { .. }
        ),
        "expected a decoding error, got {error:?}"
    );
}

#[test]
fn private_and_public_derivation_agree() {
    // The private-key path and the watch-only public-key path must produce
    // the same keys at every index.
    for (name, xpriv) in XPRIV {
        let xpub = XPUB.iter().find(|(n, _)| *n == name).unwrap().1;
        for index in [0u32, 1u32, 42u32] {
            let private_blob = make_receiving_address(hex_bytes(xpriv), index)
                .unwrap_or_else(|e| panic!("{name} #{index}: {e}"));
            let from_private = public_key_from_private_key(private_blob)
                .unwrap_or_else(|e| panic!("{name} #{index}: {e}"));
            let from_public = make_receiving_address_public_key(hex_bytes(xpub), index)
                .unwrap_or_else(|e| panic!("{name} #{index}: {e}"));
            assert_eq!(
                from_private, from_public,
                "{name} receiving #{index}: private and public derivation disagree"
            );

            let private_blob = make_change_address(hex_bytes(xpriv), index)
                .unwrap_or_else(|e| panic!("{name} change #{index}: {e}"));
            let from_private = public_key_from_private_key(private_blob)
                .unwrap_or_else(|e| panic!("{name} change #{index}: {e}"));
            let from_public = make_change_address_public_key(hex_bytes(xpub), index)
                .unwrap_or_else(|e| panic!("{name} change #{index}: {e}"));
            assert_eq!(
                from_private, from_public,
                "{name} change #{index}: private and public derivation disagree"
            );
        }
    }
}

#[test]
fn decoded_plain_private_key_key_is_self_consistent() {
    // The plain private key blob in the decode record must resolve to the
    // same public key the record carries.
    for (name, xpriv) in XPRIV {
        let decoded =
            extended_private_key_decode(hex_bytes(xpriv)).unwrap_or_else(|e| panic!("{name}: {e}"));
        let from_plain = public_key_from_private_key(decoded.private_key.clone())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            from_plain, decoded.public_key,
            "{name}: plain private key does not match record public key"
        );
        let _ = ExtendedPrivateKeyData {
            derivation_path: decoded.derivation_path,
            chain_code: decoded.chain_code,
            public_key: decoded.public_key,
            private_key: decoded.private_key,
        };
    }
}

#[test]
fn invalid_input_is_an_error_not_a_panic() {
    assert!(extended_public_key_from_extended_private_key(vec![1, 2, 3]).is_err());
    assert!(make_receiving_address_public_key(vec![], 0).is_err());
    assert!(public_key_from_private_key(vec![0xff; 40]).is_err());
    assert!(extended_private_key_decode(vec![0x00, 0x01]).is_err());
    assert!(pubkey_to_pubkeyhash_address(vec![0u8; 33], Network::Testnet).is_err());
    assert!(
        make_default_account_privkey("not a mnemonic".to_owned(), Network::Mainnet, None).is_err()
    );
    assert!(make_default_account_privkey(MNEMONIC.to_owned(), Network::Mainnet, None).is_ok());
}
