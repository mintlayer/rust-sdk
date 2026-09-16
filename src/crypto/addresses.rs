// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Address and multisig-challenge encoding.

use std::num::NonZeroU8;

use ml_common as common;
use ml_crypto as crypto;

use common::address::pubkeyhash::PublicKeyHash;
use common::chain::{Destination, classic_multisig::ClassicMultisigChallenge};
use crypto::key::PublicKey;

use super::{Error, Network, parse_addressable};

/// Parses a bech32 address into a [`Destination`].
pub fn encode_destination(address: &str, network: Network) -> Result<Destination, Error> {
    parse_addressable(network, address)
}

/// Returns the bech32 pubkeyhash address of a public key.
pub fn pubkey_to_pubkeyhash_address(public_key: &PublicKey, network: Network) -> String {
    let public_key_hash = PublicKeyHash::from(public_key);
    common::address::Address::new(
        super::chain_config(network),
        Destination::PublicKeyHash(public_key_hash),
    )
    .expect("pubkeyhash address creation must not fail")
    .to_string()
}

/// Builds a classic multisig challenge from concatenated public keys.
pub fn encode_multisig_challenge(
    public_keys: &[PublicKey],
    min_required_signatures: u8,
    network: Network,
) -> Result<ClassicMultisigChallenge, Error> {
    let min_sigs =
        NonZeroU8::new(min_required_signatures).ok_or(Error::ZeroMultisigRequiredSignatures)?;
    ClassicMultisigChallenge::new(super::chain_config(network), min_sigs, public_keys.to_vec())
        .map_err(|error| Error::MessageSigning(error.to_string()))
}

/// Returns the bech32 address of a multisig challenge.
pub fn multisig_challenge_to_address(
    challenge: &ClassicMultisigChallenge,
    network: Network,
) -> String {
    let pkh = PublicKeyHash::from(challenge);
    common::address::Address::new(
        super::chain_config(network),
        Destination::ClassicMultisig(pkh),
    )
    .expect("multisig address creation must not fail")
    .to_string()
}
