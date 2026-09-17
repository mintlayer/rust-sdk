// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wallet lifecycle RPC methods: creation, opening, accounts, addresses.

use serde::{Deserialize, Serialize};

use super::{
    AccountInfo, AddressWithUsage, Balance, BestBlock, Client, CreateWalletParams,
    CreateWalletResult, Error, RecoverWalletParams, RevealPublicKey, WalletInfo,
};

#[derive(Serialize)]
struct OpenWalletParams<'a> {
    path: &'a str,
    password: Option<&'a str>,
    force_migrate_wallet_type: Option<()>,
    hardware_wallet: Option<()>,
}

#[derive(Serialize)]
struct RenameAccountParams {
    account: u32,
    name: Option<String>,
}

#[derive(Serialize)]
struct BalanceParams {
    account: u32,
    utxo_states: [&'static str; 1],
    with_locked: Option<()>,
}

#[derive(Serialize)]
struct ShowAddressesParams {
    account: u32,
    include_change_addresses: bool,
}

#[derive(Deserialize)]
struct NewAddressResponse {
    address: String,
}

impl Client {
    /// Creates a new wallet file (`wallet_create`).
    ///
    /// Returns the generated mnemonic when the daemon generated one.
    pub async fn create_wallet(
        &self,
        params: CreateWalletParams,
    ) -> Result<CreateWalletResult, Error> {
        self.call("wallet_create", &params).await
    }

    /// Recovers a wallet from a mnemonic (`wallet_recover`).
    pub async fn recover_wallet(&self, params: RecoverWalletParams) -> Result<(), Error> {
        self.call("wallet_recover", &params).await
    }

    /// Opens a wallet file; `password` should be `None` for unencrypted
    /// wallets (`wallet_open`).
    pub async fn open_wallet(&self, path: &str, password: Option<&str>) -> Result<(), Error> {
        self.call(
            "wallet_open",
            &OpenWalletParams {
                path,
                password,
                force_migrate_wallet_type: None,
                hardware_wallet: None,
            },
        )
        .await
    }

    /// Closes the open wallet (`wallet_close`).
    pub async fn close_wallet(&self) -> Result<(), Error> {
        self.call("wallet_close", &serde_json::json!({})).await
    }

    /// Returns general information about the open wallet (`wallet_info`).
    pub async fn wallet_info(&self) -> Result<WalletInfo, Error> {
        self.call("wallet_info", &serde_json::json!({})).await
    }

    /// Synchronizes the wallet with the chain up to the tip (`wallet_sync`).
    pub async fn sync_wallet(&self) -> Result<(), Error> {
        self.call("wallet_sync", &serde_json::json!({})).await
    }

    /// Performs a full rescan of the chain from genesis (`wallet_rescan`).
    pub async fn rescan_wallet(&self) -> Result<(), Error> {
        self.call("wallet_rescan", &serde_json::json!({})).await
    }

    /// Returns the best block known by the wallet (`wallet_best_block`).
    pub async fn best_block(&self) -> Result<BestBlock, Error> {
        self.call("wallet_best_block", &serde_json::json!({})).await
    }

    /// Creates a new account with a name (`account_create`).
    pub async fn create_account(&self, name: &str) -> Result<AccountInfo, Error> {
        self.call("account_create", &serde_json::json!({ "name": name })).await
    }

    /// Renames an account; `None` clears the name (`account_rename`).
    pub async fn rename_account(&self, account: u32, name: Option<&str>) -> Result<(), Error> {
        self.call(
            "account_rename",
            &RenameAccountParams {
                account,
                name: name.map(str::to_owned),
            },
        )
        .await
    }

    /// Returns the balance of an account, including only confirmed UTXOs
    /// (`account_balance`).
    pub async fn balance(&self, account: u32) -> Result<Balance, Error> {
        self.call(
            "account_balance",
            &BalanceParams {
                account,
                utxo_states: ["Confirmed"],
                with_locked: None,
            },
        )
        .await
    }

    /// Generates a new address for an account (`address_new`).
    pub async fn new_address(&self, account: u32) -> Result<String, Error> {
        let response: NewAddressResponse =
            self.call("address_new", &serde_json::json!({ "account": account })).await?;
        Ok(response.address)
    }

    /// Shows the receive addresses of an account with usage information
    /// (`address_show`).
    pub async fn show_receive_addresses(
        &self,
        account: u32,
    ) -> Result<Vec<AddressWithUsage>, Error> {
        self.call(
            "address_show",
            &ShowAddressesParams {
                account,
                include_change_addresses: false,
            },
        )
        .await
    }

    /// Reveals the public key behind an address
    /// (`address_reveal_public_key`).
    pub async fn reveal_public_key(
        &self,
        account: u32,
        address: &str,
    ) -> Result<RevealPublicKey, Error> {
        self.call(
            "address_reveal_public_key",
            &serde_json::json!({ "account": account, "address": address }),
        )
        .await
    }

    /// Encrypts the private keys of the wallet with a password
    /// (`wallet_encrypt_private_keys`).
    pub async fn encrypt_private_keys(&self, password: &str) -> Result<(), Error> {
        self.call(
            "wallet_encrypt_private_keys",
            &serde_json::json!({ "password": password }),
        )
        .await
    }

    /// Unlocks the encrypted private keys of the wallet
    /// (`wallet_unlock_private_keys`).
    pub async fn unlock_private_keys(&self, password: &str) -> Result<(), Error> {
        self.call(
            "wallet_unlock_private_keys",
            &serde_json::json!({ "password": password }),
        )
        .await
    }

    /// Locks the private keys of the wallet (`wallet_lock_private_keys`).
    pub async fn lock_private_keys(&self) -> Result<(), Error> {
        self.call("wallet_lock_private_keys", &serde_json::json!({})).await
    }
}
