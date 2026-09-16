// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Top-level SDK client wiring the sub-clients together.

use crate::indexer;
use crate::node;
use crate::wallet;

/// The top-level Mintlayer SDK client.
///
/// Each sub-client is `Some` only when its URL was set on the
/// [`ClientBuilder`]. The [`crypto`](crate::crypto) functions are always
/// available directly and require no client.
#[derive(Debug, Clone)]
pub struct Client {
    /// JSON-RPC client for the node daemon.
    pub node: Option<node::Client>,
    /// REST client for the indexer.
    pub indexer: Option<indexer::Client>,
    /// JSON-RPC client for the wallet daemon.
    pub wallet: Option<wallet::Client>,
}

impl Client {
    /// Returns a builder; sub-clients are created only for the URLs that are
    /// set.
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder {
            node_url: None,
            indexer_url: None,
            wallet_url: None,
            basic_auth: None,
            timeout: None,
        }
    }
}

/// Builder for [`Client`].
#[derive(Clone)]
pub struct ClientBuilder {
    node_url: Option<String>,
    indexer_url: Option<String>,
    wallet_url: Option<String>,
    basic_auth: Option<(String, String)>,
    timeout: Option<std::time::Duration>,
}

impl std::fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("node_url", &self.node_url)
            .field("indexer_url", &self.indexer_url)
            .field("wallet_url", &self.wallet_url)
            .field("basic_auth", &self.basic_auth.as_ref().map(|_| "***"))
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl ClientBuilder {
    /// Creates a node daemon client for the given base URL.
    #[must_use]
    pub fn node_url(mut self, url: impl Into<String>) -> Self {
        self.node_url = Some(url.into());
        self
    }

    /// Creates an indexer client for the given base URL.
    #[must_use]
    pub fn indexer_url(mut self, url: impl Into<String>) -> Self {
        self.indexer_url = Some(url.into());
        self
    }

    /// Creates a wallet daemon client for the given base URL.
    #[must_use]
    pub fn wallet_url(mut self, url: impl Into<String>) -> Self {
        self.wallet_url = Some(url.into());
        self
    }

    /// Sends HTTP basic auth credentials with node and wallet requests.
    ///
    /// The indexer API is unauthenticated; the credentials are not applied
    /// to it.
    #[must_use]
    pub fn basic_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.basic_auth = Some((username.into(), password.into()));
        self
    }

    /// Overrides the default 30 second timeout of every sub-client.
    #[must_use]
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Builds the client.
    pub fn build(self) -> Result<Client, reqwest::Error> {
        let node = self.node_url.map(|url| {
            let builder = node::Client::builder(url);
            let builder = match &self.basic_auth {
                Some((username, password)) => builder.basic_auth(username, password),
                None => builder,
            };
            match self.timeout {
                Some(timeout) => builder.timeout(timeout),
                None => builder,
            }
            .build()
        });
        let wallet = self.wallet_url.map(|url| {
            let builder = wallet::Client::builder(url);
            let builder = match &self.basic_auth {
                Some((username, password)) => builder.basic_auth(username, password),
                None => builder,
            };
            match self.timeout {
                Some(timeout) => builder.timeout(timeout),
                None => builder,
            }
            .build()
        });
        let indexer = self.indexer_url.map(|url| {
            let builder = indexer::Client::builder(url);
            match self.timeout {
                Some(timeout) => builder.timeout(timeout),
                None => builder,
            }
            .build()
        });

        Ok(Client {
            node: node.transpose()?,
            indexer: indexer.transpose()?,
            wallet: wallet.transpose()?,
        })
    }
}
