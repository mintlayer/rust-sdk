// swift-tools-version: 5.9
//
// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

// CI/test package: builds the FFI XCFramework from a local path (produced
// by scripts/ios/build-xcframework.sh) and runs the on-simulator
// acceptance tests. Consumers use the root Package.swift instead, which
// points at the release artifact.

import PackageDescription

let package = Package(
    name: "MintlayerWalletBindingsTests",
    platforms: [
        .iOS(.v15)
    ],
    products: [
        .library(
            name: "MintlayerWalletBindingsTests",
            targets: ["MintlayerWalletBindings"]
        )
    ],
    targets: [
        .binaryTarget(
            name: "MintlayerWalletBindingsFFI",
            path: "../../dist/MintlayerWalletBindings.xcframework"
        ),
        .target(
            name: "MintlayerWalletBindings",
            dependencies: ["MintlayerWalletBindingsFFI"],
            path: "SwiftSources"
        ),
        .testTarget(
            name: "AcceptanceTests",
            dependencies: ["MintlayerWalletBindingsSwift"],
            path: "Tests"
        )
    ]
)
