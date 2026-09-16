// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Error type for the indexer client.

/// Errors returned by [`crate::indexer::Client`] calls.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The indexer answered with an HTTP status of 400 or above.
    #[error("HTTP {status_code}: {body}")]
    Http {
        /// The HTTP status code.
        status_code: u16,
        /// The trimmed response body.
        body: String,
    },
    /// The HTTP request to the indexer failed.
    #[error("HTTP request failed: {0}")]
    Transport(#[from] reqwest::Error),
    /// The response could not be decoded into the expected type.
    #[error("failed to decode response: {0}")]
    Json(#[from] serde_json::Error),
    /// The request URL could not be constructed.
    #[error("invalid request URL: {message}")]
    InvalidUrl {
        /// The reason the URL is invalid.
        message: String,
    },
    /// The indexer response exceeded the maximum accepted body size.
    #[error("indexer response exceeds the maximum accepted size of {limit} bytes")]
    ResponseTooLarge {
        /// The limit that was exceeded.
        limit: usize,
    },
}
