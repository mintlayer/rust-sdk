// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! UniFFI bindings exposing the Mintlayer wallet primitives to Kotlin and
//! Swift.
//!
//! The exported surface mirrors the `wasm-bindings` exports operation for
//! operation (same names, same parameter shapes, same runtime blob formats),
//! so a consumer that used the WASM wrappers can move to the native bindings
//! without changing its key/address handling. Keys and encoded objects cross
//! the FFI boundary as byte arrays in the runtime SCALE format; addresses and
//! other identifiers as strings.
//!
//! All fallible operations return [`BindingsError`] — nothing panics across
//! the FFI boundary. There is no I/O, no storage and no networking here:
//! this is a pure cryptography and encoding library.

pub mod addresses;
pub mod error;
pub mod keys;
pub mod network;

pub use addresses::pubkey_to_pubkeyhash_address;
pub use error::BindingsError;
pub use keys::{
    ExtendedPrivateKeyData, extended_private_key_decode, extended_private_key_encode,
    extended_public_key_from_extended_private_key, make_change_address,
    make_change_address_public_key, make_default_account_privkey, make_private_key,
    make_receiving_address, make_receiving_address_public_key, public_key_from_private_key,
};
pub use network::Network;

uniffi::setup_scaffolding!("mintlayer_wallet_bindings");
