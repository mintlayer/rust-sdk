// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wire types for the indexer (api-web-server) REST API.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An unsigned 64-bit integer that the indexer may encode as a JSON integer
/// or as a decimal string.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uint64(pub u64);

impl fmt::Display for Uint64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Uint64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::number::u64_lenient::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for Uint64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::number::u64_lenient::deserialize(deserializer).map(Self)
    }
}

/// A per-mille ratio (e.g. a pool margin ratio) that the indexer may encode
/// as a number or as a decimal string with an optional trailing `%`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PerThousand(pub f64);

impl fmt::Display for PerThousand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for PerThousand {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::number::f64_percent::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for PerThousand {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::number::f64_percent::deserialize(deserializer).map(Self)
    }
}

/// A coin or token amount as reported by the indexer, with the atom value
/// and the human-readable decimal representation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    /// Amount in atoms, the smallest indivisible unit
    /// (1 ML = 100,000,000,000 atoms).
    #[serde(with = "crate::number::atoms")]
    pub atoms: u128,
    /// The decimal representation in whole coins.
    pub decimal: String,
}

/// A UNIX timestamp in seconds.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Timestamp {
    /// Seconds since the UNIX epoch.
    pub timestamp: i64,
}

/// The tip of the main chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainTip {
    /// Height of the best block.
    pub block_height: u64,
    /// Hex-encoded id of the best block.
    pub block_id: String,
}

/// The genesis block and consensus message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenesisInfo {
    /// Hex-encoded genesis block id.
    pub block_id: String,
    /// The signed genesis message.
    pub genesis_message: String,
    /// Genesis timestamp.
    pub timestamp: Timestamp,
    /// The initial UTXO set.
    pub utxos: serde_json::Value,
}

/// A block header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Hex-encoded id of the parent block.
    pub previous_block_id: String,
    /// Header timestamp.
    pub timestamp: Timestamp,
    /// Hex-encoded merkle root of transaction ids.
    pub merkle_root: String,
    /// Hex-encoded merkle root of witness merkle roots.
    pub witness_merkle_root: String,
    /// Consensus data payload (PoS or PoW specific).
    pub consensus_data: serde_json::Value,
}

/// A block body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockBody {
    /// The block reward outputs.
    pub reward: Vec<serde_json::Value>,
    /// The transactions included in the block.
    pub transactions: Vec<Transaction>,
}

/// A block with header and body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Height of the block.
    pub height: u64,
    /// The block header.
    pub header: BlockHeader,
    /// The block body.
    pub body: BlockBody,
}

/// A transaction as reported by the indexer.
///
/// [`block_id`](#structfield.block_id), [`timestamp`](#structfield.timestamp)
/// and [`confirmations`](#structfield.confirmations) are empty strings while
/// the transaction is unconfirmed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    /// Hex-encoded transaction id.
    pub id: String,
    /// The transaction inputs.
    pub inputs: serde_json::Value,
    /// The transaction outputs.
    pub outputs: serde_json::Value,
    /// Hex-encoded id of the confirming block, empty when unconfirmed.
    pub block_id: String,
    /// Confirmation timestamp, empty when unconfirmed.
    pub timestamp: String,
    /// Number of confirmations, empty when unconfirmed.
    pub confirmations: String,
}

/// The merkle path of a confirmed transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerklePath {
    /// Hex-encoded id of the confirming block.
    pub block_id: String,
    /// Index of the transaction in the block.
    pub transaction_index: u32,
    /// Hex-encoded merkle root of the block.
    pub merkle_root: String,
    /// The hashes along the path from the transaction to the root.
    pub path: Vec<String>,
}

/// A token balance of an address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalance {
    /// Bech32 token id.
    pub token_id: String,
    /// The token amount.
    pub amount: Amount,
}

/// Aggregated information about an address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressInfo {
    /// Spendable coin balance.
    pub coin_balance: Amount,
    /// Coin balance locked by timelocks.
    pub locked_coin_balance: Amount,
    /// Hex-encoded ids of transactions involving the address.
    pub transaction_history: Vec<String>,
    /// Token balances held by the address.
    pub tokens: Vec<TokenBalance>,
}

/// A flat outpoint as reported by the indexer (`source_id` is a hex string).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UtxoOutpoint {
    /// Hex-encoded transaction id or block id.
    pub source_id: String,
    /// Index of the output in the source.
    pub index: u32,
}

/// An unspent output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Utxo {
    /// The outpoint referencing the output.
    pub outpoint: UtxoOutpoint,
    /// The output itself.
    #[serde(rename = "utxo")]
    pub output: serde_json::Value,
}

/// Delegation summary as seen from an address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationInfo {
    /// Bech32 delegation id.
    pub delegation_id: String,
    /// Bech32 pool id.
    pub pool_id: String,
    /// The next expected account nonce.
    pub next_nonce: Uint64,
    /// The destination that receives delegation withdrawals.
    pub spend_destination: String,
    /// The delegated balance.
    pub balance: Amount,
}

/// A stake pool as reported by the indexer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pool {
    /// Bech32 pool id.
    pub pool_id: String,
    /// The destination receiving decommission outputs.
    pub decommission_destination: String,
    /// The staker's own pledge balance.
    pub staker_balance: Amount,
    /// The pool margin ratio in per-mille.
    pub margin_ratio_per_thousand: PerThousand,
    /// The pool cost per block.
    pub cost_per_block: Amount,
    /// Bech32 VRF public key of the pool.
    pub vrf_public_key: String,
    /// The balance delegated to the pool.
    pub delegations_balance: Amount,
}

/// A delegation as reported by the indexer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delegation {
    /// Bech32 delegation id.
    pub delegation_id: String,
    /// Bech32 pool id.
    pub pool_id: String,
    /// The next expected account nonce.
    pub next_nonce: Uint64,
    /// The destination that receives delegation withdrawals.
    pub spend_destination: String,
    /// The delegated balance.
    pub balance: Amount,
    /// Height of the block that created the delegation.
    pub creation_block_height: Uint64,
}

/// A delegation of a pool as reported by the indexer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolDelegation {
    /// Bech32 delegation id.
    pub delegation_id: String,
    /// The next expected account nonce.
    pub next_nonce: Uint64,
    /// The destination that receives delegation withdrawals.
    pub spend_destination: String,
    /// The delegated balance.
    pub balance: Amount,
    /// Height of the block that created the delegation.
    pub creation_block_height: Uint64,
}

/// A fungible token or NFT collection as reported by the indexer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Bech32 authority address of the token.
    pub authority: String,
    /// Whether the token supply is locked.
    pub is_locked: bool,
    /// The circulating supply.
    pub circulating_supply: Amount,
    /// The token ticker.
    pub token_ticker: String,
    /// The metadata URI.
    pub metadata_uri: String,
    /// Number of decimal places.
    pub number_of_decimals: u8,
    /// The total supply variant.
    pub total_supply: serde_json::Value,
    /// Whether the token is frozen.
    pub frozen: bool,
    /// Whether a frozen token can be unfrozen; present when frozen.
    pub is_token_unfreezable: Option<bool>,
    /// Whether the token can be frozen; present when not frozen.
    pub is_token_freezable: Option<bool>,
    /// The next expected account nonce.
    pub next_nonce: Uint64,
}

/// A token transfer tracked by the indexer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenTx {
    /// Global index of the transaction in the token history.
    pub tx_global_index: u64,
    /// Hex-encoded transaction id.
    pub tx_id: String,
}

/// NFT metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NftMetadata {
    /// The creator's public key, if provided.
    pub creator: Option<String>,
    /// NFT name.
    pub name: String,
    /// NFT description.
    pub description: String,
    /// NFT ticker.
    pub ticker: String,
    /// Icon URI, if provided.
    pub icon_uri: Option<String>,
    /// Additional metadata URI, if provided.
    pub additional_metadata_uri: Option<String>,
    /// Media URI, if provided.
    pub media_uri: Option<String>,
    /// Hex-encoded hash of the media.
    pub media_hash: String,
}

/// An NFT as reported by the indexer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NftInfo {
    /// Bech32 owner address.
    pub owner: String,
    /// Bech32 token id.
    pub token_id: String,
    /// The NFT metadata.
    pub metadata: NftMetadata,
}

/// A DEX order as reported by the indexer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    /// Bech32 order id.
    pub order_id: String,
    /// The destination that may conclude the order.
    pub conclude_destination: String,
    /// The currency initially given.
    pub give_currency: serde_json::Value,
    /// The amount initially given.
    pub initially_given: Amount,
    /// Remaining give balance.
    pub give_balance: Amount,
    /// The currency initially asked.
    pub ask_currency: serde_json::Value,
    /// The amount initially asked.
    pub initially_asked: Amount,
    /// Remaining ask balance.
    pub ask_balance: Amount,
    /// The order account nonce.
    pub nonce: Uint64,
}

/// Global coin statistics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoinStats {
    /// The circulating supply.
    pub circulating_supply: Amount,
    /// The preminted supply.
    pub preminted: Amount,
    /// The total burned amount.
    pub burned: Amount,
    /// The total staked amount.
    pub staked: Amount,
}

/// Pagination parameters shared by list endpoints.
///
/// The indexer defaults to offset 0 and 10 items per page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PageOpts {
    /// Number of items to skip.
    pub offset: u32,
    /// Number of items to return.
    pub items: u32,
}

/// Sort order for pool listings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PoolSort {
    /// Sort by creation height (indexer default).
    #[default]
    ByHeight,
    /// Sort by pledge amount.
    ByPledge,
}

impl PoolSort {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ByHeight => "by_height",
            Self::ByPledge => "by_pledge",
        }
    }
}

/// Parameters for listing stake pools.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PoolListOpts {
    /// Number of pools to skip.
    pub offset: u32,
    /// Number of pools to return.
    pub items: u32,
    /// Sort order; defaults to [`PoolSort::ByHeight`] on the server.
    pub sort: Option<PoolSort>,
}
