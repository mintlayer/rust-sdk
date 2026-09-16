// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Key generation and BIP32-style derivation.

use ml_common as common;
use ml_crypto as crypto;

use common::chain::config::BIP44_PATH;
use crypto::key::{
    KeyKind, PrivateKey,
    extended::{ExtendedKeyKind, ExtendedPrivateKey, ExtendedPublicKey},
    hdkd::{
        child_number::ChildNumber, derivable::Derivable, derivation_path::DerivationPath, u31::U31,
    },
};

use super::{Error, Network, chain_config};

const RECEIVE_FUNDS_INDEX: ChildNumber = ChildNumber::from_normal(U31::from_u32_with_msb(0).0);
const CHANGE_FUNDS_INDEX: ChildNumber = ChildNumber::from_normal(U31::from_u32_with_msb(1).0);

/// Generates a new random private key from entropy.
pub fn make_private_key() -> PrivateKey {
    PrivateKey::new_from_entropy(KeyKind::Secp256k1Schnorr).0
}

/// Creates the default account extended private key for a mnemonic.
///
/// The derivation path is `44'/mintlayer_coin_type'/0'`. The optional BIP39
/// `passphrase` changes the derived seed entirely and must be remembered
/// together with the mnemonic.
pub fn make_default_account_privkey(
    mnemonic: &str,
    network: Network,
    passphrase: Option<&str>,
) -> Result<ExtendedPrivateKey, Error> {
    let mnemonic =
        bip39::Mnemonic::parse_in(bip39::Language::English, mnemonic).map_err(Error::from)?;

    let passphrase = passphrase.map(|p| zeroize::Zeroizing::new(p.to_owned()));
    let passphrase: &str = passphrase.as_ref().map(|p| p.as_str()).unwrap_or("");
    let seed = zeroize::Zeroizing::new(mnemonic.to_seed(passphrase));

    let root_key = ExtendedPrivateKey::new_master(seed.as_ref(), ExtendedKeyKind::Secp256k1Schnorr)
        .map_err(|error| Error::MessageSigning(error.to_string()))?;

    let account_index = U31::ZERO;
    let path: DerivationPath = vec![
        BIP44_PATH,
        chain_config(network).bip44_coin_type(),
        ChildNumber::from_hardened(account_index),
    ]
    .try_into()
    .map_err(|error: crypto::key::hdkd::derivable::DerivationError| {
        Error::InputSigning(error.to_string())
    })?;
    root_key
        .derive_absolute_path(&path)
        .map_err(|error| Error::InputSigning(error.to_string()))
}

/// Derives the receiving private key at `key_index` from an account key
/// (path: `<account>/0/<key_index>`).
pub fn make_receiving_address(
    account_key: &ExtendedPrivateKey,
    key_index: u32,
) -> Result<PrivateKey, Error> {
    derive_key(account_key.clone(), RECEIVE_FUNDS_INDEX, key_index).map(|key| key.private_key())
}

/// Derives the change private key at `key_index` from an account key
/// (path: `<account>/1/<key_index>`).
pub fn make_change_address(
    account_key: &ExtendedPrivateKey,
    key_index: u32,
) -> Result<PrivateKey, Error> {
    derive_key(account_key.clone(), CHANGE_FUNDS_INDEX, key_index).map(|key| key.private_key())
}

/// Derives the receiving public key at `key_index` from an extended public
/// key (watch-only derivation).
pub fn make_receiving_address_public_key(
    account_public_key: &ExtendedPublicKey,
    key_index: u32,
) -> Result<crypto::key::PublicKey, Error> {
    derive_key(account_public_key.clone(), RECEIVE_FUNDS_INDEX, key_index)
        .map(|key| key.into_public_key())
}

/// Derives the change public key at `key_index` from an extended public key
/// (watch-only derivation).
pub fn make_change_address_public_key(
    account_public_key: &ExtendedPublicKey,
    key_index: u32,
) -> Result<crypto::key::PublicKey, Error> {
    derive_key(account_public_key.clone(), CHANGE_FUNDS_INDEX, key_index)
        .map(|key| key.into_public_key())
}

/// Returns the public key corresponding to a private key.
pub fn public_key_from_private_key(private_key: &PrivateKey) -> crypto::key::PublicKey {
    crypto::key::PublicKey::from_private_key(private_key)
}

/// Returns the extended public key corresponding to an extended private key.
pub fn extended_public_key_from_extended_private_key(
    account_key: &ExtendedPrivateKey,
) -> ExtendedPublicKey {
    account_key.to_public_key()
}

fn derive_key<D: Derivable>(derivable: D, branch: ChildNumber, key_index: u32) -> Result<D, Error> {
    let index = U31::from_u32(key_index).ok_or(Error::InvalidKeyIndex)?;
    derivable
        .derive_child(branch)
        .map_err(|error| Error::InputSigning(error.to_string()))?
        .derive_child(ChildNumber::from_normal(index))
        .map_err(|error| Error::InputSigning(error.to_string()))
}
