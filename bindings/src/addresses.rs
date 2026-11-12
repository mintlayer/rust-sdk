// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Address encoding, mirroring the wasm-bindings exports.

use mintlayer_sdk::crypto;
use mintlayer_sdk::crypto::types::{ClassicMultisigChallenge, DecodeAll, Encode, PublicKey};

use super::error::{BindingsError, decoding};
use super::network::Network;

/// Encodes a public key into its bech32 public-key-hash address for the
/// given network.
#[uniffi::export]
pub fn pubkey_to_pubkeyhash_address(
    public_key: Vec<u8>,
    network: Network,
) -> Result<String, BindingsError> {
    let key = PublicKey::decode_all(&mut &public_key[..]).map_err(decoding)?;
    Ok(crypto::pubkey_to_pubkeyhash_address(&key, network.into()))
}

/// Parses a bech32 address into its encoded destination blob.
#[uniffi::export]
pub fn encode_destination(address: String, network: Network) -> Result<Vec<u8>, BindingsError> {
    Ok(crypto::encode_destination(&address, network.into())?.encode())
}

/// Builds a classic multisig challenge blob from a list of public keys.
/// `minRequiredSignatures` must be at least one and at most the number of
/// keys.
#[uniffi::export]
pub fn encode_multisig_challenge(
    public_keys: Vec<Vec<u8>>,
    min_required_signatures: u8,
    network: Network,
) -> Result<Vec<u8>, BindingsError> {
    let keys = public_keys
        .into_iter()
        .map(|blob| PublicKey::decode_all(&mut &blob[..]).map_err(decoding))
        .collect::<Result<Vec<_>, _>>()?;
    let challenge =
        crypto::encode_multisig_challenge(&keys, min_required_signatures, network.into())?;
    Ok(challenge.encode())
}

/// Returns the bech32 address of a multisig challenge.
#[uniffi::export]
pub fn multisig_challenge_to_address(
    multisig_challenge: Vec<u8>,
    network: Network,
) -> Result<String, BindingsError> {
    let challenge =
        ClassicMultisigChallenge::decode_all(&mut &multisig_challenge[..]).map_err(decoding)?;
    Ok(crypto::multisig_challenge_to_address(
        &challenge,
        network.into(),
    ))
}
