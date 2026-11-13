# Mobile bindings (Kotlin and Swift)

Native mobile bindings for the Mintlayer wallet key and address
primitives, generated with [UniFFI](https://mozilla.github.io/uniffi-rs/)
from the `mintlayer-wallet-bindings` crate.

For mobile applications these bindings are the recommended integration and
replace the WASM wrappers: React Native's Hermes engine does not provide a
`WebAssembly` global, so the WASM path is unusable there, while the native
bindings run everywhere. The WebAssembly route remains available for
browser use cases and its behavior is unchanged.

The exported surface mirrors the `wasm-bindings` exports — same function
names, equivalent parameter shapes, same runtime blob formats — so existing
key and address handling ports to the native bindings with minimal changes
(amounts move from JS numbers to decimal atom strings). Blob outputs are
byte-for-byte identical to the WASM wrappers; parity is pinned by tests
against captured reference outputs of `@mintlayer/wasm-lib` 1.4.0 (see
"Parity guarantees" below).

## Supported platforms and architectures

| Platform | Artifact | Architectures |
|---|---|---|
| Android 7.0+ (API 24) | `mintlayer-wallet-bindings-<ver>.aar` | `arm64-v8a`, `armeabi-v7a`, `x86_64` |
| iOS 15+ | `MintlayerWalletBindings.xcframework` | `ios-arm64` (device), `ios-arm64_x86_64-simulator` |

The Intel simulator slice (`x86_64-apple-ios`) is omitted: recent
Xcode releases no longer ship Intel simulator runtimes.

## API surface

**Keys and addresses**

| Function | Purpose |
|---|---|
| `makePrivateKey()` | Generate a new random private key |
| `makeDefaultAccountPrivkey(mnemonic, network, passphrase?)` | BIP39 mnemonic → default account extended private key (`m/44'/<coin type>/0'`) |
| `extendedPrivateKeyDecode(blob)` | Extended private key blob → structured parts |
| `extendedPrivateKeyEncode(parts)` | Structured parts → canonical extended private key blob |
| `makeReceivingAddress(accountPrivkey, keyIndex)` | Derive receiving private key (`<account>/0/<index>`); returns a plain private-key blob, not an address |
| `makeChangeAddress(accountPrivkey, keyIndex)` | Derive change private key (`<account>/1/<index>`); returns a plain private-key blob, not an address |
| `makeReceivingAddressPublicKey(accountPublicKey, keyIndex)` | Watch-only receiving public key derivation |
| `makeChangeAddressPublicKey(accountPublicKey, keyIndex)` | Watch-only change public key derivation |
| `publicKeyFromPrivateKey(privateKey)` | Public key of a private key |
| `extendedPublicKeyFromExtendedPrivateKey(accountPrivkey)` | Extended public key of an account key |
| `pubkeyToPubkeyhashAddress(publicKey, network)` | Public key → bech32 address |
| `encodeDestination(address, network)` | bech32 address → encoded destination blob |

**Outputs** (each returns one `TxOutput` blob)

`encodeOutputTransfer`, `encodeOutputTokenTransfer`, `encodeOutputLockThenTransfer`, `encodeOutputTokenLockThenTransfer`, `encodeOutputCoinBurn`, `encodeOutputTokenBurn`, `encodeOutputCreateDelegation`, `encodeOutputDelegateStaking`, `encodeOutputCreateStakePool` (takes the blob from `encodeStakePoolData`), `encodeOutputProduceBlockFromStake`, `encodeOutputDataDeposit`, `encodeOutputIssueFungibleToken`, `encodeOutputIssueNft`, `encodeOutputHtlc`, `encodeCreateOrderOutput`

**Inputs** (each returns one `TxInput` blob)

`encodeInputForUtxo`, `encodeInputForWithdrawFromDelegation`, `encodeInputForMintTokens`, `encodeInputForUnmintTokens`, `encodeInputForLockTokenSupply`, `encodeInputForFreezeToken`, `encodeInputForUnfreezeToken`, `encodeInputForChangeTokenAuthority`, `encodeInputForChangeTokenMetadataUri`, `encodeInputForFillOrder`, `encodeInputForFreezeOrder`, `encodeInputForConcludeOrder`

**Transactions**

| Function | Purpose |
|---|---|
| `encodeOutpointSourceId(id, source)` | 32-byte id → outpoint source blob (UTXO reference) |
| `encodeTransaction(inputs, outputs, flags)` | Assemble an unsigned transaction from input/output blobs |
| `getTransactionId(transaction, strictByteSize)` | Hex transaction id |
| `decodeTransactionToJson(transaction, network)` | Unsigned transaction → JSON (empty witness list shape) |
| `decodeSignedTransactionToJson(transaction, network)` | Signed transaction → JSON |
| `decodePartiallySignedTransactionToJson(transaction, network)` | Partially signed transaction → JSON |
| `encodeSignedTransaction(transaction, signatures)` | Transaction + witness blobs → signed transaction blob |
| `encodePartiallySignedTransaction(...)` | Assemble a partially signed transaction (per-input optional witnesses, UTXOs, destinations, HTLC secrets) |
| `estimateTransactionSize(inputs, inputDestinations, outputs, network)` | Encoded size in bytes |
| `extractHtlcSecret(signedTx, strictByteSize, htlcOutpointSourceId, htlcOutputIndex)` | Revealed HTLC pre-image from a spend |

**Signing and witnesses**

`encodeWitness`, `encodeWitnessNoSignature`, `encodeWitnessHtlcSpend`, `encodeWitnessHtlcRefundSingleSig`, `encodeWitnessHtlcRefundMultisig`, `signMessageForSpending`, `verifySignatureForSpending`, `signChallenge`, `verifyChallenge`

**Intents**

`makeTransactionIntentMessageToSign`, `encodeSignedTransactionIntent`, `verifyTransactionIntent`

**Fees, locks, ids, staking, multisig**

`fungibleTokenIssuanceFee`, `nftIssuanceFee`, `tokenSupplyChangeFee`, `tokenFreezeFee`, `tokenChangeAuthorityFee`, `dataDepositFee`, `encodeLockForBlockCount`, `encodeLockForSeconds`, `encodeLockUntilTime`, `encodeLockUntilHeight`, `getTokenId`, `getOrderId`, `getDelegationId`, `getPoolId`, `encodeStakePoolData`, `effectivePoolBalance`, `stakingPoolSpendMaturityBlockCount`, `encodeMultisigChallenge`, `multisigChallengeToAddress`

Every fallible function returns a typed error (`BindingsException` in
Kotlin, `BindingsError` in Swift); nothing panics across the FFI boundary.
There is no storage, no networking and no mnemonic generation: the library
is a pure cryptography and encoding primitive.

## Data formats

- Extended private keys, extended public keys, plain private keys,
  public keys, inputs, outputs, witnesses, timelocks and transactions
  cross the boundary as byte arrays in the runtime SCALE format — the same
  bytes the WASM wrappers produce and consume.
- Addresses and token/pool/delegation/order ids are bech32 strings.
- Amounts are decimal atom strings (1 ML = 100,000,000,000 atoms), since
  128-bit atom counts do not fit into `Long`/`Int`.
- `Network` is `mainnet` / `testnet` / `regtest` / `signet`.

## Kotlin (Android)

The generated Kotlin lives under `org.mintlayer.wallet.bindings`. The
Android artifact is an AAR containing the native libraries for
`arm64-v8a`, `armeabi-v7a` and `x86_64`, attached to each release (see
"Releasing").

```kotlin
import org.mintlayer.wallet.bindings.Network
import org.mintlayer.wallet.bindings.extendedPublicKeyFromExtendedPrivateKey
import org.mintlayer.wallet.bindings.makeDefaultAccountPrivkey
import org.mintlayer.wallet.bindings.makeReceivingAddressPublicKey
import org.mintlayer.wallet.bindings.pubkeyToPubkeyhashAddress

val mnemonic = /* the user's seed phrase */
val account = makeDefaultAccountPrivkey(mnemonic, Network.TESTNET, null)
val xpub = extendedPublicKeyFromExtendedPrivateKey(account)
val receiving0 = makeReceivingAddressPublicKey(xpub, 0u)
val address = pubkeyToPubkeyhashAddress(receiving0, Network.TESTNET)
```

## Swift (iOS)

The iOS artifact is an XCFramework covering iOS devices (`ios-arm64`)
and Apple Silicon simulators (`ios-arm64_x86_64-simulator`); the Intel
simulator slice is omitted, matching recent Xcode releases. The repository
root carries an SPM `Package.swift` whose binary target fetches the
released artifact, so Xcode users add the package by repository URL. The
generated Swift wrapper is compiled by Swift
Package Manager against the XCFramework.

```swift
import MintlayerWalletBindings

let mnemonic = /* the user's seed phrase */
let account = try makeDefaultAccountPrivkey(mnemonic: mnemonic, network: .testnet, passphrase: nil)
let xpub = try extendedPublicKeyFromExtendedPrivateKey(accountPrivkey: account)
let receiving0 = try makeReceivingAddressPublicKey(accountPublicKey: xpub, keyIndex: 0)
let address = try pubkeyToPubkeyhashAddress(publicKey: receiving0, network: .testnet)
```

## Parity guarantees

The public BIP39 test vector `"abandon" × 23 + "art"` (24 words, checksum
valid — a public test vector, no secret material) produces, through these
bindings, exactly the same outputs as `@mintlayer/wasm-lib` 1.4.0:

| Network | Path | Address |
|---|---|---|
| testnet | `m/44'/1'/0'` | `tmt1qyxqfq37rra32sq6zhayqcpjwh4trc07mcfardwt` |
| mainnet | `m/44'/19788'/0'` | `mtc1q82twmwwxa3l2pe26gf0zgghu69jtez3qq2ajqyl` |

The `bindings/tests/parity.rs` test suite pins these vectors byte-for-byte
(extended private key blob, extended public key blob, receiving public key,
address, and the normalized `levels || point || chain code` form), together
with encode/decode roundtrip guarantees and private/public derivation
agreement at every tested index. The `bindings/tests/differential.rs`
suite additionally replays a 3,400-case deterministic input set captured
from the WASM reference through every export except the two VRF-gated
stake-pool encoders (their VRF-address input is not manufacturable from
the wasm API; those are pinned structurally) — 101 mnemonics, both
networks, plus encoder, signing, transaction and intent flows — and
asserts identical outputs; signature blobs use random nonces, so those
cases are cross-verified instead. CI runs both suites on every pull
request.

## Releasing

Binding releases are tagged `wallet-bindings-vX.Y.Z` (matching the
`version` in `bindings/Cargo.toml`). Pushing such a tag triggers the
release workflow, which builds the AAR and the XCFramework from the
tagged commit, attaches them with a `SHA256SUMS` manifest to the GitHub
release, and then opens a pull request that pins the SPM checksum in the
root `Package.swift` to the released artifact. The release exists before
the pin PR is opened; a maintainer merges that pin PR to activate SPM
resolution for the release. The `ci.yml` "Pin ordering" gate runs on every pull_request or push
that touches `Package.swift`. On pull_request events the gate code is
checked out from the base branch (PR content cannot alter the
enforcement logic); on push events the pushed branch's code runs with
ordinary branch-CI privilege. Either way, a pin
PR that does not advance the pinned release is refused — so other edits
to `Package.swift` require release-team coordination. Recommended
hardening: protect the `pin-spm-checksum-*` branch namespace (restrict
who may push it) — bot pin PRs opened with the workflow token emit no
pull_request CI events, so namespace protection plus the creation-time
rewind guard are the controls for them. Consumers never need a
Rust toolchain.
The major/minor/patch policy: bump the major version for FFI surface
breaks, minor for additive exports, patch for fixes; entries are recorded
in the repository `CHANGELOG.md`.

## Building the iOS XCFramework

The canonical entry point for contributors on macOS with Xcode:

```sh
bash scripts/ios/build-xcframework.sh
```

Outputs `dist/MintlayerWalletBindings.xcframework`, its zip and the
SHA-256 checksum used by the release.

## Building the Android AAR

The canonical entry point for contributors with the Android toolchain
installed (Rust 1.92+, cargo-ndk, JDK 17, Android SDK + NDK, Gradle 8.7+):

```sh
bash scripts/kotlin/build-aar.sh
```

## Building from source

Contributors need Rust 1.92+ (see the repository MSRV):

```sh
cargo build -p mintlayer-wallet-bindings --lib
cargo test -p mintlayer-wallet-bindings
```

Regenerate the foreign-language bindings from the compiled library:

```sh
cargo run -p mintlayer-wallet-bindings --bin uniffi-bindgen -- \
  generate --library target/debug/libmintlayer_wallet_bindings.so \
  --language kotlin --out-dir target/bindings/kotlin
cargo run -p mintlayer-wallet-bindings --bin uniffi-bindgen -- \
  generate --library target/debug/libmintlayer_wallet_bindings.dylib \
  --language swift --out-dir target/bindings/swift
```
