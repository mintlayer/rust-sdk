// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! JSON-RPC 2.0 client for the Mintlayer wallet daemon (default port 3034 on
//! mainnet, 13034 on testnet).

mod error;
mod management;
mod orders;
mod staking;
mod tokens;
mod transaction;
mod types;

pub use error::Error;
pub use types::{
    AccountInfo, ActiveOrder, AddressWithUsage, Amount, Balance, BestBlock, ChangeAuthorityParams,
    ComposeParams, ComposedTx, ConcludeOrderParams, CreateDelegationParams, CreateDelegationResult,
    CreateOrderParams, CreatePoolParams, CreateWalletParams, CreateWalletResult, CurrencyFilter,
    DecommissionParams, DelegateParams, DelegationInfo, FillOrderParams, FreezeOrderParams,
    FreezeParams, IssueNftParams, IssueTokenParams, IssueTokenResult, ListOrdersParams,
    LockSupplyParams, MintParams, MnemonicContent, MnemonicResult, NftMetadata, OrderCreated,
    OrderState, Outpoint, OutpointSourceId, OutputValue, OwnOrder, OwnedPool, RecoverWalletParams,
    RevealPublicKey, SendParams, SendResult, SignedTx, StakingStatus, SubmitResult, SweepParams,
    TokenMetadata, TokenSendParams, TokenSupply, TxInspection, TxOptions, TxStats, UnfreezeParams,
    UnmintParams, UtxoSpendParams, WalletExtraInfo, WalletInfo, WalletTx, WithdrawParams,
};

use crate::jsonrpc;

/// Client for the Mintlayer wallet daemon JSON-RPC API.
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
            transport: jsonrpc::Transport::from_parts(
                endpoint.into(),
                crate::limits::default_http_client(),
                None,
            ),
        }
    }

    /// Returns a builder to construct a client with custom settings.
    #[must_use]
    pub fn builder(endpoint: impl Into<String>) -> ClientBuilder {
        ClientBuilder {
            inner: jsonrpc::ClientBuilder::new(endpoint),
        }
    }

    pub(crate) async fn call<P, R>(&self, method: &str, params: &P) -> Result<R, Error>
    where
        P: serde::Serialize + ?Sized,
        R: serde::de::DeserializeOwned,
    {
        self.transport.call(method, params).await.map_err(Error::from)
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
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
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
