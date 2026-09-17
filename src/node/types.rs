// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wire types for the node daemon JSON-RPC API.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Coin or token amount in atoms, the smallest indivisible unit
/// (1 ML = 100,000,000,000 atoms).
///
/// Serialized as a decimal string to avoid precision loss in JSON.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount(u128);

impl Amount {
    /// Creates an amount from a number of atoms.
    #[must_use]
    pub const fn from_atoms(atoms: u128) -> Self {
        Self(atoms)
    }

    /// The zero amount.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Returns the number of atoms.
    #[must_use]
    pub const fn atoms(self) -> u128 {
        self.0
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Amount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::number::atoms_object::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::number::atoms_object::deserialize(deserializer).map(Self)
    }
}

/// A UNIX timestamp in seconds.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Timestamp {
    /// Seconds since the UNIX epoch.
    pub timestamp: i64,
}

/// The source of a [`UtxoOutPoint`](crate::node::Outpoint).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum OutpointSourceId {
    /// The output of a transaction, keyed by its hex-encoded transaction id.
    Transaction {
        /// Hex-encoded transaction id.
        tx_id: String,
    },
    /// The block reward of a block, keyed by its hex-encoded block id.
    BlockReward {
        /// Hex-encoded block id.
        block_id: String,
    },
}

/// A reference to a specific transaction output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Outpoint {
    /// Where the spent output comes from.
    pub source_id: OutpointSourceId,
    /// The index of the output in the source transaction or block reward.
    pub index: u32,
}

/// Information about a token as stored in the chainstate.
///
/// The `content` payload is kept as raw JSON because its shape depends on
/// [`kind`](#structfield.kind) (`"FungibleToken"` or `"NonFungibleToken"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Either `"FungibleToken"` or `"NonFungibleToken"`.
    #[serde(rename = "type")]
    pub kind: String,
    /// The type-specific payload.
    pub content: serde_json::Value,
}

/// Information about a DEX order as stored in the chainstate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderInfo {
    /// Public key that may conclude the order.
    pub conclude_key: String,
    /// The currency and amount initially asked.
    pub initially_asked: serde_json::Value,
    /// The currency and amount initially given.
    pub initially_given: serde_json::Value,
    /// Remaining ask balance.
    pub ask_balance: Amount,
    /// Remaining give balance.
    pub give_balance: Amount,
    /// Account nonce; `None` while the order has no account spending history.
    #[serde(default, with = "crate::number::option_u64_lenient")]
    pub nonce: Option<u64>,
    /// Whether the order is frozen.
    pub is_frozen: bool,
}

/// A currency filter used to query orders.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "type", content = "content")]
pub enum Currency {
    /// The native coin.
    Coin,
    /// A token identified by its bech32 token id.
    Token(String),
}

/// The trust policy applied when submitting a transaction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
pub enum TrustPolicy {
    /// Accept the transaction only if fully valid against the current
    /// chainstate.
    #[default]
    Trusted,
    /// Accept the transaction even if some inputs are not yet known.
    Untrusted,
}

/// A transaction tracked by the mempool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MempoolTx {
    /// Hex-encoded transaction id.
    pub id: String,
    /// Pool status (`"InMempool"`, `"InMempoolDuplicate"`, `"InOrphanPool"`
    /// or `"InOrphanPoolDuplicate"`).
    pub status: String,
    /// Hex-encoded signed transaction bytes.
    pub transaction: String,
}

/// A fee rate in atoms per kilobyte.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeRate {
    /// Atoms per kilobyte.
    pub amount_per_kb: Amount,
}

/// One point of the mempool fee-rate estimate curve.
///
/// Serialized as a two-element array `[size, rate]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "(u64, FeeRate)", into = "(u64, FeeRate)")]
pub struct FeeRatePoint {
    /// Cumulative mempool size in bytes at which this rate applies.
    pub size: u64,
    /// The fee rate for that fill level.
    pub rate: FeeRate,
}

impl From<(u64, FeeRate)> for FeeRatePoint {
    fn from((size, rate): (u64, FeeRate)) -> Self {
        Self { size, rate }
    }
}

impl From<FeeRatePoint> for (u64, FeeRate) {
    fn from(point: FeeRatePoint) -> Self {
        (point.size, point.rate)
    }
}

/// Summary information about the chain tip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainstateInfo {
    /// Height of the best block.
    pub best_block_height: u64,
    /// Hex-encoded id of the best block.
    pub best_block_id: String,
    /// Timestamp of the best block.
    pub best_block_timestamp: Timestamp,
    /// Median timestamp of the last blocks.
    pub median_time: Timestamp,
    /// Whether the node is still in initial block download.
    pub is_initial_block_download: bool,
}

/// Details about a connected P2P peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer identifier assigned by the node.
    pub peer_id: u64,
    /// Remote address in `host:port` form.
    pub address: String,
    /// Peer role (`"Inbound"`, `"OutboundFullRelay"`, `"OutboundBlockRelay"`,
    /// `"OutboundReserved"`, `"OutboundManual"` or `"Feeler"`).
    pub peer_role: String,
    /// Current ban score.
    pub ban_score: u32,
    /// Peer user agent string.
    pub user_agent: String,
    /// Software version reported by the peer.
    pub software_version: String,
    /// Milliseconds the last ping took, if measured.
    pub ping_wait: Option<i64>,
    /// Milliseconds of the last completed ping, if any.
    pub ping_last: Option<i64>,
    /// Milliseconds of the fastest ping, if any.
    pub ping_min: Option<i64>,
    /// Timestamp of the peer's last known tip block, if reported.
    pub last_tip_block_time: Option<i64>,
}

/// When a peer was banned until.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BannedTime {
    /// Seconds component of the ban deadline.
    pub seconds: i64,
    /// Nanoseconds component of the ban deadline.
    pub nanos: i64,
}

/// A banned peer as reported by the node.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(from = "(String, BannedTimeWire)")]
pub struct BannedPeer {
    /// The banned address.
    pub address: String,
    /// When the ban expires.
    pub ban_time: BannedTime,
}

#[derive(Deserialize)]
struct BannedTimeWire {
    time: (i64, i64),
}

impl From<(String, BannedTimeWire)> for BannedPeer {
    fn from((address, wire): (String, BannedTimeWire)) -> Self {
        Self {
            address,
            ban_time: BannedTime {
                seconds: wire.time.0,
                nanos: wire.time.1,
            },
        }
    }
}
