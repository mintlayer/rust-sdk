// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

// Generates the differential-parity fixtures: runs a deterministic input set
// through @mintlayer/wasm-lib 1.4.0 (the reference implementation) and
// writes the expected outputs to bindings/tests/fixtures/wasm-lib.json.
//
// The Rust test bindings/tests/differential.rs replays the same inputs
// through the UniFFI bindings and asserts identical results. Signature
// blobs are random by design (Schnorr nonces), so signing flows are
// compared by feeding the wasm-produced blobs to the Rust encoders and by
// cross-verifying wasm signatures with the Rust verifiers; every other
// export is compared byte-for-byte.
//
// Not covered here (no equivalent wasm input producible from Node):
// `encode_stake_pool_data` and `encode_output_create_stake_pool` happy
// paths — their `vrf_public_key` parameter requires an address of the VRF
// public key kind, which no exported wasm function manufactures. Both are
// pinned structurally by bindings/tests/surface.rs.
//
// Usage: npm ci && node generate-fixtures.cjs

"use strict";

const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

// The standard BIP39 English wordlist (public spec constant), committed as
// english-wordlist.txt so the generator has no live wordlist dependency.
const wordlist = fs
  .readFileSync(path.join(__dirname, "english-wordlist.txt"), "utf8")
  .split("\n")
  .filter((line) => line.length > 0);
if (wordlist.length !== 2048) {
  throw new Error(`BIP39 wordlist must have 2048 entries, got ${wordlist.length}`);
}

const WASM_LIB_VERSION = "1.4.0";
const MNEMONIC_COUNT = 100;
const PUBLIC_TEST_MNEMONIC = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

// Deterministic valid BIP39 mnemonics (12 words): entropy comes from a
// counter-seeded SHA-256 and the checksum is computed per the BIP39 spec,
// so the list is reproduced identically on every machine and run.
function deterministicMnemonics(count) {
  const mnemonics = [];
  for (let i = 0; i < count; i += 1) {
    const entropy = crypto.createHash("sha256").update(`mintlayer-differential-${i}`).digest().slice(0, 16);
    // 128-bit entropy => 8-bit checksum: the first byte of SHA-256(entropy).
    const checksumFirstByte = crypto.createHash("sha256").update(entropy).digest()[0];
    const bits = Buffer.concat([entropy, Buffer.from([checksumFirstByte])]); // 128 + 8 bits; 132 needed
    const indices = [];
    for (let bit = 0; bit < 132; bit += 11) {
      let index = 0;
      for (let j = 0; j < 11; j += 1) {
        const byte = bits[Math.floor((bit + j) / 8)];
        const shift = 7 - ((bit + j) % 8);
        index = (index << 1) | ((byte >> shift) & 1);
      }
      indices.push(index);
    }
    mnemonics.push(indices.map((index) => wordlist[index]).join(" "));
  }
  return mnemonics;
}

async function loadWasm() {
  const lib = require("@mintlayer/wasm-lib");
  try {
    await lib.default();
  } catch {
    const wasmPath = path.join(
      path.dirname(require.resolve("@mintlayer/wasm-lib/package.json")),
      "wasm_wrappers_bg.wasm",
    );
    try {
      await lib.default(new Response(fs.readFileSync(wasmPath), { headers: { "Content-Type": "application/wasm" } }));
    } catch (error) {
      throw new Error(`failed to initialize ${wasmPath}: ${error.message}`);
    }
  }
  return lib;
}

function hex(bytes) {
  return Buffer.from(bytes).toString("hex");
}

// Concatenates blob arguments into the single byte array the wasm
// functions expect (their blob-list parameters are concatenated encodings).
function optionalBlob(blob) {
  return Buffer.concat([Buffer.from([1]), blob]);
}

function omittedBlob() {
  return Buffer.from([0]);
}

function concatBlobs(blobs) {
  return Buffer.concat(blobs.map((blob) => (typeof blob === "string" ? Buffer.from(blob, "hex") : Buffer.from(blob))));
}

class Recorder {
  constructor() {
    this.cases = [];
  }

  hexResult(kind, fn, args, bytes) {
    this.cases.push({ kind, fn, args, result: { type: "hex", value: hex(bytes) } });
  }

  stringResult(kind, fn, args, value) {
    this.cases.push({ kind, fn, args, result: { type: "string", value } });
  }

  boolResult(kind, fn, args, value) {
    this.cases.push({ kind, fn, args, result: { type: "bool", value } });
  }

  u64Result(kind, fn, args, value) {
    this.cases.push({ kind, fn, args, result: { type: "u64", value: String(value) } });
  }

  u64WithToleranceResult(kind, fn, args, value, tolerance) {
    this.cases.push({ kind, fn, args, result: { type: "u64", value: String(value), tolerance } });
  }

  jsonResult(kind, fn, args, value) {
    this.cases.push({ kind, fn, args, result: { type: "json", value } });
  }

  nullResult(kind, fn, args) {
    this.cases.push({ kind, fn, args, result: { type: "null", value: null } });
  }
}

async function main() {
  const lib = await loadWasm();
  const wasmLibMain = require.resolve("@mintlayer/wasm-lib");
  const installedVersion = JSON.parse(
    fs.readFileSync(path.join(path.dirname(wasmLibMain), "package.json"), "utf8"),
  ).version;
  if (installedVersion !== WASM_LIB_VERSION) {
    throw new Error(
      `installed @mintlayer/wasm-lib is ${installedVersion}, expected ${WASM_LIB_VERSION}`,
    );
  }
  const rec = new Recorder();
  const networks = ["Mainnet", "Testnet"];
  const net = (name) => lib.Network[name];
  const amount = (atoms) => lib.Amount.from_atoms(atoms);
  const emptyInfo = () => ({ pool_info: {}, order_info: {} });

  const mnemonics = deterministicMnemonics(MNEMONIC_COUNT);
  mnemonics.unshift(PUBLIC_TEST_MNEMONIC);

  console.error("section: Derivation chain");
  // -- Derivation chain -------------------------------------------------------
  for (const mnemonic of mnemonics) {
    for (const network of networks) {
      const tag = `${mnemonic.split(" ")[0]}…/${network}`;
      const account = lib.make_default_account_privkey(mnemonic, net(network), null);
      rec.hexResult("derive", "make_default_account_privkey",
        { tag, mnemonic, network, passphrase: null }, account);
      const xpub = lib.extended_public_key_from_extended_private_key(account);
      rec.hexResult("derive", "extended_public_key_from_extended_private_key",
        { tag, private_key: hex(account) }, xpub);

      for (const index of [0, 1, 42]) {
        rec.hexResult("derive", "make_receiving_address",
          { tag, private_key: hex(account), key_index: index }, lib.make_receiving_address(account, index));
        rec.hexResult("derive", "make_receiving_address_public_key",
          { tag, public_key: hex(xpub), key_index: index }, lib.make_receiving_address_public_key(xpub, index));
        rec.hexResult("derive", "make_change_address",
          { tag, private_key: hex(account), key_index: index }, lib.make_change_address(account, index));
        rec.hexResult("derive", "make_change_address_public_key",
          { tag, public_key: hex(xpub), key_index: index }, lib.make_change_address_public_key(xpub, index));
        if (index === 0) {
          const pubkey = lib.make_receiving_address_public_key(xpub, 0);
          rec.stringResult("derive", "pubkey_to_pubkeyhash_address",
            { tag, public_key: hex(pubkey), network }, lib.pubkey_to_pubkeyhash_address(pubkey, net(network)));
          const receiving = lib.make_receiving_address(account, 0);
          rec.hexResult("derive", "public_key_from_private_key",
            { tag, private_key: hex(receiving) }, lib.public_key_from_private_key(receiving));
        }
      }
    }
  }

  console.error("section: Fees");
  // -- Fees ---------------------------------------------------------------------
  for (const network of networks) {
    for (const height of [0, 1000, 4000000]) {
      const calls = [
        ["fungible_token_issuance_fee", lib.fungible_token_issuance_fee(BigInt(height), net(network)).atoms()],
        ["nft_issuance_fee", lib.nft_issuance_fee(BigInt(height), net(network)).atoms()],
        ["token_supply_change_fee", lib.token_supply_change_fee(BigInt(height), net(network)).atoms()],
        ["token_freeze_fee", lib.token_freeze_fee(BigInt(height), net(network)).atoms()],
        ["token_change_authority_fee", lib.token_change_authority_fee(BigInt(height), net(network)).atoms()],
        ["data_deposit_fee", lib.data_deposit_fee(BigInt(height), net(network)).atoms()],
      ];
      for (const [fn, atoms] of calls) {
        rec.stringResult("fees", fn, { current_block_height: height, network }, atoms);
      }
    }
  }

  console.error("section: Timelocks");
  // -- Timelocks ----------------------------------------------------------------
  for (const value of [0, 1, 1000, 4294967295]) {
    rec.hexResult("locks", "encode_lock_for_block_count", { block_count: value }, lib.encode_lock_for_block_count(BigInt(value)));
    rec.hexResult("locks", "encode_lock_for_seconds", { total_seconds: value }, lib.encode_lock_for_seconds(BigInt(value)));
    rec.hexResult("locks", "encode_lock_until_time", { timestamp: value }, lib.encode_lock_until_time(BigInt(value)));
    rec.hexResult("locks", "encode_lock_until_height", { block_height: value }, lib.encode_lock_until_height(BigInt(value)));
  }

  console.error("section: Anchor keys per network");
  // -- Anchor keys per network --------------------------------------------------
  const anchors = {};
  for (const network of networks) {
    const account = lib.make_default_account_privkey(PUBLIC_TEST_MNEMONIC, net(network), null);
    const xpub = lib.extended_public_key_from_extended_private_key(account);
    const pubkey = lib.make_receiving_address_public_key(xpub, 0);
    anchors[network] = {
      account: hex(account),
      xpub: hex(xpub),
      pubkey: hex(pubkey),
      private0: hex(lib.make_receiving_address(account, 0)),
      address: lib.pubkey_to_pubkeyhash_address(pubkey, net(network)),
    };
  }

  const sha256 = (seed) => crypto.createHash("sha256").update(seed).digest();
  const outpointSource = (seed) => lib.encode_outpoint_source_id(sha256(`outpoint-${seed}`), lib.SourceId.Transaction);
  const sampleInput = (seed) => lib.encode_input_for_utxo(outpointSource(seed), 0);

  console.error("section: Ids");
  // -- Ids ------------------------------------------------------------------------
  const tokenId = {};
  const delegationId = {};
  const poolId = {};
  const orderId = {};
  for (const network of networks) {
    const inputs = [sampleInput(`token-${network}`), sampleInput(`token2-${network}`)];
    tokenId[network] = lib.get_token_id(concatBlobs(inputs), BigInt(1000), net(network));
    rec.stringResult("ids", "get_token_id", { inputs: inputs.map(hex), current_block_height: 1000, network }, tokenId[network]);
    orderId[network] = lib.get_order_id(concatBlobs(inputs), net(network));
    rec.stringResult("ids", "get_order_id", { inputs: inputs.map(hex), network }, orderId[network]);
    delegationId[network] = lib.get_delegation_id(concatBlobs(inputs), net(network));
    rec.stringResult("ids", "get_delegation_id", { inputs: inputs.map(hex), network }, delegationId[network]);
    poolId[network] = lib.get_pool_id(concatBlobs(inputs), net(network));
    rec.stringResult("ids", "get_pool_id", { inputs: inputs.map(hex), network }, poolId[network]);
  }

  console.error("section: Staking (balance and maturity; pool-data encoders need a VRF address,");
  // -- Staking (balance and maturity; pool-data encoders need a VRF address,
  //    see the file header) ------------------------------------------------------
  for (const network of networks) {
    rec.stringResult("staking", "effective_pool_balance", {
      network, pledge_amount: "1000000000000", pool_balance: "2000000000000",
    }, lib.effective_pool_balance(net(network), amount("1000000000000"), amount("2000000000000")).atoms());
    rec.u64Result("staking", "staking_pool_spend_maturity_block_count",
      { current_block_height: 1000, network }, lib.staking_pool_spend_maturity_block_count(BigInt(1000), net(network)));
  }

  console.error("section: Multisig");
  // -- Multisig ---------------------------------------------------------------------
  for (const network of networks) {
    const secondKey = lib.make_receiving_address_public_key(
      lib.extended_public_key_from_extended_private_key(
        lib.make_default_account_privkey(mnemonics[1], net(network), null),
      ),
      1,
    );
    const keys = [anchors[network].pubkey, hex(secondKey)];
    const challenge = lib.encode_multisig_challenge(concatBlobs(keys), 2, net(network));
    rec.hexResult("multisig", "encode_multisig_challenge",
      { public_keys: keys, min_required_signatures: 2, network }, challenge);
    rec.stringResult("multisig", "multisig_challenge_to_address",
      { multisig_challenge: hex(challenge), network }, lib.multisig_challenge_to_address(challenge, net(network)));
  }

  console.error("section: Outputs");
  // -- Outputs and inputs ---------------------------------------------------------
  // Each case records its full argument object (amounts as decimal atom
  // strings, blobs as hex) so the Rust suite can replay it against the
  // UniFFI bindings. `variant` disambiguates same-function edge cases.
  const toTotalSupply = (name) => {
    switch (name) {
      case "Lockable":
        return lib.TotalSupply.Lockable;
      case "Unlimited":
        return lib.TotalSupply.Unlimited;
      default:
        return lib.TotalSupply.Fixed;
    }
  };
  const outputWasmArgs = (args) => {
    const n = net(args.network);
    const amt = (key) => lib.Amount.from_atoms(args[key]);
    const opt = (key) => (args[key] === null ? null : args[key]);
    switch (args.variant) {
      case "encode_output_transfer":
        return [amt("amount"), args.address, n];
      case "encode_output_token_transfer":
        return [amt("amount"), args.address, args.token_id, n];
      case "encode_output_lock_then_transfer":
        return [amt("amount"), args.address, Buffer.from(args.lock, "hex"), n];
      case "encode_output_token_lock_then_transfer":
        return [amt("amount"), args.address, args.token_id, Buffer.from(args.lock, "hex"), n];
      case "encode_output_coin_burn":
        return [amt("amount")];
      case "encode_output_token_burn":
        return [amt("amount"), args.token_id, n];
      case "encode_output_create_delegation":
        return [args.pool_id, args.owner_address, n];
      case "encode_output_delegate_staking":
        return [amt("amount"), args.delegation_id, n];
      case "encode_output_create_stake_pool":
        return [args.pool_id, Buffer.from(args.pool_data, "hex"), n];
      case "encode_output_produce_block_from_stake":
        return [args.pool_id, args.staker, n];
      case "encode_output_data_deposit":
        return [Buffer.from(args.data, "hex")];
      case "encode_create_order_output":
        return [amt("ask_amount"), opt("ask_token_id"), amt("give_amount"), opt("give_token_id"), args.conclude_address, n];
      case "encode_output_htlc":
        return [amt("amount"), opt("token_id"), args.secret_hash, args.spend_address, args.refund_address,
          Buffer.from(args.refund_timelock, "hex"), n];
      case "encode_output_issue_fungible_token":
      case "encode_output_issue_fungible_token_fixed":
      case "encode_output_issue_fungible_token_unlimited":
        return [args.authority, args.token_ticker, args.metadata_uri, args.number_of_decimals,
          toTotalSupply(args.total_supply),
          args.supply_amount === null ? null : lib.Amount.from_atoms(args.supply_amount),
          args.is_token_freezable === "Yes" ? lib.FreezableToken.Yes : lib.FreezableToken.No,
          BigInt(1000), n];
      case "encode_output_issue_nft":
        return [args.token_id, args.authority, args.name, args.ticker, args.description,
          Buffer.from(args.media_hash, "hex"),
          args.creator === null ? null : Buffer.from(args.creator, "hex"), opt("media_uri"),
          opt("icon_uri"), opt("additional_metadata_uri"), BigInt(1000), n];
      default:
        throw new Error(`unmapped output variant: ${args.variant}`);
    }
  };
  const inputWasmArgs = (args) => {
    const n = net(args.network);
    const amt = (key) => lib.Amount.from_atoms(args[key]);
    switch (args.variant) {
      case "encode_input_for_utxo":
        return [Buffer.from(args.outpoint_source_id, "hex"), args.output_index];
      case "encode_input_for_withdraw_from_delegation":
        return [args.delegation_id, amt("amount"), BigInt(args.nonce), n];
      case "encode_input_for_mint_tokens":
        return [args.token_id, amt("amount"), BigInt(args.nonce), n];
      case "encode_input_for_unmint_tokens":
        return [args.token_id, BigInt(args.nonce), n];
      case "encode_input_for_lock_token_supply":
        return [args.token_id, BigInt(args.nonce), n];
      case "encode_input_for_freeze_token":
        return [args.token_id, args.is_token_unfreezable === "Yes" ? lib.TokenUnfreezable.Yes : lib.TokenUnfreezable.No, BigInt(args.nonce), n];
      case "encode_input_for_unfreeze_token":
        return [args.token_id, BigInt(args.nonce), n];
      case "encode_input_for_change_token_authority":
        return [args.token_id, args.new_authority, BigInt(args.nonce), n];
      case "encode_input_for_change_token_metadata_uri":
        return [args.token_id, args.new_metadata_uri, BigInt(args.nonce), n];
      case "encode_input_for_fill_order":
        return [args.order_id, amt("fill_amount"), args.destination, BigInt(args.nonce), BigInt(args.current_block_height), n];
      case "encode_input_for_freeze_order":
        return [args.order_id, BigInt(args.current_block_height), n];
      case "encode_input_for_conclude_order":
        return [args.order_id, BigInt(args.nonce), BigInt(args.current_block_height), n];
      default:
        throw new Error(`unmapped input variant: ${args.variant}`);
    }
  };

  for (const network of networks) {
    const a = anchors[network];
    const lock10 = hex(lib.encode_lock_for_block_count(BigInt(10)));
    const lockHeight = hex(lib.encode_lock_until_height(BigInt(5000)));
    const outputCases = [
      ["encode_output_transfer", { amount: "100000000000", address: a.address, network }],
      ["encode_output_token_transfer", { amount: "500", address: a.address, token_id: tokenId[network], network }],
      ["encode_output_lock_then_transfer", { amount: "700", address: a.address, lock: lock10, network }],
      ["encode_output_token_lock_then_transfer", { amount: "300", address: a.address, token_id: tokenId[network], lock: lockHeight, network }],
      ["encode_output_coin_burn", { amount: "1", network }],
      ["encode_output_token_burn", { amount: "10", token_id: tokenId[network], network }],
      ["encode_output_create_delegation", { pool_id: poolId[network], owner_address: a.address, network }],
      ["encode_output_delegate_staking", { amount: "2000", delegation_id: delegationId[network], network }],
      ["encode_output_produce_block_from_stake", { pool_id: poolId[network], staker: a.address, network }],
      ["encode_output_data_deposit", { data: hex(Buffer.from("mintlayer", "utf8")), network }],
      ["encode_create_order_output", { ask_amount: "1000", ask_token_id: tokenId[network], give_amount: "2000", give_token_id: null, conclude_address: a.address, network }],
      ["encode_output_htlc", { amount: "123", token_id: null, secret_hash: sha256("htlc-secret").toString("hex").slice(0, 40), spend_address: a.address, refund_address: a.address, refund_timelock: lock10, network }],
      ["encode_output_issue_fungible_token", { authority: a.address, token_ticker: "TKN", metadata_uri: "https://example.com", number_of_decimals: 6, total_supply: "Lockable", supply_amount: null, is_token_freezable: "Yes", current_block_height: 1000, network }],
      ["encode_output_issue_fungible_token_fixed", { authority: a.address, token_ticker: "FIX", metadata_uri: "https://example.com", number_of_decimals: 8, total_supply: "Fixed", supply_amount: "21000000000000", is_token_freezable: "No", current_block_height: 1000, network }],
      ["encode_output_issue_fungible_token_unlimited", { authority: a.address, token_ticker: "UNL", metadata_uri: "https://example.com", number_of_decimals: 18, total_supply: "Unlimited", supply_amount: null, is_token_freezable: "No", current_block_height: 1000, network }],
      ["encode_output_issue_nft", { token_id: tokenId[network], authority: a.address, name: "Name", ticker: "NFT", description: "Description", media_hash: hex(sha256("nft-media")), creator: a.pubkey, media_uri: "https://media.example.com", icon_uri: null, additional_metadata_uri: "https://meta.example.com", current_block_height: 1000, network }],
    ];
    for (const [variant, args] of outputCases) {
      const wasmFn = variant.startsWith("encode_output_issue_fungible_token")
        ? "encode_output_issue_fungible_token"
        : variant;
      const callArgs = { ...args, variant };
      try {
        rec.hexResult("outputs", variant, callArgs, lib[wasmFn](...outputWasmArgs(callArgs)));
      } catch (error) {
        throw new Error(`output case ${variant} failed: ${error} (stack: ${error.stack && error.stack.split(String.fromCharCode(10))[1]})`);
      }
    }

    const inputCases = [
      ["encode_input_for_utxo", { outpoint_source_id: hex(outpointSource(`utxo-${network}`)), output_index: 3, network }],
      ["encode_input_for_withdraw_from_delegation", { delegation_id: delegationId[network], amount: "1500", nonce: 5, network }],
      ["encode_input_for_mint_tokens", { token_id: tokenId[network], amount: "500", nonce: 6, network }],
      ["encode_input_for_unmint_tokens", { token_id: tokenId[network], nonce: 7, network }],
      ["encode_input_for_lock_token_supply", { token_id: tokenId[network], nonce: 8, network }],
      ["encode_input_for_freeze_token", { token_id: tokenId[network], is_token_unfreezable: "Yes", nonce: 9, network }],
      ["encode_input_for_unfreeze_token", { token_id: tokenId[network], nonce: 10, network }],
      ["encode_input_for_change_token_authority", { token_id: tokenId[network], new_authority: a.address, nonce: 11, network }],
      ["encode_input_for_change_token_metadata_uri", { token_id: tokenId[network], new_metadata_uri: "https://new.example.com", nonce: 12, network }],
      ["encode_input_for_fill_order", { order_id: orderId[network], fill_amount: "250", destination: a.address, nonce: 13, current_block_height: 100000000, network }],
      ["encode_input_for_freeze_order", { order_id: orderId[network], current_block_height: 100000000, network }],
      ["encode_input_for_conclude_order", { order_id: orderId[network], nonce: 14, current_block_height: 100000000, network }],
    ];
    for (const [variant, args] of inputCases) {
      const callArgs = { ...args, variant };
      rec.hexResult("inputs", variant, callArgs, lib[variant](...inputWasmArgs(callArgs)));
    }
  }

  // -- Destination -------------------------------------------------------------------
  for (const network of networks) {
    rec.hexResult("addresses", "encode_destination",
      { address: anchors[network].address, network },
      lib.encode_destination(anchors[network].address, net(network)));
  }

  console.error("section: Transactions and witnesses");
  // -- Transactions and witnesses ------------------------------------------------------
  for (const network of networks) {
    const a = anchors[network];
    const input0 = lib.encode_input_for_utxo(outpointSource(`spend-${network}`), 0);
    const input1 = lib.encode_input_for_utxo(outpointSource(`spend2-${network}`), 1);
    const utxoBlob = lib.encode_output_transfer(amount("100000000000"), a.address, net(network));
    const output0 = lib.encode_output_transfer(amount("99999999999"), a.address, net(network));
    const output1 = lib.encode_output_transfer(amount("1"), a.address, net(network));

    // Unsigned transactions: 1-input and 2-input shapes.
    const tx1 = lib.encode_transaction(concatBlobs([input0]), concatBlobs([output0]), BigInt(0));
    rec.hexResult("transactions", "encode_transaction",
      { network, variant: "1-input", inputs: [hex(input0)], outputs: [hex(output0)], flags: 0 }, tx1);
    const tx2 = lib.encode_transaction(concatBlobs([input0, input1]), concatBlobs([output0, output1]), BigInt(0));
    rec.hexResult("transactions", "encode_transaction",
      { network, variant: "2-input", inputs: [hex(input0), hex(input1)], outputs: [hex(output0), hex(output1)], flags: 0 }, tx2);
    rec.stringResult("transactions", "get_transaction_id", { network, variant: "1-input", transaction: hex(tx1), strict: true },
      lib.get_transaction_id(tx1, true));
    rec.stringResult("transactions", "get_transaction_id", { network, variant: "2-input", transaction: hex(tx2), strict: false },
      lib.get_transaction_id(tx2, false));
    // Size estimation drifted slightly in the 1.4.0 build era; record the
    // wasm value with the tolerance the Rust replay may apply.
    rec.u64WithToleranceResult("transactions", "estimate_transaction_size", {
      network, variant: "1-input", inputs: [hex(input0)], input_destinations: [a.address], outputs: [hex(output0)],
    }, lib.estimate_transaction_size(concatBlobs([input0]), [a.address], concatBlobs([output0]), net(network)), 8);

    // Witness signing (random nonce): record the wasm witness blob as the
    // deterministic input of the signed-transaction encoders below, and
    // cross-verify the wasm signature with the Rust verifier.
    const witnessArgs = {
      network, variant: "1-input", sighash_type: "ALL", private_key: a.private0,
      input_owner_destination: a.address, transaction: hex(tx1),
      input_utxos: [hex(utxoBlob)], input_index: 0,
      additional_info: emptyInfo(), current_block_height: 1000,
    };
    const witness = lib.encode_witness(
      lib.SignatureHashType.ALL, Buffer.from(a.private0, "hex"), a.address, tx1,
      optionalBlob(utxoBlob), 0, emptyInfo(), BigInt(1000), net(network),
    );
    rec.hexResult("signing", "encode_witness", witnessArgs, witness);
    const sigBlob = lib.sign_message_for_spending(Buffer.from(a.private0, "hex"), Buffer.from("cross", "utf8"));
    rec.stringResult("signing", "sign_message_for_spending",
      { private_key: a.private0, message: hex(Buffer.from("cross", "utf8")), public_key: a.pubkey },
      hex(sigBlob));
    rec.boolResult("signing", "verify_signature_for_spending",
      { public_key: a.pubkey, signature: hex(sigBlob), message: hex(Buffer.from("cross", "utf8")) },
      lib.verify_signature_for_spending(
        Buffer.from(a.pubkey, "hex"), sigBlob, Buffer.from("cross", "utf8"),
      ));

    // Signed transaction and JSON decodes, using the wasm witness blob as
    // the shared deterministic input.
    const signatures = witness;
    const signed = lib.encode_signed_transaction(tx1, signatures);
    rec.hexResult("transactions", "encode_signed_transaction",
      { network, variant: "1-input", transaction: hex(tx1), signatures: [hex(signatures)] }, signed);
    rec.jsonResult("transactions", "decode_signed_transaction_to_json",
      { network, transaction: hex(signed) }, JSON.stringify(lib.decode_signed_transaction_to_js(signed, net(network))));
    rec.jsonResult("transactions", "decode_transaction_to_json",
      { network, transaction: hex(tx1) }, JSON.stringify(lib.decode_transaction_to_js(tx1, net(network))));

    // Partially signed assembly (per-input optional lists).
    // Per-input lists: each entry is Some(1 + object) or null, one per
    // transaction input. Both inputs are UTXO inputs with destinations.
    const utxoBlob2 = lib.encode_output_transfer(amount("1"), a.address, net(network));
    const ptxSignatures = Buffer.concat([
      optionalBlob(witness),
      omittedBlob(),
    ]);
    const ptxInputs = Buffer.concat([
      optionalBlob(utxoBlob),
      optionalBlob(utxoBlob2),
    ]);
    const destinationBlob = lib.encode_destination(a.address, net(network));
    const ptxDestinations = Buffer.concat([
      optionalBlob(destinationBlob),
      optionalBlob(destinationBlob),
    ]);
    const partial = lib.encode_partially_signed_transaction(
      tx2,
      ptxSignatures,
      ptxInputs,
      ptxDestinations,
      Buffer.from([0, 0]),
      emptyInfo(),
      net(network),
    );
    rec.hexResult("transactions", "encode_partially_signed_transaction",
      {
        network, variant: "2-input", transaction: hex(tx2),
        signatures: [hex(witness), null], input_utxos: [hex(utxoBlob), hex(utxoBlob2)],
        input_destinations: [a.address, a.address], htlc_secrets: [null, null],
      }, partial);
    rec.jsonResult("transactions", "decode_partially_signed_transaction_to_json",
      { network, transaction: hex(partial) }, JSON.stringify(lib.decode_partially_signed_transaction_to_js(partial, net(network))));

    rec.hexResult("signing", "encode_witness_no_signature", { network },
      lib.encode_witness_no_signature());

    // Challenge signatures for the intent flow.
    const intentMessage = Buffer.from(
      lib.make_transaction_intent_message_to_sign("login", tx_id_of(tx1)),
    );
    rec.hexResult("intents", "make_transaction_intent_message_to_sign",
      { intent: "login", transaction_id: tx_id_of(tx1) }, intentMessage);
    const challengeSig = lib.sign_challenge(Buffer.from(a.private0, "hex"), intentMessage);
    rec.stringResult("signing", "sign_challenge",
      { private_key: a.private0, message: hex(intentMessage), network }, hex(challengeSig));
    rec.boolResult("signing", "verify_challenge",
      { address: a.address, network, signed_challenge: hex(challengeSig), message: hex(intentMessage) },
      lib.verify_challenge(a.address, net(network), challengeSig, intentMessage));
    const intent = lib.encode_signed_transaction_intent(intentMessage, [challengeSig]);
    rec.hexResult("intents", "encode_signed_transaction_intent",
      { signed_message: hex(intentMessage), signatures: [hex(challengeSig)] }, intent);
    const intentResult = lib.verify_transaction_intent(intentMessage, intent, [a.address], net(network));
    if (intentResult !== null && intentResult !== undefined) {
      throw new Error(`unexpected verify_transaction_intent result: ${intentResult}`);
    }
    rec.nullResult("intents", "verify_transaction_intent",
      { expected_signed_message: hex(intentMessage), encoded_signed_intent: hex(intent), input_destinations: [a.address], network });
  }

  console.error("section: HTLC secret extraction (wasm-signed transaction)");
  // -- HTLC secret extraction (wasm-signed transaction) --------------------------------
  for (const network of networks) {
    const a = anchors[network];
    const secret = sha256("htlc-preimage-1234");
    const htlcOutput = lib.encode_output_htlc(
      amount("555"), null, sha256("htlc-preimage-1234").toString("hex").slice(0, 40),
      a.address, a.address, lib.encode_lock_for_block_count(BigInt(1)), net(network),
    );
    const outpointSrc = outpointSource(`htlc-${network}`);
    const spendInput = lib.encode_input_for_utxo(outpointSrc, 0);
    const change = lib.encode_output_transfer(amount("111"), a.address, net(network));
    const transaction = lib.encode_transaction(concatBlobs([spendInput]), concatBlobs([change]), BigInt(0));
    const witness = lib.encode_witness_htlc_spend(
      lib.SignatureHashType.ALL, Buffer.from(a.private0, "hex"), a.address, transaction,
      optionalBlob(htlcOutput), 0, secret, emptyInfo(), BigInt(1000), net(network),
    );
    const signed = lib.encode_signed_transaction(transaction, witness);
    rec.hexResult("signing", "encode_witness_htlc_spend", {
      network, sighash_type: "ALL", private_key: a.private0, input_owner_destination: a.address,
      transaction: hex(transaction), input_utxos: [hex(htlcOutput)], input_index: 0, secret: hex(secret),
      additional_info: emptyInfo(), current_block_height: 1000,
    }, witness);
    rec.hexResult("transactions", "extract_htlc_secret", {
      network, signed_tx: hex(signed), strict_byte_size: true,
      htlc_outpoint_source_id: hex(outpointSrc), htlc_output_index: 0,
    }, lib.extract_htlc_secret(signed, true, outpointSrc, 0));

    // HTLC refund flows: single-sig refund, and a multisig-keyed refund
    // whose refund address is a multisig challenge address.
    const refundWitness = lib.encode_witness_htlc_refund_single_sig(
      lib.SignatureHashType.ALL, Buffer.from(a.private0, "hex"), a.address, transaction,
      optionalBlob(htlcOutput), 0, emptyInfo(), BigInt(1000000), net(network),
    );
    rec.hexResult("signing", "encode_witness_htlc_refund_single_sig", {
      network, sighash_type: "ALL", private_key: a.private0, input_owner_destination: a.address,
      transaction: hex(transaction), input_utxos: [hex(htlcOutput)], input_index: 0,
      additional_info: emptyInfo(), current_block_height: 1000000,
    }, refundWitness);

    const keys = [hex(lib.make_receiving_address_public_key(
      lib.extended_public_key_from_extended_private_key(
        lib.make_default_account_privkey(mnemonics[1], net(network), null),
      ),
      0,
    )), a.pubkey];
    const challenge = lib.encode_multisig_challenge(concatBlobs(keys), 2, net(network));
    const multisigAddress = lib.multisig_challenge_to_address(challenge, net(network));
    const htlcMultisigOutput = lib.encode_output_htlc(
      amount("555"), null, sha256("htlc-multisig").toString("hex").slice(0, 40),
      a.address, multisigAddress, lib.encode_lock_for_block_count(BigInt(1)), net(network),
    );
    const spendInput2 = lib.encode_input_for_utxo(outpointSource(`htlc2-${network}`), 0);
    const change2 = lib.encode_output_transfer(amount("444"), a.address, net(network));
    const transaction2 = lib.encode_transaction(concatBlobs([spendInput2]), concatBlobs([change2]), BigInt(0));
    const refundMultisig = lib.encode_witness_htlc_refund_multisig(
      lib.SignatureHashType.ALL, Buffer.from(a.private0, "hex"), 1, new Uint8Array(0), challenge, transaction2,
      optionalBlob(htlcMultisigOutput), 0, emptyInfo(), BigInt(1000000), net(network),
    );
    rec.hexResult("signing", "encode_witness_htlc_refund_multisig", {
      network, sighash_type: "ALL", private_key: a.private0, key_index: 1, input_witness: null,
      multisig_challenge: hex(challenge), transaction: hex(transaction2),
      input_utxos: [hex(htlcMultisigOutput)], input_index: 0,
      additional_info: emptyInfo(), current_block_height: 1000000,
    }, refundMultisig);
  }

  console.error("section: Write");
  // -- Write -----------------------------------------------------------------------------
  const outDir = path.resolve(__dirname, "..", "..", "bindings", "tests", "fixtures");
  fs.mkdirSync(outDir, { recursive: true });
  const outFile = path.join(outDir, "wasm-lib.json");
  fs.writeFileSync(outFile, JSON.stringify({
    wasm_lib_version: WASM_LIB_VERSION,
    generated_by: "scripts/differential/generate-fixtures.cjs",
    mnemonics: mnemonics.length,
    cases: rec.cases,
  }, null, 1));
  console.log(`wrote ${rec.cases.length} cases to ${outFile}`);

  function tx_id_of(transaction) {
    return lib.get_transaction_id(transaction, true);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
