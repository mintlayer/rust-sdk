// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Output timelock constructors. The returned blobs are passed to
//! [`encode_output_lock_then_transfer`](super::outputs::encode_output_lock_then_transfer)
//! and [`encode_output_htlc`](super::outputs::encode_output_htlc).

use mintlayer_sdk::crypto;
use mintlayer_sdk::crypto::types::Encode;

/// Locks an output for a number of blocks after it is included in a block.
#[uniffi::export]
pub fn encode_lock_for_block_count(block_count: u64) -> Vec<u8> {
    crypto::encode_lock_for_block_count(block_count).encode()
}

/// Locks an output for a number of seconds after it is included in a block.
#[uniffi::export]
pub fn encode_lock_for_seconds(total_seconds: u64) -> Vec<u8> {
    crypto::encode_lock_for_seconds(total_seconds).encode()
}

/// Locks an output until a UNIX timestamp (in seconds).
#[uniffi::export]
pub fn encode_lock_until_time(timestamp_since_epoch_in_seconds: u64) -> Vec<u8> {
    crypto::encode_lock_until_time(timestamp_since_epoch_in_seconds).encode()
}

/// Locks an output until a block height.
#[uniffi::export]
pub fn encode_lock_until_height(block_height: u64) -> Vec<u8> {
    crypto::encode_lock_until_height(block_height).encode()
}
