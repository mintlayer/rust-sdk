// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Key generation and BIP32-style derivation, mirroring the wasm-bindings
//! exports.
//!
//! Keys cross the FFI boundary as byte arrays in the runtime SCALE format
//! (the same bytes the WASM wrappers produce and consume), so outputs are
//! byte-for-byte identical between the two frontends.

use mintlayer_sdk::crypto;
use mintlayer_sdk::crypto::types::{
    DecodeAll, Derivable, Encode, ExtendedPrivateKey, ExtendedPublicKey, PrivateKey,
};

use super::error::{BindingsError, decoding};
use super::network::Network;

/// The decoded parts of an extended private key in the runtime format.
///
/// `derivation_path` holds the absolute path levels in encoded form (with
/// the hardened bit set as in the runtime format, e.g. `44'` is
/// `0x8000002c`). `private_key` carries the plain private key as its own
/// runtime blob, which can be passed back to [`public_key_from_private_key`]
/// and friends. [`extended_private_key_encode`] rebuilds the canonical
/// extended-key blob from these parts; the pair is covered by roundtrip
/// tests.
#[derive(Clone, uniffi::Record)]
pub struct ExtendedPrivateKeyData {
    /// The absolute derivation path levels, in encoded form.
    pub derivation_path: Vec<u32>,
    /// The BIP32 chain code (32 bytes).
    pub chain_code: Vec<u8>,
    /// The plain public key as a runtime blob (holder byte + 33-byte
    /// compressed point).
    pub public_key: Vec<u8>,
    /// The plain private key as a runtime blob.
    pub private_key: Vec<u8>,
}

/// Generates a new random private key.
#[uniffi::export]
pub fn make_private_key() -> Vec<u8> {
    crypto::make_private_key().encode()
}

/// Creates the default account extended private key for a BIP39 mnemonic.
///
/// The derivation path is `m/44'/<coin type>/0'` where the coin type depends
/// on the network. The optional BIP39 passphrase changes the derived seed
/// entirely and must be remembered together with the mnemonic.
#[uniffi::export]
pub fn make_default_account_privkey(
    mnemonic: String,
    network: Network,
    passphrase: Option<String>,
) -> Result<Vec<u8>, BindingsError> {
    let key =
        crypto::make_default_account_privkey(&mnemonic, network.into(), passphrase.as_deref())?;
    Ok(key.encode())
}

/// Decodes an extended private key blob into its parts.
///
/// The input is the runtime blob produced by [`make_default_account_privkey`]
/// (or [`extended_private_key_encode`]).
#[uniffi::export]
pub fn extended_private_key_decode(key: Vec<u8>) -> Result<ExtendedPrivateKeyData, BindingsError> {
    let key = ExtendedPrivateKey::decode_all(&mut &key[..]).map_err(decoding)?;
    let derivation_path = Derivable::get_derivation_path(&key)
        .as_slice()
        .iter()
        .map(|level| (*level).into_encoded_index())
        .collect();
    let (public_key, chain_code) = key.to_public_key().into_public_key_and_chain_code();
    let private_key = key.private_key().encode();
    Ok(ExtendedPrivateKeyData {
        derivation_path,
        chain_code: chain_code.into_array().to_vec(),
        public_key: public_key.encode(),
        private_key,
    })
}

/// Encodes decoded extended-private-key parts back into the canonical
/// runtime blob.
///
/// The parts must be consistent: the canonical blob is re-derived and
/// re-checked against the parts, so a mismatched record is rejected instead
/// of producing a corrupt key.
#[uniffi::export]
pub fn extended_private_key_encode(key: ExtendedPrivateKeyData) -> Result<Vec<u8>, BindingsError> {
    let scalar = scalar_from_plain_private_key(&key.private_key)?;
    if key.chain_code.len() != 32 {
        return Err(BindingsError::Decoding {
            message: "chain code must be exactly 32 bytes".to_owned(),
        });
    }
    // An empty path is valid: it is a master key in the runtime format
    // (holder byte, zero path levels, chain code, scalar).
    if key.derivation_path.len() > u8::MAX as usize {
        return Err(BindingsError::Decoding {
            message: "derivation path length out of range".to_owned(),
        });
    }

    // Assemble the runtime blob: holder byte, one-byte path size,
    // big-endian path levels, chain code, scalar. The layout is not assumed
    // on trust: the assembled bytes are parsed by mintlayer-core's own
    // decoder below, the decoded key is cross-checked field-by-field against
    // this record before anything is returned, and the parity tests pin the
    // same chain against captured reference outputs of the WASM bindings.
    let mut bytes = Vec::with_capacity(2 + key.derivation_path.len() * 4 + 32 + 32);
    bytes.push(0x00); // ExtendedPrivateKeyHolder::Secp256k1Schnorr
    bytes.push(key.derivation_path.len() as u8);
    for level in &key.derivation_path {
        bytes.extend_from_slice(&level.to_be_bytes());
    }
    bytes.extend_from_slice(&key.chain_code);
    bytes.extend_from_slice(scalar);

    let decoded = ExtendedPrivateKey::decode_all(&mut &bytes[..]).map_err(decoding)?;
    let canonical = decoded.encode();
    // The reassembled blob must decode to exactly the parts it was built
    // from; anything else is a corrupted record. The public-key comparison
    // binds the scalar (secp256k1 keys are determined by their public
    // key), so the secret scalar itself is never copied for comparison.
    let path_matches = Derivable::get_derivation_path(&decoded)
        .as_slice()
        .iter()
        .map(|level| (*level).into_encoded_index())
        .eq(key.derivation_path.iter().copied());
    let (public_key, chain_code) = decoded.to_public_key().into_public_key_and_chain_code();
    if !path_matches
        || chain_code.into_array().as_slice() != key.chain_code.as_slice()
        || public_key.encode() != key.public_key
    {
        return Err(BindingsError::Decoding {
            message: "extended private key parts are inconsistent".to_owned(),
        });
    }
    Ok(canonical)
}

/// Derives the receiving private key at `key_index` from an account key.
///
/// The derivation path is `<account>/0/<key_index>`. The result is a plain
/// private key blob, not an address.
#[uniffi::export]
pub fn make_receiving_address(
    account_privkey: Vec<u8>,
    key_index: u32,
) -> Result<Vec<u8>, BindingsError> {
    let key = ExtendedPrivateKey::decode_all(&mut &account_privkey[..]).map_err(decoding)?;
    Ok(crypto::make_receiving_address(&key, key_index)?.encode())
}

/// Derives the change private key at `key_index` from an account key.
///
/// The derivation path is `<account>/1/<key_index>`. The result is a plain
/// private key blob, not an address.
#[uniffi::export]
pub fn make_change_address(
    account_privkey: Vec<u8>,
    key_index: u32,
) -> Result<Vec<u8>, BindingsError> {
    let key = ExtendedPrivateKey::decode_all(&mut &account_privkey[..]).map_err(decoding)?;
    Ok(crypto::make_change_address(&key, key_index)?.encode())
}

/// Derives the receiving public key at `key_index` from an extended public
/// key (watch-only derivation).
#[uniffi::export]
pub fn make_receiving_address_public_key(
    account_public_key: Vec<u8>,
    key_index: u32,
) -> Result<Vec<u8>, BindingsError> {
    let key = ExtendedPublicKey::decode_all(&mut &account_public_key[..]).map_err(decoding)?;
    Ok(crypto::make_receiving_address_public_key(&key, key_index)?.encode())
}

/// Derives the change public key at `key_index` from an extended public key
/// (watch-only derivation).
#[uniffi::export]
pub fn make_change_address_public_key(
    account_public_key: Vec<u8>,
    key_index: u32,
) -> Result<Vec<u8>, BindingsError> {
    let key = ExtendedPublicKey::decode_all(&mut &account_public_key[..]).map_err(decoding)?;
    Ok(crypto::make_change_address_public_key(&key, key_index)?.encode())
}

/// Returns the public key corresponding to a private key.
#[uniffi::export]
pub fn public_key_from_private_key(private_key: Vec<u8>) -> Result<Vec<u8>, BindingsError> {
    let key = PrivateKey::decode_all(&mut &private_key[..]).map_err(decoding)?;
    Ok(crypto::public_key_from_private_key(&key).encode())
}

/// Returns the extended public key corresponding to an extended private key.
#[uniffi::export]
pub fn extended_public_key_from_extended_private_key(
    account_privkey: Vec<u8>,
) -> Result<Vec<u8>, BindingsError> {
    let key = ExtendedPrivateKey::decode_all(&mut &account_privkey[..]).map_err(decoding)?;
    Ok(crypto::extended_public_key_from_extended_private_key(&key).encode())
}

/// Extracts the raw scalar bytes from a plain private key runtime blob.
///
/// The runtime format for the plain key is fixed by the codebase (a single
/// secp256k1/Schnorr key kind), and the layout is pinned by the parity tests
/// against the reference implementation rather than assumed.
fn scalar_from_plain_private_key(blob: &[u8]) -> Result<&[u8], BindingsError> {
    match blob {
        // [holder 0x00][32-byte scalar]
        [0x00, scalar @ ..] if scalar.len() == 32 => Ok(scalar),
        _ => Err(BindingsError::Decoding {
            message: "unsupported plain private key blob layout".to_owned(),
        }),
    }
}
