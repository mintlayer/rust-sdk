# Changelog

All notable changes to the Mintlayer Rust SDK are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- bindings: new `mintlayer-wallet-bindings` crate exposing the Mintlayer
  wallet primitives to Kotlin and Swift via UniFFI, mirroring the full
  wasm-bindings surface: mnemonic → default account extended private key,
  receiving/change derivation (private and watch-only public),
  public-key/address encoding, extended-private-key decode/encode with
  roundtrip guarantees, transaction input/output construction, witness
  signing, message and challenge signatures, transaction intents, protocol
  fees, timelock encodings, token/pool/delegation/order id derivation,
  stake pool helpers and multisig challenges. Blob outputs are
  byte-for-byte identical to `@mintlayer/wasm-lib` 1.4.0; parity is pinned
  by tests on the public BIP39 test vector. The generated Kotlin package is
  `org.mintlayer.wallet.bindings`; the Swift module is
  `MintlayerWalletBindings`.
- indexer: cursor (keyset) pagination for api-server v2
  (mintlayer-core PR #2130): `Page<T>`, the reusable `Pager` walker
  (`*_pager` constructors for pools, the global transaction listing, both
  holders listings and the order book), the `*_paged`/`*_pager` listing
  methods with explicit cursors, `list_transactions_with_offset_mode`
  (`OffsetMode::Legacy` / `Absolute`), the holders listings
  (`coin_holders`, `token_holders`, `Holder`) and the order book
  (`order_pair_book`, `OrderBook`/`OrderBookLevel`/`OrderBookPrice` with
  the `truncated` invariant and side-specific cursors).
- indexer: typed server errors `Error::InvalidCursor`,
  `Error::InvalidNumItems`, `Error::BadRequest` and `Error::TokenNotFound`,
  mapped from the api-server error bodies (`{"error": ...}`).
- `examples/indexer-pagination.rs`: runnable walks for holders, the order
  book, the global transaction listing and pools.

- `node::Error::OutcomeUnknown` / `wallet::Error::OutcomeUnknown`: the
  request was delivered but its outcome could not be confirmed (lost,
  oversized, undecodable or mismatched response, or a timeout while
  waiting for the response), so a mutating call may already have taken
  effect. `IdMismatch`, `Json`, `ResponseTooLarge` and ambiguous
  transport errors now arrive wrapped in this variant; only failures that
  never established a connection (e.g. connection refused, DNS resolution
  failure) surface as the plain transport variant.

### Changed

- crypto: the `ml-*` mintlayer-core git dependencies are re-pinned from
  rev `7f4393ac` to `3a43aba00` (the v1.4.5 release line plus the upstream
  testnet checkpoint fix). The v1.4.5 tag itself contains two malformed
  hex hashes in the testnet checkpoint data that make any testnet chain
  config construction panic; upstream commit `3a43aba00` corrects them.
  No SDK-facing behavior changes otherwise; the full test suite passes
  unchanged.
- **Breaking:** indexer `Transaction.block_id`, `Transaction.timestamp`
  and `Transaction.confirmations` are now `Option<...>`: a pending
  (mempool) transaction has `null` for all three on the wire, which
  previously failed to decode; update direct field uses.
- **Breaking:** post-delivery transport failures surface as
  `OutcomeUnknown(...)` instead of the bare variant, so a retry on `Err`
  can no longer silently double a fund-moving mutation; update matches on
  `IdMismatch` / `Json` / `ResponseTooLarge` to unwrap the new variant.

### Fixed

- Daemon-controlled JSON-RPC error messages are sanitized before they
  reach the public error `Display`: control characters are stripped and
  the message is capped at 8 KiB (matching the indexer client), defusing
  forged multi-line log entries and terminal escape injection. The limit
  and sanitizer live in the shared `limits` module so the indexer and
  JSON-RPC transports cannot drift.

## [0.1.0] - 2026-09-17

### Added

- `node` sub-client: JSON-RPC 2.0 client for the node daemon with the full
  chainstate, mempool, node and P2P surface (including
  `p2p_submit_transaction` broadcast).
- `indexer` sub-client: REST client for api-web-server covering chain,
  block, transaction, address, pool, delegation, token, NFT, order and
  statistics endpoints.
- `wallet` sub-client: JSON-RPC 2.0 client for the wallet daemon covering
  management, transactions, staking, tokens and DEX orders.
- `crypto` module: native cryptography and transaction building backed by
  mintlayer-core — BIP39/BIP32 key derivation, addresses, timelocks, fees,
  id derivation, all input/output constructors, stake pool data, witness
  production (standard, HTLC spend/refund single-sig and multisig),
  partially signed transactions, size estimation, HTLC secret extraction
  and signed transaction intents.
- Umbrella `Client` with a builder that wires the remote sub-clients
  together and a `prelude` of shared type re-exports.
- Examples: `send-coins` (native signing flow) and `issue-token` (wallet
  daemon flow).
- Wire-format integration tests for every sub-client, and consensus-vector
  tests for the crypto module (BIP39 legacy derivations, transaction ids,
  fees, timelocks).

### Security

- Basic-auth credentials and mnemonics/passphrases are redacted from `Debug`
  output across all client and builder types.
- Daemon response bodies are capped at 64 MiB; HTTP error bodies are
  additionally truncated and stripped of control characters.
- JSON-RPC responses must echo the request id.
- Indexer path segments are validated against a safe charset before being
  interpolated into request paths.
- The BIP39 seed and passphrase are zeroized on drop.
