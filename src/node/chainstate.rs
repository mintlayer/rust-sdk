// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Chainstate RPC methods: best block, blocks, UTXOs, pools, tokens, orders.

use std::collections::BTreeMap;

use super::{Amount, ChainstateInfo, Currency, Error, OrderInfo, Outpoint, TokenInfo};
use crate::node::Client;

impl Client {
    /// Returns a summary of the current chain state (`chainstate_info`).
    pub async fn chainstate_info(&self) -> Result<ChainstateInfo, Error> {
        self.transport.call("chainstate_info", &serde_json::json!({})).await
    }

    /// Returns the hex-encoded id of the best block
    /// (`chainstate_best_block_id`).
    pub async fn best_block_id(&self) -> Result<String, Error> {
        self.transport.call("chainstate_best_block_id", &serde_json::json!({})).await
    }

    /// Returns the height of the best block (`chainstate_best_block_height`).
    pub async fn best_block_height(&self) -> Result<u64, Error> {
        self.transport
            .call("chainstate_best_block_height", &serde_json::json!({}))
            .await
    }

    /// Returns the block id at a given height, or `None` if the height is not
    /// part of the main chain (`chainstate_block_id_at_height`).
    pub async fn block_id_at_height(&self, height: u64) -> Result<Option<String>, Error> {
        self.transport
            .call(
                "chainstate_block_id_at_height",
                &serde_json::json!({ "height": height }),
            )
            .await
    }

    /// Returns the height of a block in the main chain, or `None` if the
    /// block is not part of it (`chainstate_block_height_in_main_chain`).
    pub async fn block_height_in_main_chain(&self, block_id: &str) -> Result<Option<u64>, Error> {
        self.transport
            .call(
                "chainstate_block_height_in_main_chain",
                &serde_json::json!({ "block_id": block_id }),
            )
            .await
    }

    /// Returns the hex-encoded serialization of a block, or `None` if it is
    /// unknown (`chainstate_get_block`). The genesis block cannot be
    /// retrieved.
    pub async fn block(&self, id: &str) -> Result<Option<String>, Error> {
        self.transport
            .call("chainstate_get_block", &serde_json::json!({ "id": id }))
            .await
    }

    /// Returns the JSON representation of a block (`chainstate_get_block_json`).
    pub async fn block_json(&self, id: &str) -> Result<serde_json::Value, Error> {
        self.transport
            .call(
                "chainstate_get_block_json",
                &serde_json::json!({ "id": id }),
            )
            .await
    }

    /// Returns up to `max_count` hex-encoded mainchain blocks starting at
    /// `from` (`chainstate_get_mainchain_blocks`).
    pub async fn mainchain_blocks(&self, from: u64, max_count: u32) -> Result<Vec<String>, Error> {
        self.transport
            .call(
                "chainstate_get_mainchain_blocks",
                &serde_json::json!({ "from": from, "max_count": max_count }),
            )
            .await
    }

    /// Returns the output at an outpoint, or `None` if it is spent or unknown
    /// (`chainstate_get_utxo`).
    pub async fn utxo(&self, outpoint: &Outpoint) -> Result<Option<serde_json::Value>, Error> {
        self.transport
            .call(
                "chainstate_get_utxo",
                &serde_json::json!({ "outpoint": outpoint }),
            )
            .await
    }

    /// Returns the balance of a stake pool (`chainstate_stake_pool_balance`).
    pub async fn stake_pool_balance(&self, pool_address: &str) -> Result<Option<Amount>, Error> {
        self.transport
            .call(
                "chainstate_stake_pool_balance",
                &serde_json::json!({ "pool_address": pool_address }),
            )
            .await
    }

    /// Returns the staker balance of a stake pool
    /// (`chainstate_staker_balance`).
    pub async fn staker_balance(&self, pool_address: &str) -> Result<Option<Amount>, Error> {
        self.transport
            .call(
                "chainstate_staker_balance",
                &serde_json::json!({ "pool_address": pool_address }),
            )
            .await
    }

    /// Returns the decommission destination of a stake pool
    /// (`chainstate_pool_decommission_destination`).
    pub async fn pool_decommission_destination(
        &self,
        pool_address: &str,
    ) -> Result<Option<String>, Error> {
        self.transport
            .call(
                "chainstate_pool_decommission_destination",
                &serde_json::json!({ "pool_address": pool_address }),
            )
            .await
    }

    /// Returns the share of a delegation in a pool
    /// (`chainstate_delegation_share`).
    pub async fn delegation_share(
        &self,
        pool_address: &str,
        delegation_address: &str,
    ) -> Result<Option<Amount>, Error> {
        self.transport
            .call(
                "chainstate_delegation_share",
                &serde_json::json!({
                    "pool_address": pool_address,
                    "delegation_address": delegation_address,
                }),
            )
            .await
    }

    /// Returns information about a token (`chainstate_token_info`).
    pub async fn token_info(&self, token_id: &str) -> Result<Option<TokenInfo>, Error> {
        self.transport
            .call(
                "chainstate_token_info",
                &serde_json::json!({ "token_id": token_id }),
            )
            .await
    }

    /// Returns information about multiple tokens, preserving order
    /// (`chainstate_tokens_info`).
    pub async fn tokens_info(&self, token_ids: &[String]) -> Result<Vec<TokenInfo>, Error> {
        self.transport
            .call(
                "chainstate_tokens_info",
                &serde_json::json!({ "token_ids": token_ids }),
            )
            .await
    }

    /// Returns information about a DEX order (`chainstate_order_info`).
    pub async fn order_info(&self, order_id: &str) -> Result<Option<OrderInfo>, Error> {
        self.transport
            .call(
                "chainstate_order_info",
                &serde_json::json!({ "order_id": order_id }),
            )
            .await
    }

    /// Returns all orders matching a pair of currencies; `None` filters
    /// include every currency (`chainstate_orders_info_by_currencies`).
    pub async fn orders_info_by_currencies(
        &self,
        ask_currency: Option<&Currency>,
        give_currency: Option<&Currency>,
    ) -> Result<BTreeMap<String, OrderInfo>, Error> {
        self.transport
            .call(
                "chainstate_orders_info_by_currencies",
                &serde_json::json!({
                    "ask_currency": ask_currency,
                    "give_currency": give_currency,
                }),
            )
            .await
    }

    /// Submits a hex-encoded block to the chainstate
    /// (`chainstate_submit_block`).
    pub async fn submit_block(&self, block_hex: &str) -> Result<(), Error> {
        self.transport
            .call(
                "chainstate_submit_block",
                &serde_json::json!({ "block_hex": block_hex }),
            )
            .await
    }
}
