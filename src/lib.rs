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
//! | `crypto`   | Cryptography & transaction building (native)         | —             |
//!
//! # Amount types
//!
//! The daemon wire format differs per client, so each module has its own
//! `Amount` type:
//!
//! - `crypto::Amount` — a `u128` atom value
//!   (re-exported from mintlayer-core) used when building transactions.
//! - `node::Amount` — atoms as `u128`, decoded from the daemon's
//!   `{"atoms": "..."}` object.
//! - `indexer::Amount` — `atoms` plus the daemon's `decimal` string.
//! - `wallet::Amount` — optional `atoms`/`decimal` (requests typically set
//!   `atoms` only, via [`Amount::from_atoms`](crate::wallet::Amount)).
//!
//! All four are denominated in atoms (1 ML = 10^11 atoms); converting
//! between them is done through the atom value.

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

// The lenient wire-format helpers serve different feature combinations, so
// some are unused under any single-feature build.
#[cfg(any(feature = "node", feature = "wallet", feature = "indexer"))]
#[allow(dead_code)]
mod number;

#[cfg(any(feature = "node", feature = "wallet", feature = "indexer"))]
mod limits;

#[cfg(any(feature = "node", feature = "wallet", feature = "indexer"))]
mod client;

#[cfg(any(feature = "node", feature = "wallet", feature = "indexer"))]
pub use client::{Client, ClientBuilder};

/// Convenience re-exports so callers that only import the crate root do not
/// need to reach into the sub-modules (mirrors the go-sdk alias block).
///
/// # Secret handling
///
/// The re-exported key types (`PrivateKey`, `ExtendedPrivateKey`, ...) come
/// from mintlayer-core and do not redact their [`std::fmt::Debug`] output.
/// Never log or debug-print keys; keep their lifetime short.
#[cfg(feature = "crypto")]
pub mod prelude {
    pub use crate::crypto::types::*;
    pub use crate::crypto::{
        Amount, FreezableToken, IsTokenFreezable, IsTokenUnfreezable, Network, SigHashType,
        SourceId, TokenTotalSupply, TxAdditionalInfo,
    };
}
