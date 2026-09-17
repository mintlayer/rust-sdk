// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Output timelock constructors.

use ml_common as common;

use common::chain::{block::timestamp::BlockTimestamp, timelock::OutputTimeLock};

/// Locks an output for a number of blocks after it is included in a block.
pub fn encode_lock_for_block_count(block_count: u64) -> OutputTimeLock {
    OutputTimeLock::ForBlockCount(block_count)
}

/// Locks an output for a number of seconds after it is included in a block.
pub fn encode_lock_for_seconds(total_seconds: u64) -> OutputTimeLock {
    OutputTimeLock::ForSeconds(total_seconds)
}

/// Locks an output until a UNIX timestamp (in seconds).
pub fn encode_lock_until_time(timestamp_since_epoch_in_seconds: u64) -> OutputTimeLock {
    OutputTimeLock::UntilTime(BlockTimestamp::from_int_seconds(
        timestamp_since_epoch_in_seconds,
    ))
}

/// Locks an output until a block height.
pub fn encode_lock_until_height(block_height: u64) -> OutputTimeLock {
    OutputTimeLock::UntilHeight(common::primitives::BlockHeight::new(block_height))
}
