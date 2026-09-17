// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Error type for the wallet daemon client.

crate::jsonrpc::define_error!(
    /// Errors returned by [`crate::wallet::Client`] calls.
    Error
);
