// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wallet transaction RPC methods: sending, composing, signing, history.

use serde::Serialize;

use super::{
    Client, ComposeParams, ComposedTx, Error, SendParams, SendResult, SignedTx, SubmitResult,
    SweepParams, TokenSendParams, TxInspection, TxOptions, UtxoSpendParams, WalletTx,
};

#[derive(Serialize)]
struct SignRawTransactionParams {
    account: u32,
    raw_tx: String,
    options: TxOptions,
}

#[derive(Serialize)]
struct SubmitTransactionParams {
    tx: String,
    do_not_store: bool,
    options: SubmitOptions,
}

#[derive(Serialize)]
struct SubmitOptions {
    trust_policy: &'static str,
}

#[derive(Serialize)]
struct ListByAddressParams {
    account: u32,
    address: Option<String>,
    limit: u32,
}

#[derive(Serialize)]
struct GetTransactionParams {
    account: u32,
    transaction_id: String,
}

#[derive(Serialize)]
struct DepositDataParams {
    account: u32,
    data: String,
    options: TxOptions,
}

impl Client {
    /// Sends coins to an address (`address_send`).
    pub async fn send(&self, params: SendParams) -> Result<SendResult, Error> {
        self.call("address_send", &params).await
    }

    /// Sends tokens to an address (`token_send`).
    pub async fn send_token(&self, params: TokenSendParams) -> Result<SendResult, Error> {
        self.call("token_send", &params).await
    }

    /// Sweeps all spendable funds from the given addresses
    /// (`address_sweep_spendable`).
    pub async fn sweep_spendable(&self, params: SweepParams) -> Result<SendResult, Error> {
        self.call("address_sweep_spendable", &params).await
    }

    /// Spends a single UTXO (`utxo_spend`).
    pub async fn spend_utxo(&self, params: UtxoSpendParams) -> Result<SendResult, Error> {
        self.call("utxo_spend", &params).await
    }

    /// Composes a transaction from explicit inputs and outputs without
    /// signing it (`transaction_compose`). The returned hex encodes a
    /// partially signed transaction.
    pub async fn compose_transaction(&self, params: ComposeParams) -> Result<ComposedTx, Error> {
        self.call("transaction_compose", &params).await
    }

    /// Signs a raw partially signed transaction with the keys of an account
    /// (`account_sign_raw_transaction`).
    pub async fn sign_raw_transaction(
        &self,
        account: u32,
        raw_tx: &str,
    ) -> Result<SignedTx, Error> {
        self.call(
            "account_sign_raw_transaction",
            &SignRawTransactionParams {
                account,
                raw_tx: raw_tx.to_owned(),
                options: TxOptions::default(),
            },
        )
        .await
    }

    /// Inspects a hex-encoded transaction (`transaction_inspect`).
    pub async fn inspect_transaction(&self, tx_hex: &str) -> Result<TxInspection, Error> {
        self.call(
            "transaction_inspect",
            &serde_json::json!({ "transaction": tx_hex }),
        )
        .await
    }

    /// Submits a hex-encoded signed transaction to the node
    /// (`node_submit_transaction`, trusted policy).
    pub async fn submit_transaction(
        &self,
        tx_hex: &str,
        do_not_store: bool,
    ) -> Result<SubmitResult, Error> {
        self.call(
            "node_submit_transaction",
            &SubmitTransactionParams {
                tx: tx_hex.to_owned(),
                do_not_store,
                options: SubmitOptions {
                    trust_policy: "Trusted",
                },
            },
        )
        .await
    }

    /// Lists the transactions of an address in an account
    /// (`transaction_list_by_address`).
    pub async fn list_transactions_by_address(
        &self,
        account: u32,
        address: Option<&str>,
        limit: u32,
    ) -> Result<Vec<WalletTx>, Error> {
        self.call(
            "transaction_list_by_address",
            &ListByAddressParams {
                account,
                address: address.map(str::to_owned),
                limit,
            },
        )
        .await
    }

    /// Lists the pending transactions of an account
    /// (`transaction_list_pending`).
    pub async fn list_pending_transactions(&self, account: u32) -> Result<Vec<String>, Error> {
        self.call(
            "transaction_list_pending",
            &serde_json::json!({ "account": account }),
        )
        .await
    }

    /// Returns a transaction tracked by the wallet
    /// (`transaction_get`).
    pub async fn transaction(&self, account: u32, tx_id: &str) -> Result<serde_json::Value, Error> {
        self.call(
            "transaction_get",
            &GetTransactionParams {
                account,
                transaction_id: tx_id.to_owned(),
            },
        )
        .await
    }

    /// Abandons an unconfirmed transaction (`transaction_abandon`).
    pub async fn abandon_transaction(&self, account: u32, tx_id: &str) -> Result<(), Error> {
        self.call(
            "transaction_abandon",
            &GetTransactionParams {
                account,
                transaction_id: tx_id.to_owned(),
            },
        )
        .await
    }

    /// Deposits arbitrary on-chain data from an account
    /// (`address_deposit_data`).
    pub async fn deposit_data(&self, account: u32, data_hex: &str) -> Result<SendResult, Error> {
        self.call(
            "address_deposit_data",
            &DepositDataParams {
                account,
                data: data_hex.to_owned(),
                options: TxOptions::default(),
            },
        )
        .await
    }
}
