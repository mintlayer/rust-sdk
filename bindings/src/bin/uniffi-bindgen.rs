// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Helper binary that generates the Kotlin and Swift foreign-language
//! bindings from the compiled library. Not linked into any shipped artifact.

fn main() {
    uniffi::uniffi_bindgen_main()
}
