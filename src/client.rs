// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Top-level SDK client wiring the sub-clients together.

#[cfg(feature = "indexer")]
use crate::indexer;
#[cfg(feature = "node")]
use crate::node;
#[cfg(feature = "wallet")]
use crate::wallet;

/// The top-level Mintlayer SDK client.
///
/// Each sub-client is `Some` only when its URL was set on the
/// [`ClientBuilder`]. The `crypto` module functions are always available
/// directly (when the `crypto` feature is enabled) and require no client.
#[derive(Debug, Clone, Default)]
pub struct Client {
    /// JSON-RPC client for the node daemon.
    #[cfg(feature = "node")]
    pub node: Option<node::Client>,
    /// REST client for the indexer.
    #[cfg(feature = "indexer")]
    pub indexer: Option<indexer::Client>,
    /// JSON-RPC client for the wallet daemon.
    #[cfg(feature = "wallet")]
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
    #[cfg(feature = "node")]
    #[must_use]
    pub fn node_url(mut self, url: impl Into<String>) -> Self {
        self.node_url = Some(url.into());
        self
    }

    /// Creates an indexer client for the given base URL.
    #[cfg(feature = "indexer")]
    #[must_use]
    pub fn indexer_url(mut self, url: impl Into<String>) -> Self {
        self.indexer_url = Some(url.into());
        self
    }

    /// Creates a wallet daemon client for the given base URL.
    #[cfg(feature = "wallet")]
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
        #[cfg(any(feature = "node", feature = "wallet"))]
        let basic_auth = self.basic_auth;
        #[cfg(not(any(feature = "node", feature = "wallet")))]
        let _ = self.basic_auth;
        let timeout = self.timeout;
        #[cfg(feature = "node")]
        let node_url = self.node_url;
        #[cfg(feature = "indexer")]
        let indexer_url = self.indexer_url;
        #[cfg(feature = "wallet")]
        let wallet_url = self.wallet_url;
        #[cfg(not(any(feature = "node", feature = "indexer", feature = "wallet")))]
        let _ = self;

        #[cfg(any(feature = "node", feature = "wallet"))]
        let rpc_options = RpcOptions {
            basic_auth: &basic_auth,
            timeout,
        };
        #[cfg(feature = "indexer")]
        let rest_options = RestOptions { timeout };

        Ok(Client {
            #[cfg(feature = "node")]
            node: node_url
                .map(|url| node::Client::builder(url).rpc_options(&rpc_options).build())
                .transpose()?,
            #[cfg(feature = "indexer")]
            indexer: indexer_url
                .map(|url| indexer::Client::builder(url).rest_options(&rest_options).build())
                .transpose()?,
            #[cfg(feature = "wallet")]
            wallet: wallet_url
                .map(|url| wallet::Client::builder(url).rpc_options(&rpc_options).build())
                .transpose()?,
        })
    }
}

#[cfg(any(feature = "node", feature = "wallet"))]
struct RpcOptions<'a> {
    basic_auth: &'a Option<(String, String)>,
    timeout: Option<std::time::Duration>,
}

#[cfg(feature = "indexer")]
struct RestOptions {
    timeout: Option<std::time::Duration>,
}

#[cfg(feature = "node")]
impl node::ClientBuilder {
    fn rpc_options(self, options: &RpcOptions<'_>) -> Self {
        let builder = match options.basic_auth {
            Some((username, password)) => self.basic_auth(username, password),
            None => self,
        };
        match options.timeout {
            Some(timeout) => builder.timeout(timeout),
            None => builder,
        }
    }
}

#[cfg(feature = "wallet")]
impl wallet::ClientBuilder {
    fn rpc_options(self, options: &RpcOptions<'_>) -> Self {
        let builder = match options.basic_auth {
            Some((username, password)) => self.basic_auth(username, password),
            None => self,
        };
        match options.timeout {
            Some(timeout) => builder.timeout(timeout),
            None => builder,
        }
    }
}

#[cfg(feature = "indexer")]
impl indexer::ClientBuilder {
    fn rest_options(self, options: &RestOptions) -> Self {
        match options.timeout {
            Some(timeout) => self.timeout(timeout),
            None => self,
        }
    }
}
