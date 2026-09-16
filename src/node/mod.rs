// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! JSON-RPC 2.0 client for the Mintlayer node daemon (default port 3030 on
//! mainnet, 13030 on testnet).

mod chainstate;
mod error;
mod mempool;
mod p2p;
mod types;

use std::sync::OnceLock;
use std::time::Duration;

pub use error::Error;
pub use types::{
    Amount, BannedPeer, BannedTime, ChainstateInfo, Currency, FeeRate, FeeRatePoint, MempoolTx,
    OrderInfo, Outpoint, OutpointSourceId, PeerInfo, Timestamp, TokenInfo, TrustPolicy,
};

use crate::jsonrpc::{self, DEFAULT_TIMEOUT};

static DEFAULT_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn default_http_client() -> reqwest::Client {
    DEFAULT_HTTP_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(DEFAULT_TIMEOUT)
                .build()
                .expect("reqwest client with default settings must build")
        })
        .clone()
}

/// Client for the Mintlayer node daemon JSON-RPC API.
#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) transport: jsonrpc::Transport,
}

impl Client {
    /// Creates a client with default settings: a 30 second timeout and no
    /// authentication.
    #[must_use]
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            transport: jsonrpc::Transport::from_parts(endpoint.into(), default_http_client(), None),
        }
    }

    /// Returns a builder to construct a client with custom settings.
    #[must_use]
    pub fn builder(endpoint: impl Into<String>) -> ClientBuilder {
        ClientBuilder {
            inner: jsonrpc::ClientBuilder::new(endpoint),
        }
    }

    /// Returns the node software version, e.g. `"1.3.0"`.
    pub async fn node_version(&self) -> Result<String, Error> {
        self.transport.call("node_version", &serde_json::json!({})).await
    }

    /// Requests a graceful shutdown of the node daemon.
    pub async fn node_shutdown(&self) -> Result<(), Error> {
        self.transport.call("node_shutdown", &serde_json::json!({})).await
    }
}

/// Builder for [`Client`].
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    inner: jsonrpc::ClientBuilder,
}

impl ClientBuilder {
    /// Sends HTTP basic auth credentials with every request.
    #[must_use]
    pub fn basic_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.inner = self.inner.basic_auth(username, password);
        self
    }

    /// Overrides the default 30 second request timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    /// Replaces the internally built HTTP client entirely.
    #[must_use]
    pub fn http_client(mut self, http_client: reqwest::Client) -> Self {
        self.inner = self.inner.http_client(http_client);
        self
    }

    /// Builds the client.
    pub fn build(self) -> Result<Client, reqwest::Error> {
        Ok(Client {
            transport: self.inner.build()?,
        })
    }
}
