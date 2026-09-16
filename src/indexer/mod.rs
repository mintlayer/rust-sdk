// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! REST client for the Mintlayer indexer (api-web-server). All paths are
//! relative to `/api/v2/` (default port 3000).

mod address;
mod block;
mod chain;
mod delegation;
mod error;
mod order;
mod pool;
mod statistics;
mod token;
mod transaction;
mod types;

use std::time::Duration;

use serde::de::DeserializeOwned;

pub use error::Error;
pub use types::{
    AddressInfo, Amount, Block, BlockBody, BlockHeader, ChainTip, CoinStats, Delegation,
    DelegationInfo, GenesisInfo, MerklePath, NftInfo, NftMetadata, Order, PageOpts, PerThousand,
    Pool, PoolDelegation, PoolListOpts, PoolSort, Timestamp, TokenBalance, TokenInfo, TokenTx,
    Transaction, Uint64, Utxo, UtxoOutpoint,
};

use crate::limits::{DEFAULT_TIMEOUT, MAX_RESPONSE_BYTES};

/// Client for the Mintlayer indexer REST API (api-web-server).
#[derive(Debug, Clone)]
pub struct Client {
    api_base: String,
    http: reqwest::Client,
}

impl Client {
    /// Creates a client with default settings: a 30 second timeout.
    ///
    /// The indexer API is unauthenticated, so there are no credential
    /// options.
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::builder(base_url)
            .build()
            .expect("reqwest client with default settings must build")
    }

    /// Returns a builder to construct a client with custom settings.
    #[must_use]
    pub fn builder(base_url: impl Into<String>) -> ClientBuilder {
        ClientBuilder {
            api_base: format!("{}/api/v2", base_url.into().trim_end_matches('/')),
            timeout: DEFAULT_TIMEOUT,
            http_client: None,
        }
    }

    pub(crate) async fn get<R>(&self, path: &str, query: &[(&str, String)]) -> Result<R, Error>
    where
        R: DeserializeOwned,
    {
        let mut url =
            reqwest::Url::parse(&format!("{}{}", self.api_base, path)).map_err(|error| {
                Error::InvalidUrl {
                    message: error.to_string(),
                }
            })?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query.iter().cloned());
        }
        let response = self.http.get(url).header("accept", "application/json").send().await?;
        self.decode(response).await
    }

    pub(crate) async fn post<R>(&self, path: &str, body: &str) -> Result<R, Error>
    where
        R: DeserializeOwned,
    {
        let url = reqwest::Url::parse(&format!("{}{}", self.api_base, path)).map_err(|error| {
            Error::InvalidUrl {
                message: error.to_string(),
            }
        })?;
        let response = self
            .http
            .post(url)
            .header("accept", "application/json")
            .header(reqwest::header::CONTENT_TYPE, "text/plain")
            .body(body.to_owned())
            .send()
            .await?;
        self.decode(response).await
    }

    async fn decode<R: DeserializeOwned>(&self, response: reqwest::Response) -> Result<R, Error> {
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Http {
                status_code: status.as_u16(),
                body: body.trim().to_owned(),
            });
        }
        let bytes = Self::read_capped(response).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    async fn read_capped(mut response: reqwest::Response) -> Result<Vec<u8>, Error> {
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(Error::ResponseTooLarge {
                limit: MAX_RESPONSE_BYTES,
            });
        }
        let mut body: Vec<u8> = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(Error::ResponseTooLarge {
                    limit: MAX_RESPONSE_BYTES,
                });
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

/// Builds the pagination query parameters; zero values are omitted.
pub(crate) fn page_query(offset: u32, items: u32) -> Vec<(&'static str, String)> {
    let mut query = Vec::new();
    if offset > 0 {
        query.push(("offset", offset.to_string()));
    }
    if items > 0 {
        query.push(("items", items.to_string()));
    }
    query
}

/// Builder for [`Client`].
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    api_base: String,
    timeout: Duration,
    http_client: Option<reqwest::Client>,
}

impl ClientBuilder {
    /// Overrides the default 30 second request timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Replaces the internally built HTTP client entirely.
    #[must_use]
    pub fn http_client(mut self, http_client: reqwest::Client) -> Self {
        self.http_client = Some(http_client);
        self
    }

    /// Builds the client.
    pub fn build(self) -> Result<Client, reqwest::Error> {
        let http = match self.http_client {
            Some(http) => http,
            None => reqwest::Client::builder().timeout(self.timeout).build()?,
        };
        Ok(Client {
            api_base: self.api_base,
            http,
        })
    }
}
