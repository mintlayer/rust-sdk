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
    /// The indexer rejected a cursor (`400`, body `Invalid cursor`): the
    /// cursor is malformed or oversized, belongs to a different listing,
    /// or was minted for the other side of an order book.
    #[error("invalid cursor")]
    InvalidCursor,
    /// The indexer rejected the requested page size (`400`, body `Invalid
    /// number of items`); every paginated v2 endpoint accepts 1..=100
    /// items.
    #[error("invalid number of items (must be 1..=100)")]
    InvalidNumItems,
    /// The indexer rejected the query (`400`, body `Bad request`), for
    /// example mutually incompatible parameters (`cursor` together with
    /// `offset_mode`, or a cursor with a non-default pools sort).
    #[error("bad request (incompatible query parameters)")]
    BadRequest,
    /// The referenced token does not exist (`404`, body `Token not
    /// found`).
    #[error("token not found")]
    TokenNotFound,
}

impl Error {
    /// Maps an indexer error response to a typed error when the
    /// `{ "error": "<message>" }` body matches a known server message;
    /// falls back to [`Error::Http`] otherwise.
    pub(crate) fn from_status_body(status_code: u16, body: &str) -> Error {
        #[derive(serde::Deserialize)]
        struct ErrorBody {
            error: String,
        }
        if let Ok(parsed) = serde_json::from_str::<ErrorBody>(body) {
            match (status_code, parsed.error.as_str()) {
                (400, "Invalid cursor") => return Error::InvalidCursor,
                (400, "Invalid number of items") => return Error::InvalidNumItems,
                (400, "Bad request") => return Error::BadRequest,
                (404, "Token not found") => return Error::TokenNotFound,
                _ => {}
            }
        }
        Error::Http {
            status_code,
            body: body.to_owned(),
        }
    }
}
