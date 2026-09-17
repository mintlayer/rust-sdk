# Changelog

All notable changes to the Mintlayer Rust SDK are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
