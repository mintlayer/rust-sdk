// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Internal JSON-RPC 2.0 transport shared by the node and wallet clients.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::limits::{DEFAULT_TIMEOUT, MAX_RESPONSE_BYTES};

/// HTTP basic auth credentials with a redacted [`Debug`] implementation so
/// that logging a client never leaks the password.
#[derive(Clone)]
pub(crate) struct BasicAuth {
    username: String,
    password: String,
}

impl fmt::Debug for BasicAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicAuth")
            .field("username", &"***")
            .field("password", &"***")
            .finish()
    }
}

/// Defines the public error enum for an RPC sub-client from the internal
/// [`RequestError`].
macro_rules! define_error {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, thiserror::Error)]
        pub enum $name {
            /// The daemon answered with a JSON-RPC error response.
            #[error("RPC error {code}: {message}")]
            Rpc {
                /// The JSON-RPC error code.
                code: i64,
                /// The JSON-RPC error message.
                message: String,
            },
            /// The HTTP request to the daemon failed.
            #[error("HTTP request failed: {0}")]
            Http(#[from] reqwest::Error),
            /// The JSON-RPC response could not be decoded into the expected type.
            #[error("failed to decode JSON-RPC response: {0}")]
            Json(#[from] serde_json::Error),
            /// The response `id` did not match the request `id`.
            #[error("JSON-RPC response id mismatch: expected {expected}, got {actual}")]
            IdMismatch {
                /// The `id` that was sent.
                expected: u64,
                /// The `id` that was received.
                actual: serde_json::Value,
            },
            /// The daemon response exceeded the maximum accepted body size.
            #[error("daemon response exceeds the maximum accepted size of {limit} bytes")]
            ResponseTooLarge {
                /// The limit that was exceeded.
                limit: usize,
            },
        }

        impl From<crate::jsonrpc::RequestError> for $name {
            fn from(err: crate::jsonrpc::RequestError) -> Self {
                match err {
                    crate::jsonrpc::RequestError::Rpc { code, message } => Self::Rpc { code, message },
                    crate::jsonrpc::RequestError::Http(err) => Self::Http(err),
                    crate::jsonrpc::RequestError::Json(err) => Self::Json(err),
                    crate::jsonrpc::RequestError::IdMismatch { expected, actual } => {
                        Self::IdMismatch { expected, actual }
                    }
                    crate::jsonrpc::RequestError::ResponseTooLarge { limit } => {
                        Self::ResponseTooLarge { limit }
                    }
                }
            }
        }
    };
}
pub(crate) use define_error;

/// JSON-RPC `error` object as carried inside a daemon response.
#[derive(Debug, Deserialize, Error)]
#[error("RPC error {code}: {message}")]
pub(crate) struct RpcErrorWire {
    code: i64,
    message: String,
}

/// Transport-level failure of a JSON-RPC call.
#[derive(Debug, Error)]
pub(crate) enum RequestError {
    #[error("RPC error {code}: {message}")]
    Rpc { code: i64, message: String },
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("failed to decode JSON-RPC response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("JSON-RPC response id mismatch: expected {expected}, got {actual}")]
    IdMismatch {
        expected: u64,
        actual: serde_json::Value,
    },
    #[error("daemon response exceeds the maximum accepted size of {limit} bytes")]
    ResponseTooLarge { limit: usize },
}

#[derive(Serialize)]
struct Request<'a, P: ?Sized> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: &'a P,
}

#[derive(Deserialize)]
struct Response {
    id: Option<serde_json::Value>,
    #[serde(default)]
    result: serde_json::Value,
    error: Option<RpcErrorWire>,
}

/// A shared JSON-RPC 2.0 transport: one HTTP endpoint, monotonically
/// increasing request ids starting at 1, optional HTTP basic auth.
///
/// Cloning a transport shares the id counter, so every clone of a client
/// produces globally unique request ids.
#[derive(Debug, Clone)]
pub(crate) struct Transport {
    endpoint: String,
    http: reqwest::Client,
    basic_auth: Option<BasicAuth>,
    next_id: Arc<AtomicU64>,
}

impl Transport {
    pub(crate) fn from_parts(
        endpoint: String,
        http: reqwest::Client,
        basic_auth: Option<BasicAuth>,
    ) -> Self {
        Self {
            endpoint,
            http,
            basic_auth,
            next_id: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(crate) async fn call<P, R, E>(&self, method: &str, params: &P) -> Result<R, E>
    where
        P: Serialize + ?Sized,
        R: DeserializeOwned,
        E: From<RequestError>,
    {
        self.call_inner(method, params).await.map_err(E::from)
    }

    async fn call_inner<P, R>(&self, method: &str, params: &P) -> Result<R, RequestError>
    where
        P: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let mut builder = self.http.post(&self.endpoint).json(&Request {
            jsonrpc: "2.0",
            id,
            method,
            params,
        });
        if let Some(auth) = &self.basic_auth {
            builder = builder.basic_auth(&auth.username, Some(&auth.password));
        }
        // The HTTP status code is intentionally not inspected: the daemon may
        // answer with a valid JSON-RPC envelope on a non-2xx status (parity
        // with the go-sdk client).
        let http_response = builder.send().await?;
        let response: Response = read_json_body(http_response).await?;
        if let Some(err) = response.error {
            return Err(RequestError::Rpc {
                code: err.code,
                message: err.message,
            });
        }
        match response.id {
            Some(actual) if actual.as_u64() != Some(id) => {
                return Err(RequestError::IdMismatch {
                    expected: id,
                    actual,
                });
            }
            _ => {}
        }
        Ok(serde_json::from_value(response.result)?)
    }
}

async fn read_json_body(mut http_response: reqwest::Response) -> Result<Response, RequestError> {
    if http_response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(RequestError::ResponseTooLarge {
            limit: MAX_RESPONSE_BYTES,
        });
    }
    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = http_response.chunk().await? {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(RequestError::ResponseTooLarge {
                limit: MAX_RESPONSE_BYTES,
            });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&body)?)
}

/// Shared builder state for the node and wallet clients.
#[derive(Debug, Clone)]
pub(crate) struct ClientBuilder {
    endpoint: String,
    timeout: Duration,
    basic_auth: Option<BasicAuth>,
    http_client: Option<reqwest::Client>,
}

impl ClientBuilder {
    pub(crate) fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            timeout: DEFAULT_TIMEOUT,
            basic_auth: None,
            http_client: None,
        }
    }

    pub(crate) fn basic_auth(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.basic_auth = Some(BasicAuth {
            username: username.into(),
            password: password.into(),
        });
        self
    }
    pub(crate) fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub(crate) fn http_client(mut self, http_client: reqwest::Client) -> Self {
        self.http_client = Some(http_client);
        self
    }

    pub(crate) fn build(self) -> Result<Transport, reqwest::Error> {
        let http = match self.http_client {
            Some(http) => http,
            None => reqwest::Client::builder().timeout(self.timeout).build()?,
        };
        Ok(Transport::from_parts(self.endpoint, http, self.basic_auth))
    }
}
