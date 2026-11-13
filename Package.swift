// swift-tools-version: 5.9
//
// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

// SPM entry point for iOS consumers. The binary target is attached to the
// `wallet-bindings-vX.Y.Z` GitHub release; its checksum is updated by the
// release workflow. CI tests use the path-based package under
// bindings/swift-tests, which builds the framework from source instead.

import PackageDescription

let releaseTag = "wallet-bindings-v0.1.0"
let releaseBaseUrl =
    "https://github.com/mintlayer/rust-sdk/releases/download/\(releaseTag)"

let package = Package(
    name: "MintlayerWalletBindings",
    platforms: [
        // Matches the React Native 0.88 minimum deployment target.
        .iOS(.v15)
    ],
    products: [
        .library(
            name: "MintlayerWalletBindings",
            targets: ["MintlayerWalletBindings"]
        )
    ],
    targets: [
        .binaryTarget(
            name: "MintlayerWalletBindingsFFI",
            url: "\(releaseBaseUrl)/MintlayerWalletBindings.xcframework.zip",
            // Replaced by the release workflow when the artifact is attached.
            checksum: "aaf7197f472c01483e1d4e9f7f222df1c046c12430820ef371f83f832f4d6b97"
        ),
        // The generated Swift wrapper ships as source (standard UniFFI
        // practice); the FFI static library inside the XCFramework is what
        // the release checksum attests.
        .target(
            name: "MintlayerWalletBindings",
            dependencies: ["MintlayerWalletBindingsFFI"],
            path: "SwiftSources"
        )
    ]
)
