// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Mintlayer Rust SDK.
//!
//! A Rust SDK for the [Mintlayer](https://www.mintlayer.org/) blockchain,
//! organised as independent sub-clients plus a top-level [`Client`] that wires
//! them together:
//!
//! | Module     | Purpose                                              | Default port  |
//! |------------|------------------------------------------------------|---------------|
//! | [`node`]   | JSON-RPC 2.0 client for the node daemon              | 3030 mainnet  |
//! | [`indexer`]| REST client for the indexer (api-web-server)         | 3000          |
//! | [`wallet`] | JSON-RPC 2.0 client for the wallet daemon            | 3034 mainnet  |
//! | [`crypto`] | Cryptography & transaction building (native)         | —             |

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "crypto")]
pub mod crypto;
#[cfg(feature = "indexer")]
pub mod indexer;
#[cfg(feature = "node")]
pub mod node;
#[cfg(feature = "wallet")]
pub mod wallet;

#[cfg(any(feature = "node", feature = "wallet"))]
mod jsonrpc;

#[cfg(any(feature = "node", feature = "wallet", feature = "indexer"))]
mod number;

#[cfg(any(feature = "node", feature = "wallet", feature = "indexer"))]
mod client;

#[cfg(any(feature = "node", feature = "wallet", feature = "indexer"))]
pub use client::{Client, ClientBuilder};
