// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Shared limits and defaults for daemon clients.

use std::time::Duration;

/// Upper bound for a single daemon response body, guarding against memory
/// exhaustion from a misconfigured or hostile endpoint.
pub(crate) const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

/// Default request timeout for the daemon HTTP clients.
pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Builds the default HTTP client: overall timeout and no redirects, so a
/// misbehaving endpoint can never forward request bodies (which may carry
/// credentials or mnemonics) to another host.
pub(crate) fn default_http_client_with_timeout(
    timeout: std::time::Duration,
) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .build()
}

/// Lazily built process-wide default HTTP client for the daemon RPC
/// clients.
#[cfg(any(feature = "node", feature = "wallet"))]
pub(crate) fn default_http_client() -> reqwest::Client {
    static DEFAULT_HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    DEFAULT_HTTP_CLIENT
        .get_or_init(|| {
            default_http_client_with_timeout(DEFAULT_TIMEOUT)
                .expect("reqwest client with default settings must build")
        })
        .clone()
}

/// Reads a daemon response body in chunks, rejecting anything beyond
/// [`MAX_RESPONSE_BYTES`] before it can exhaust memory.
///
/// `too_large` constructs the caller's own body-size error.
pub(crate) async fn read_capped_body<E>(
    mut response: reqwest::Response,
    too_large: fn(usize) -> E,
) -> Result<Vec<u8>, E>
where
    E: From<reqwest::Error>,
{
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(too_large(MAX_RESPONSE_BYTES));
    }
    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(too_large(MAX_RESPONSE_BYTES));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
