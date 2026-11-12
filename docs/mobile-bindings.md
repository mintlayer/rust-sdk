# Mobile bindings (Kotlin and Swift)

Native mobile bindings for the Mintlayer wallet key and address
primitives, generated with [UniFFI](https://mozilla.github.io/uniffi-rs/)
from the `mintlayer-wallet-bindings` crate.

For mobile applications these bindings are the recommended integration and
replace the WASM wrappers: React Native's Hermes engine does not provide a
`WebAssembly` global, so the WASM path is unusable there, while the native
bindings run everywhere. The WebAssembly route remains available for
browser use cases and its behavior is unchanged.

The exported surface mirrors the `wasm-bindings` exports operation for
operation — same function names, same parameter shapes, same runtime blob
formats — so existing key and address handling ports to the native bindings
without modification. All outputs are byte-for-byte identical to the WASM
wrappers; parity is pinned by tests against captured reference outputs of
`@mintlayer/wasm-lib` 1.4.0 (see "Parity guarantees" below).

## API surface

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

Every fallible function returns a typed error (`BindingsException` in
Kotlin, `BindingsError` in Swift); nothing panics across the FFI boundary.
There is no storage, no networking and no mnemonic generation: the library
is a pure cryptography and encoding primitive.

## Data formats

- Extended private keys, extended public keys, plain private keys and
  public keys cross the boundary as byte arrays in the runtime SCALE
  format — the same bytes the WASM wrappers produce and consume.
- Addresses are bech32 strings (`mtc1…` mainnet, `tmt1…` testnet).
- `Network` is `mainnet` / `testnet` / `regtest` / `signet`.

## Kotlin (Android)

The generated Kotlin lives under `org.mintlayer.wallet.bindings`. The
Android artifact will be an AAR containing the native libraries for
`arm64-v8a`, `armeabi-v7a` and `x86_64`; it ships with the first tagged
release (see "Versioning").

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

The iOS artifact will be an XCFramework covering iOS devices
(`ios-arm64`) and simulators (`ios-arm64_x86_64-simulator`), exposed as the
`MintlayerWalletBindings` module; it ships with the first tagged release
(see "Versioning").

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
agreement at every tested index. CI runs the suite on every pull request
and re-verifies parity against the WASM reference on a fixed input set.

## Versioning

Binding releases will be tagged `wallet-bindings-vX.Y.Z` and will ship
prebuilt artifacts (Android AAR, iOS XCFramework) with SHA-256 checksums
attached to the GitHub release, so consumers never need a Rust toolchain.
The major/minor/patch policy: bump the major version for FFI surface
breaks, minor for additive exports, patch for fixes; entries are recorded
in the repository `CHANGELOG.md`.

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
