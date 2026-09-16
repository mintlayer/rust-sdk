// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wire types for the wallet daemon JSON-RPC API.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A coin or token amount. Requests typically set only [`atoms`](#structfield.atoms);
/// daemon responses carry both fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Amount {
    /// Amount in atoms, the smallest indivisible unit
    /// (1 ML = 100,000,000,000 atoms).
    atoms: Option<u128>,
    /// The decimal representation in whole coins.
    decimal: Option<String>,
}

impl Amount {
    /// Creates an amount from a number of atoms.
    #[must_use]
    pub fn from_atoms(atoms: u128) -> Self {
        Self {
            atoms: Some(atoms),
            decimal: None,
        }
    }

    /// Creates an amount from a decimal string of atoms and an optional
    /// decimal representation.
    #[must_use]
    pub fn from_parts(atoms: Option<u128>, decimal: Option<String>) -> Self {
        Self { atoms, decimal }
    }

    /// Returns the amount in atoms, if known.
    #[must_use]
    pub const fn atoms(&self) -> Option<u128> {
        self.atoms
    }

    /// Returns the decimal representation, if known.
    #[must_use]
    pub fn decimal(&self) -> Option<&str> {
        self.decimal.as_deref()
    }
}

impl Serialize for Amount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Amount", 2)?;
        match self.atoms {
            Some(atoms) => state.serialize_field("atoms", &atoms.to_string())?,
            None => state.skip_field("atoms")?,
        }
        match &self.decimal {
            Some(decimal) => state.serialize_field("decimal", decimal)?,
            None => state.skip_field("decimal")?,
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(
                default,
                skip_serializing_if = "Option::is_none",
                with = "crate::number::option_atoms_lenient"
            )]
            atoms: Option<u128>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            decimal: Option<String>,
        }
        Wire::deserialize(deserializer).map(|wire| Self {
            atoms: wire.atoms,
            decimal: wire.decimal,
        })
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

/// Fee estimation for a transaction, split by currency.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeesBreakdown {
    /// The fee paid in coins.
    pub coins: Amount,
    /// The fee paid in tokens, keyed by bech32 token id.
    pub tokens: BTreeMap<String, Amount>,
}

/// The result of a wallet send operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendResult {
    /// Hex-encoded transaction id.
    pub tx_id: String,
    /// The fees paid.
    pub fees: FeesBreakdown,
    /// Whether the transaction was broadcast to the network.
    pub broadcasted: bool,
}

/// The result of submitting a transaction to the node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitResult {
    /// Hex-encoded transaction id.
    pub tx_id: String,
}

/// A composed (unsigned) transaction and its estimated fees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComposedTx {
    /// Hex-encoded partially signed transaction.
    pub hex: String,
    /// The estimated fees.
    pub fees: FeesBreakdown,
}

/// A signed raw transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedTx {
    /// Hex-encoded signed transaction.
    pub hex: String,
    /// The signatures collected so far.
    pub current_signatures: serde_json::Value,
}

/// Signature statistics of a transaction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxStats {
    /// Number of inputs.
    pub num_inputs: u32,
    /// Number of total signatures required.
    pub total_signatures: u32,
}

/// An inspection of a transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TxInspection {
    /// Signature statistics.
    pub stats: TxStats,
    /// The fees, if computable.
    pub fees: Option<FeesBreakdown>,
}

/// A wallet transaction history entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletTx {
    /// Hex-encoded transaction id.
    pub id: String,
    /// Height of the confirming block.
    pub height: u64,
    /// Confirmation timestamp.
    pub timestamp: Timestamp,
}

/// A mnemonic as returned by the wallet daemon.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MnemonicContent {
    /// The seed phrase words.
    pub mnemonic: String,
}

impl std::fmt::Debug for MnemonicContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MnemonicContent").field("mnemonic", &"<redacted>").finish()
    }
}

/// The mnemonic section of a [`CreateWalletResult`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MnemonicResult {
    /// Either `"NewlyGenerated"` or `"UserProvided"`.
    #[serde(rename = "type")]
    pub kind: String,
    /// The mnemonic, when the daemon returned one.
    pub content: Option<MnemonicContent>,
}

/// The result of creating a wallet.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateWalletResult {
    /// The generated mnemonic, when the daemon generated one.
    pub mnemonic: Option<MnemonicResult>,
}

/// Parameters for [`Client::create_wallet`](crate::wallet::Client::create_wallet).
#[derive(Clone, Default, PartialEq, Eq, Serialize)]
pub struct CreateWalletParams {
    /// File path for the wallet database.
    pub path: String,
    /// Whether to store the seed phrase in the wallet file.
    pub store_seed_phrase: bool,
    /// A user-provided mnemonic; `None` lets the daemon generate one.
    pub mnemonic: Option<String>,
    /// Optional BIP39 passphrase.
    pub passphrase: Option<String>,
    /// Optional hardware wallet type.
    pub hardware_wallet: Option<String>,
}

impl std::fmt::Debug for CreateWalletParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateWalletParams")
            .field("path", &self.path)
            .field("store_seed_phrase", &self.store_seed_phrase)
            .field("mnemonic", &self.mnemonic.as_ref().map(|_| "***"))
            .field("passphrase", &self.passphrase.as_ref().map(|_| "***"))
            .field("hardware_wallet", &self.hardware_wallet)
            .finish()
    }
}

/// Parameters for [`Client::recover_wallet`](crate::wallet::Client::recover_wallet).
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct RecoverWalletParams {
    /// File path for the wallet database.
    pub path: String,
    /// Whether to store the seed phrase in the wallet file.
    pub store_seed_phrase: bool,
    /// The mnemonic to recover from.
    pub mnemonic: String,
    /// Optional BIP39 passphrase.
    pub passphrase: Option<String>,
    /// Optional hardware wallet type.
    pub hardware_wallet: Option<String>,
}

impl std::fmt::Debug for RecoverWalletParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecoverWalletParams")
            .field("path", &self.path)
            .field("store_seed_phrase", &self.store_seed_phrase)
            .field("mnemonic", &"***")
            .field("passphrase", &self.passphrase.as_ref().map(|_| "***"))
            .field("hardware_wallet", &self.hardware_wallet)
            .finish()
    }
}

/// Extra wallet information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletExtraInfo {
    /// Either `"SoftwareWallet"`, `"TrezorWallet"` or `"LedgerWallet"`.
    #[serde(rename = "type")]
    pub kind: String,
}

/// General information about the open wallet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletInfo {
    /// The wallet identifier.
    pub wallet_id: String,
    /// Names of the accounts in the wallet.
    pub account_names: Vec<String>,
    /// Extra wallet information.
    pub extra_info: WalletExtraInfo,
}

/// The best block known to the wallet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BestBlock {
    /// Height of the best block.
    pub height: u64,
    /// Hex-encoded id of the best block.
    pub id: String,
}

/// A created account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountInfo {
    /// The account index.
    pub account: u32,
    /// The account name.
    pub name: String,
}

/// The balance of an account.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Balance {
    /// The coin balance.
    pub coins: Amount,
    /// Token balances keyed by bech32 token id.
    pub tokens: BTreeMap<String, Amount>,
}

/// An address with usage information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressWithUsage {
    /// The bech32 address.
    pub address: String,
    /// Whether the address has been used.
    pub used: bool,
    /// The balance held by the address.
    pub coins: Amount,
}

/// The result of revealing the public key of an address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevealPublicKey {
    /// Hex-encoded compressed public key.
    pub public_key_hex: String,
    /// Bech32 address of the public key.
    pub public_key_address: String,
}

/// A reference to a transaction output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Outpoint {
    /// Where the spent output comes from.
    pub source_id: OutpointSourceId,
    /// The index of the output in the source.
    pub index: u32,
}

/// The source of an [`Outpoint`].
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

/// Options applied to wallet transaction creation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct TxOptions {
    /// Target the top X MB of the mempool for fee estimation.
    pub in_top_x_mb: Option<u32>,
    /// Whether to broadcast the transaction to the mempool.
    pub broadcast_to_mempool: Option<bool>,
}

/// Parameters for [`Client::send`](crate::wallet::Client::send).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SendParams {
    /// The account to spend from.
    pub account: u32,
    /// The destination address.
    pub address: String,
    /// The amount to send.
    pub amount: Amount,
    /// UTXOs to spend; an empty vector lets the wallet select.
    pub selected_utxos: Vec<Outpoint>,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::send_token`](crate::wallet::Client::send_token).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TokenSendParams {
    /// The account to spend from.
    pub account: u32,
    /// The bech32 token id.
    pub token_id: String,
    /// The destination address.
    pub address: String,
    /// The amount to send.
    pub amount: Amount,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::sweep_spendable`](crate::wallet::Client::sweep_spendable).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SweepParams {
    /// The account to spend from.
    pub account: u32,
    /// The destination address.
    pub destination_address: String,
    /// Addresses whose funds are swept.
    pub from_addresses: Vec<String>,
    /// Whether to sweep all addresses of the account.
    pub all: bool,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::spend_utxo`](crate::wallet::Client::spend_utxo).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UtxoSpendParams {
    /// The account to spend from.
    pub account: u32,
    /// The UTXO to spend.
    pub utxo: Outpoint,
    /// The destination address.
    pub output_address: String,
    /// The HTLC secret, when spending an HTLC output.
    pub htlc_secret: Option<String>,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::compose_transaction`](crate::wallet::Client::compose_transaction).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComposeParams {
    /// The inputs to spend.
    pub inputs: Vec<Outpoint>,
    /// The outputs to create.
    pub outputs: Vec<serde_json::Value>,
    /// HTLC secrets for HTLC inputs.
    pub htlc_secrets: Option<serde_json::Value>,
    /// Whether to return only the transaction instead of a partial one.
    pub only_transaction: bool,
}

/// A source or destination currency for orders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum CurrencyFilter {
    /// The native coin.
    Coin,
    /// A token identified by its bech32 token id.
    Token(String),
}

/// One side of a DEX order: a currency with an amount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum OutputValue {
    /// The native coin.
    Coin {
        /// The coin amount.
        amount: Amount,
    },
    /// A token identified by its bech32 id.
    Token {
        /// The bech32 token id.
        id: String,
        /// The token amount.
        amount: Amount,
    },
}

impl OutputValue {
    /// Creates a coin side from a number of atoms.
    #[must_use]
    pub fn coins(atoms: u128) -> Self {
        Self::Coin {
            amount: Amount::from_atoms(atoms),
        }
    }

    /// Creates a token side from a token id and a number of atoms.
    #[must_use]
    pub fn token(token_id: impl Into<String>, atoms: u128) -> Self {
        Self::Token {
            id: token_id.into(),
            amount: Amount::from_atoms(atoms),
        }
    }
}

/// Parameters for [`Client::create_order`](crate::wallet::Client::create_order).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CreateOrderParams {
    /// The account to spend from.
    pub account: u32,
    /// The currency and amount asked.
    pub ask: OutputValue,
    /// The currency and amount given.
    pub give: OutputValue,
    /// The address allowed to conclude the order.
    pub conclude_address: String,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::conclude_order`](crate::wallet::Client::conclude_order).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConcludeOrderParams {
    /// The account to spend from.
    pub account: u32,
    /// The bech32 order id.
    pub order_id: String,
    /// The address receiving the order funds; `None` uses the default.
    pub output_address: Option<String>,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::fill_order`](crate::wallet::Client::fill_order).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FillOrderParams {
    /// The account to spend from.
    pub account: u32,
    /// The bech32 order id.
    pub order_id: String,
    /// The amount to fill, in the ask currency.
    pub fill_amount_in_ask_currency: Amount,
    /// The address receiving the order funds; `None` uses the default.
    pub output_address: Option<String>,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::freeze_order`](crate::wallet::Client::freeze_order).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FreezeOrderParams {
    /// The account to spend from.
    pub account: u32,
    /// The bech32 order id.
    pub order_id: String,
    /// Transaction options.
    pub options: TxOptions,
}

/// Filters for listing active orders.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ListOrdersParams {
    /// The account to list orders for.
    pub account: u32,
    /// Filter by ask currency; `None` includes every currency.
    pub ask_currency: Option<CurrencyFilter>,
    /// Filter by give currency; `None` includes every currency.
    pub give_currency: Option<CurrencyFilter>,
}

/// The live state of an own order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderState {
    /// Remaining ask balance.
    pub ask_balance: Amount,
    /// Remaining give balance.
    pub give_balance: Amount,
    /// Whether the order is frozen.
    #[serde(rename = "is_frozen")]
    pub frozen: bool,
    /// When the order was created.
    #[serde(rename = "creation_timestamp")]
    pub creation: Timestamp,
}

/// An order owned by the wallet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnOrder {
    /// Bech32 order id.
    pub order_id: String,
    /// The currency and amount initially asked.
    pub initially_asked: OutputValue,
    /// The currency and amount initially given.
    pub initially_given: OutputValue,
    /// The live order state, when the order is active on chain.
    pub existing_order_data: Option<OrderState>,
    /// Whether the order is marked as frozen in the wallet.
    #[serde(rename = "is_marked_as_frozen_in_wallet")]
    pub is_marked_as_frozen_in_wallet: bool,
    /// Whether the order is marked as concluded in the wallet.
    #[serde(rename = "is_marked_as_concluded_in_wallet")]
    pub is_marked_as_concluded_in_wallet: bool,
}

/// An active order on chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveOrder {
    /// Bech32 order id.
    pub order_id: String,
    /// The currency and amount initially asked.
    pub initially_asked: OutputValue,
    /// The currency and amount initially given.
    pub initially_given: OutputValue,
    /// Remaining ask balance.
    pub ask_balance: Amount,
    /// Remaining give balance.
    pub give_balance: Amount,
    /// Whether the order belongs to this wallet.
    pub is_own: bool,
}

/// The result of creating an order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderCreated {
    /// Bech32 order id.
    pub order_id: String,
    /// Hex-encoded transaction id.
    pub tx_id: String,
    /// Whether the order transaction was broadcast.
    pub broadcasted: bool,
}

/// Parameters for [`Client::create_stake_pool`](crate::wallet::Client::create_stake_pool).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CreatePoolParams {
    /// The account to spend from.
    pub account: u32,
    /// The pledge amount.
    pub amount: Amount,
    /// The pool cost per block.
    pub cost_per_block: Amount,
    /// The margin ratio in per-mille, as a decimal string.
    pub margin_ratio_per_thousand: String,
    /// The address receiving decommission outputs.
    pub decommission_address: String,
    /// The staker address; `None` uses a derived address.
    pub staker_address: Option<String>,
    /// The bech32 VRF public key; `None` uses a derived key.
    pub vrf_public_key: Option<String>,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::decommission_stake_pool`](crate::wallet::Client::decommission_stake_pool).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DecommissionParams {
    /// The account to spend from.
    pub account: u32,
    /// The bech32 pool id.
    pub pool_id: String,
    /// The address receiving the pool funds.
    pub output_address: String,
    /// Transaction options.
    pub options: TxOptions,
}

/// A stake pool owned by the wallet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedPool {
    /// Bech32 pool id.
    pub pool_id: String,
    /// The pledge amount.
    pub pledge: Amount,
    /// The total pool balance.
    pub balance: Amount,
    /// The margin ratio in per-mille, as a decimal string.
    pub margin_ratio_per_thousand: String,
    /// The pool cost per block.
    pub cost_per_block: Amount,
}

/// Whether an account is staking.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StakingStatus {
    /// The account is staking.
    #[serde(rename = "Staking")]
    #[default]
    Staking,
    /// The account is not staking.
    #[serde(rename = "NotStaking")]
    NotStaking,
}

/// Parameters for [`Client::create_delegation`](crate::wallet::Client::create_delegation).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CreateDelegationParams {
    /// The account to spend from.
    pub account: u32,
    /// The address owning the delegation.
    pub address: String,
    /// The bech32 pool id to delegate to.
    pub pool_id: String,
    /// Transaction options.
    pub options: TxOptions,
}

/// The result of creating a delegation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateDelegationResult {
    /// Bech32 delegation id.
    pub delegation_id: String,
    /// Hex-encoded transaction id.
    pub tx_id: String,
}

/// Parameters for [`Client::delegate_staking`](crate::wallet::Client::delegate_staking).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DelegateParams {
    /// The account to spend from.
    pub account: u32,
    /// The amount to delegate.
    pub amount: Amount,
    /// The bech32 delegation id.
    pub delegation_id: String,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::withdraw_from_delegation`](crate::wallet::Client::withdraw_from_delegation).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WithdrawParams {
    /// The account to spend from.
    pub account: u32,
    /// The address receiving the withdrawal.
    pub address: String,
    /// The amount to withdraw.
    pub amount: Amount,
    /// The bech32 delegation id.
    pub delegation_id: String,
    /// Transaction options.
    pub options: TxOptions,
}

/// A delegation owned by the wallet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationInfo {
    /// Bech32 delegation id.
    pub delegation_id: String,
    /// Bech32 pool id.
    pub pool_id: String,
    /// The delegated balance.
    pub balance: Amount,
}

/// The total supply policy of a token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum TokenSupply {
    /// A fixed supply capped at the given amount.
    Fixed(Amount),
    /// A lockable supply: minting can be stopped permanently.
    Lockable,
    /// An unlimited supply.
    Unlimited,
}

/// Metadata for a new fungible token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenMetadata {
    /// The token ticker.
    pub token_ticker: String,
    /// Number of decimal places.
    pub number_of_decimals: u8,
    /// The metadata URI.
    pub metadata_uri: String,
    /// The total supply policy.
    pub token_supply: TokenSupply,
    /// Whether the token can be frozen.
    pub is_freezable: bool,
}

/// Parameters for [`Client::issue_token`](crate::wallet::Client::issue_token).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IssueTokenParams {
    /// The account to spend from.
    pub account: u32,
    /// The address that becomes the token authority.
    pub destination_address: String,
    /// The token metadata.
    pub metadata: TokenMetadata,
    /// Transaction options.
    pub options: TxOptions,
}

/// The result of a token issuance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueTokenResult {
    /// Bech32 token id.
    pub token_id: String,
    /// Hex-encoded transaction id.
    pub tx_id: String,
}

/// Metadata for a new NFT.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct NftMetadata {
    /// Hex-encoded hash of the media.
    pub media_hash: String,
    /// The NFT name.
    pub name: String,
    /// The NFT description.
    pub description: String,
    /// The NFT ticker.
    pub ticker: String,
    /// The creator's public key, if provided.
    pub creator: Option<String>,
    /// Icon URI, if provided.
    pub icon_uri: Option<String>,
    /// Media URI, if provided.
    pub media_uri: Option<String>,
    /// Additional metadata URI, if provided.
    pub additional_metadata_uri: Option<String>,
}

/// Parameters for [`Client::issue_nft`](crate::wallet::Client::issue_nft).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IssueNftParams {
    /// The account to spend from.
    pub account: u32,
    /// The address that becomes the token authority.
    pub destination_address: String,
    /// The NFT metadata.
    pub metadata: NftMetadata,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::mint_tokens`](crate::wallet::Client::mint_tokens).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MintParams {
    /// The account to spend from.
    pub account: u32,
    /// The bech32 token id.
    pub token_id: String,
    /// The address receiving the minted tokens.
    pub address: String,
    /// The amount to mint.
    pub amount: Amount,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::unmint_tokens`](crate::wallet::Client::unmint_tokens).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UnmintParams {
    /// The account to spend from.
    pub account: u32,
    /// The bech32 token id.
    pub token_id: String,
    /// The amount to unmint.
    pub amount: Amount,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::lock_token_supply`](crate::wallet::Client::lock_token_supply).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LockSupplyParams {
    /// The account to spend from.
    pub account_index: u32,
    /// The bech32 token id.
    pub token_id: String,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::freeze_token`](crate::wallet::Client::freeze_token).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FreezeParams {
    /// The account to spend from.
    pub account: u32,
    /// The bech32 token id.
    pub token_id: String,
    /// Whether the freeze is permanent.
    pub is_unfreezable: bool,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::unfreeze_token`](crate::wallet::Client::unfreeze_token).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UnfreezeParams {
    /// The account to spend from.
    pub account: u32,
    /// The bech32 token id.
    pub token_id: String,
    /// Transaction options.
    pub options: TxOptions,
}

/// Parameters for [`Client::change_token_authority`](crate::wallet::Client::change_token_authority).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChangeAuthorityParams {
    /// The account to spend from.
    pub account: u32,
    /// The bech32 token id.
    pub token_id: String,
    /// The address of the new authority.
    pub address: String,
    /// Transaction options.
    pub options: TxOptions,
}
