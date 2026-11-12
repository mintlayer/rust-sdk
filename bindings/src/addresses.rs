// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Address encoding, mirroring the wasm-bindings exports.

use mintlayer_sdk::crypto;
use mintlayer_sdk::crypto::types::{DecodeAll, PublicKey};

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
