// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! UniFFI bindings exposing the Mintlayer wallet primitives to Kotlin and
//! Swift.
//!
//! The exported surface mirrors the `wasm-bindings` exports
//! (same names, same parameter shapes, same runtime blob formats),
//! so a consumer that used the WASM wrappers can move to the native bindings
//! without changing its key/address handling. Keys and encoded objects cross
//! the FFI boundary as byte arrays in the runtime SCALE format; addresses and
//! other identifiers as strings.
//!
//! All fallible operations return [`BindingsError`] — nothing panics across
//! the FFI boundary. There is no I/O, no storage and no networking here:
//! this is a pure cryptography and encoding library.
//!
//! # Secret material lifetime
//!
//! The BIP39 seed and passphrase are zeroized on drop, but the key blobs
//! returned over the FFI boundary are plain buffers that the foreign
//! runtime copies into its own memory; they cannot be zeroized from the
//! Rust side once returned (the WASM wrappers carry the same caveat).
//! Keep the lifetime of secret blobs short on the foreign side and never
//! log them.

pub mod addresses;
pub mod error;
pub mod fees;
pub mod ids;
pub mod inputs;
pub mod intents;
pub mod keys;
pub mod network;
pub mod outputs;
pub mod signing;
pub mod staking;
pub mod timelocks;
pub mod transactions;
pub mod types;

pub use addresses::{
    encode_destination, encode_multisig_challenge, multisig_challenge_to_address,
    pubkey_to_pubkeyhash_address,
};
pub use error::BindingsError;
pub use fees::{
    data_deposit_fee, fungible_token_issuance_fee, nft_issuance_fee, token_change_authority_fee,
    token_freeze_fee, token_supply_change_fee,
};
pub use ids::{get_delegation_id, get_order_id, get_pool_id, get_token_id};
pub use inputs::{
    encode_input_for_change_token_authority, encode_input_for_change_token_metadata_uri,
    encode_input_for_conclude_order, encode_input_for_fill_order, encode_input_for_freeze_order,
    encode_input_for_freeze_token, encode_input_for_lock_token_supply,
    encode_input_for_mint_tokens, encode_input_for_unfreeze_token, encode_input_for_unmint_tokens,
    encode_input_for_utxo, encode_input_for_withdraw_from_delegation,
};
pub use intents::{
    encode_signed_transaction_intent, make_transaction_intent_message_to_sign,
    verify_transaction_intent,
};
pub use keys::{
    ExtendedPrivateKeyData, extended_private_key_decode, extended_private_key_encode,
    extended_public_key_from_extended_private_key, make_change_address,
    make_change_address_public_key, make_default_account_privkey, make_private_key,
    make_receiving_address, make_receiving_address_public_key, public_key_from_private_key,
};
pub use network::Network;
pub use outputs::{
    encode_create_order_output, encode_output_coin_burn, encode_output_create_delegation,
    encode_output_create_stake_pool, encode_output_data_deposit, encode_output_delegate_staking,
    encode_output_htlc, encode_output_issue_fungible_token, encode_output_issue_nft,
    encode_output_lock_then_transfer, encode_output_produce_block_from_stake,
    encode_output_token_burn, encode_output_token_lock_then_transfer, encode_output_token_transfer,
    encode_output_transfer,
};
pub use signing::{
    encode_witness, encode_witness_htlc_refund_multisig, encode_witness_htlc_refund_single_sig,
    encode_witness_htlc_spend, encode_witness_no_signature, sign_challenge,
    sign_message_for_spending, verify_challenge, verify_signature_for_spending,
};
pub use staking::{
    effective_pool_balance, encode_stake_pool_data, staking_pool_spend_maturity_block_count,
};
pub use timelocks::{
    encode_lock_for_block_count, encode_lock_for_seconds, encode_lock_until_height,
    encode_lock_until_time,
};
pub use transactions::{
    decode_partially_signed_transaction_to_json, decode_signed_transaction_to_json,
    decode_transaction_to_json, encode_outpoint_source_id, encode_partially_signed_transaction,
    encode_signed_transaction, encode_transaction, estimate_transaction_size, extract_htlc_secret,
    get_transaction_id,
};
pub use types::{
    FreezableToken, OrderAdditionalInfoEntry, PoolAdditionalInfoEntry, SignatureHashType,
    SimpleCurrencyAmount, SourceId, TokenUnfreezable, TotalSupply, TxAdditionalInfo,
};

uniffi::setup_scaffolding!("mintlayer_wallet_bindings");
