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
