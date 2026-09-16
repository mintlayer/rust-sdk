// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Issue a fungible token through the wallet daemon and mint an initial
//! supply to the authority address.
//!
//! Usage:
//! ```text
//! cargo run --example issue-token --features wallet -- \
//!     --wallet /path/to/wallet.dat \
//!     --password secret \
//!     --ticker MYTOKEN \
//!     --decimals 2 \
//!     --supply 1000000 \
//!     --uri https://example.com/token \
//!     --wallet-rpc http://127.0.0.1:3034 \
//!     --account 0
//! ```
//!
//! Only `--ticker` and `--supply` are required; open the wallet by passing
//! `--wallet`. `--supply` is an atom count (the smallest indivisible unit),
//! not a whole-token count. In production, wait for the issuance
//! transaction to be confirmed before minting.
//!
//! # Secret handling
//!
//! Passing the wallet password as a command-line argument exposes it in
//! shell history and the process list; prefer an environment variable or an
//! interactive prompt. The password is sent to the wallet daemon in the RPC
//! request body, so only point `--wallet-rpc` at a daemon you trust over a
//! local or encrypted connection.

use mintlayer_sdk::wallet::{
    Amount, Client as WalletClient, IssueTokenParams, MintParams, TokenMetadata, TokenSupply,
    TxOptions,
};

struct ParsedArgs {
    wallet: Option<String>,
    password: Option<String>,
    ticker: String,
    decimals: u8,
    supply: String,
    uri: String,
    wallet_rpc: String,
    account: u32,
}

fn parse_args() -> Result<ParsedArgs, String> {
    let mut parsed = ParsedArgs {
        wallet: None,
        password: None,
        ticker: String::new(),
        decimals: 2,
        supply: String::new(),
        uri: String::new(),
        wallet_rpc: "http://127.0.0.1:3034".to_owned(),
        account: 0,
    };

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--wallet" => parsed.wallet = Some(value),
            "--password" => parsed.password = Some(value),
            "--ticker" => parsed.ticker = value,
            "--decimals" => parsed.decimals = value.parse().map_err(|_| "invalid --decimals")?,
            "--supply" => parsed.supply = value,
            "--uri" => parsed.uri = value,
            "--wallet-rpc" => parsed.wallet_rpc = value,
            "--account" => parsed.account = value.parse().map_err(|_| "invalid --account")?,
            other => return Err(format!("unknown flag {other}")),
        }
    }

    if parsed.ticker.is_empty() || parsed.supply.is_empty() {
        return Err("required flags: --ticker, --supply".to_owned());
    }
    Ok(parsed)
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args = parse_args();
    let args = match args {
        Ok(args) => args,
        Err(error) => return Err(error),
    };

    let client = WalletClient::new(&args.wallet_rpc);

    if let Some(path) = &args.wallet {
        client
            .open_wallet(path, args.password.as_deref())
            .await
            .map_err(|error| format!("failed to open wallet: {error}"))?;
        println!("wallet {path} opened");
    } else {
        return Err("no --wallet given; refusing to use an already-open wallet".to_owned());
    }

    if let Err(error) = client.sync_wallet().await {
        eprintln!("warning: wallet sync failed: {error}");
    }

    let authority = client
        .new_address(args.account)
        .await
        .map_err(|error| format!("failed to create address: {error}"))?;
    println!("authority address {authority}");

    if let Some(atoms) = client
        .balance(args.account)
        .await
        .ok()
        .and_then(|balance| balance.coins.atoms())
    {
        println!("account balance: {atoms} atoms");
    }

    let issuance = client
        .issue_token(IssueTokenParams {
            account: args.account,
            destination_address: authority.clone(),
            metadata: TokenMetadata {
                token_ticker: args.ticker.clone(),
                number_of_decimals: args.decimals,
                metadata_uri: args.uri.clone(),
                token_supply: TokenSupply::Lockable,
                is_freezable: false,
            },
            options: TxOptions::default(),
        })
        .await
        .map_err(|error| format!("token issuance failed: {error}"))?;
    println!(
        "issued token {} in transaction {}",
        issuance.token_id, issuance.tx_id
    );

    println!("note: in production wait for the issuance transaction to confirm before minting");

    let mint = client
        .mint_tokens(MintParams {
            account: args.account,
            token_id: issuance.token_id.clone(),
            address: authority,
            amount: Amount::from_atoms(
                args.supply
                    .parse()
                    .map_err(|_| "invalid --supply: must be a decimal atom count")?,
            ),
            options: TxOptions::default(),
        })
        .await
        .map_err(|error| format!("mint failed: {error}"))?;
    println!(
        "minted in transaction {} (fee {} atoms)",
        mint.tx_id,
        mint.fees.coins.atoms().unwrap_or_default()
    );

    Ok(())
}
