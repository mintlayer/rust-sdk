// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! P2P RPC methods: peers, bans, transaction broadcast.

use std::time::Duration;

use super::{BannedPeer, Error, PeerInfo, TrustPolicy};
use crate::node::Client;

impl Client {
    /// Returns the number of connected peers (`p2p_get_peer_count`).
    pub async fn peer_count(&self) -> Result<u64, Error> {
        self.transport.call("p2p_get_peer_count", &serde_json::json!({})).await
    }

    /// Returns details about all connected peers
    /// (`p2p_get_connected_peers`).
    pub async fn connected_peers(&self) -> Result<Vec<PeerInfo>, Error> {
        self.transport.call("p2p_get_connected_peers", &serde_json::json!({})).await
    }

    /// Returns the addresses the node binds to (`p2p_get_bind_addresses`).
    pub async fn bind_addresses(&self) -> Result<Vec<String>, Error> {
        self.transport.call("p2p_get_bind_addresses", &serde_json::json!({})).await
    }

    /// Adds a reserved node (`p2p_add_reserved_node`).
    pub async fn add_reserved_node(&self, addr: &str) -> Result<(), Error> {
        self.transport
            .call(
                "p2p_add_reserved_node",
                &serde_json::json!({ "addr": addr }),
            )
            .await
    }

    /// Removes a reserved node (`p2p_remove_reserved_node`).
    pub async fn remove_reserved_node(&self, addr: &str) -> Result<(), Error> {
        self.transport
            .call(
                "p2p_remove_reserved_node",
                &serde_json::json!({ "addr": addr }),
            )
            .await
    }

    /// Opens a one-time connection to a node (`p2p_connect`).
    pub async fn connect(&self, addr: &str) -> Result<(), Error> {
        self.transport.call("p2p_connect", &serde_json::json!({ "addr": addr })).await
    }

    /// Disconnects a peer (`p2p_disconnect`).
    pub async fn disconnect(&self, peer_id: u64) -> Result<(), Error> {
        self.transport
            .call("p2p_disconnect", &serde_json::json!({ "peer_id": peer_id }))
            .await
    }

    /// Returns all banned peers (`p2p_list_banned`).
    pub async fn list_banned(&self) -> Result<Vec<BannedPeer>, Error> {
        self.transport.call("p2p_list_banned", &serde_json::json!({})).await
    }

    /// Bans an address for a duration (`p2p_ban`).
    pub async fn ban(&self, address: &str, duration: Duration) -> Result<(), Error> {
        let seconds = i64::try_from(duration.as_secs()).unwrap_or(i64::MAX);
        self.transport
            .call(
                "p2p_ban",
                &serde_json::json!({
                    "address": address,
                    "duration": [seconds, i64::from(duration.subsec_nanos())],
                }),
            )
            .await
    }

    /// Lifts a ban on an address (`p2p_unban`).
    pub async fn unban(&self, address: &str) -> Result<(), Error> {
        self.transport
            .call("p2p_unban", &serde_json::json!({ "address": address }))
            .await
    }

    /// Submits a hex-encoded signed transaction to the mempool and broadcasts
    /// it to P2P peers (`p2p_submit_transaction`). This is the correct call
    /// for propagating a transaction across the network.
    pub async fn broadcast_transaction(
        &self,
        tx_hex: &str,
        trust_policy: TrustPolicy,
    ) -> Result<(), Error> {
        self.transport
            .call(
                "p2p_submit_transaction",
                &serde_json::json!({
                    "tx": tx_hex,
                    "options": { "trust_policy": trust_policy },
                }),
            )
            .await
    }
}
