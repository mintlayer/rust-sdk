// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Mempool RPC methods: transaction lookup, submission, fee rates.

use super::{Error, FeeRate, FeeRatePoint, MempoolTx, TrustPolicy};
use crate::node::Client;

impl Client {
    /// Returns whether the mempool contains a transaction with the given id
    /// (`mempool_contains_tx`).
    pub async fn contains_tx(&self, tx_id: &str) -> Result<bool, Error> {
        self.transport
            .call(
                "mempool_contains_tx",
                &serde_json::json!({ "tx_id": tx_id }),
            )
            .await
    }

    /// Returns whether the orphan pool contains a transaction with the given
    /// id (`mempool_contains_orphan_tx`).
    pub async fn contains_orphan_tx(&self, tx_id: &str) -> Result<bool, Error> {
        self.transport
            .call(
                "mempool_contains_orphan_tx",
                &serde_json::json!({ "tx_id": tx_id }),
            )
            .await
    }

    /// Returns a transaction from the mempool or orphan pool, or `None` if
    /// unknown (`mempool_get_transaction`).
    pub async fn transaction(&self, tx_id: &str) -> Result<Option<MempoolTx>, Error> {
        self.transport
            .call(
                "mempool_get_transaction",
                &serde_json::json!({ "tx_id": tx_id }),
            )
            .await
    }

    /// Submits a hex-encoded signed transaction to the local mempool without
    /// broadcasting it over P2P (`mempool_submit_transaction`).
    ///
    /// Use [`Client::broadcast_transaction`] to propagate the transaction to
    /// the network.
    pub async fn submit_transaction(
        &self,
        tx_hex: &str,
        trust_policy: TrustPolicy,
    ) -> Result<(), Error> {
        self.transport
            .call(
                "mempool_submit_transaction",
                &serde_json::json!({
                    "tx": tx_hex,
                    "options": { "trust_policy": trust_policy },
                }),
            )
            .await
    }

    /// Returns the fee rate for transactions landing in the top `in_top_x_mb`
    /// megabytes of the mempool (`mempool_get_fee_rate`).
    pub async fn fee_rate(&self, in_top_x_mb: u32) -> Result<Option<FeeRate>, Error> {
        self.transport
            .call(
                "mempool_get_fee_rate",
                &serde_json::json!({ "in_top_x_mb": in_top_x_mb }),
            )
            .await
    }

    /// Returns the fee-rate estimate curve (`mempool_get_fee_rate_points`).
    pub async fn fee_rate_points(&self) -> Result<Vec<FeeRatePoint>, Error> {
        self.transport.call("mempool_get_fee_rate_points", &serde_json::json!({})).await
    }

    /// Returns the mempool memory usage in bytes
    /// (`mempool_memory_usage`).
    pub async fn memory_usage(&self) -> Result<u64, Error> {
        self.transport.call("mempool_memory_usage", &serde_json::json!({})).await
    }
}
