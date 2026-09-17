// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wallet staking RPC methods: pools and delegations.

use serde::{Deserialize, Serialize};

use super::{
    Client, CreateDelegationParams, CreateDelegationResult, CreatePoolParams, DecommissionParams,
    DelegateParams, DelegationInfo, Error, OwnedPool, SendResult, StakingStatus, WithdrawParams,
};
use crate::wallet::types::Amount;

#[derive(Serialize)]
struct PoolBalanceParams {
    pool_id: String,
}

#[derive(Deserialize)]
struct PoolBalanceResponse {
    balance: Option<Amount>,
}

impl Client {
    /// Creates a new stake pool (`staking_create_pool`).
    pub async fn create_stake_pool(&self, params: CreatePoolParams) -> Result<SendResult, Error> {
        self.call("staking_create_pool", &params).await
    }

    /// Decommissions a stake pool (`staking_decommission_pool`).
    pub async fn decommission_stake_pool(
        &self,
        params: DecommissionParams,
    ) -> Result<SendResult, Error> {
        self.call("staking_decommission_pool", &params).await
    }

    /// Lists the stake pools owned by an account (`staking_list_pools`).
    pub async fn list_owned_pools(&self, account: u32) -> Result<Vec<OwnedPool>, Error> {
        self.call(
            "staking_list_pools",
            &serde_json::json!({ "account": account }),
        )
        .await
    }

    /// Returns the balance of a stake pool (`staking_pool_balance`).
    pub async fn pool_balance(&self, pool_id: &str) -> Result<Option<Amount>, Error> {
        let response: PoolBalanceResponse = self
            .call(
                "staking_pool_balance",
                &PoolBalanceParams {
                    pool_id: pool_id.to_owned(),
                },
            )
            .await?;
        Ok(response.balance)
    }

    /// Starts staking with an account (`staking_start`).
    pub async fn start_staking(&self, account: u32) -> Result<(), Error> {
        self.call("staking_start", &serde_json::json!({ "account": account })).await
    }

    /// Stops staking for an account (`staking_stop`).
    pub async fn stop_staking(&self, account: u32) -> Result<(), Error> {
        self.call("staking_stop", &serde_json::json!({ "account": account })).await
    }

    /// Returns whether an account is staking (`staking_status`).
    pub async fn staking_status(&self, account: u32) -> Result<StakingStatus, Error> {
        self.call("staking_status", &serde_json::json!({ "account": account })).await
    }

    /// Creates a delegation to a pool (`delegation_create`).
    pub async fn create_delegation(
        &self,
        params: CreateDelegationParams,
    ) -> Result<CreateDelegationResult, Error> {
        self.call("delegation_create", &params).await
    }

    /// Delegates an amount to an existing delegation
    /// (`delegation_stake`).
    pub async fn delegate_staking(&self, params: DelegateParams) -> Result<SendResult, Error> {
        self.call("delegation_stake", &params).await
    }

    /// Withdraws from a delegation (`delegation_withdraw`).
    pub async fn withdraw_from_delegation(
        &self,
        params: WithdrawParams,
    ) -> Result<SendResult, Error> {
        self.call("delegation_withdraw", &params).await
    }

    /// Lists the delegations owned by an account
    /// (`delegation_list_ids`).
    pub async fn list_delegations(&self, account: u32) -> Result<Vec<DelegationInfo>, Error> {
        self.call(
            "delegation_list_ids",
            &serde_json::json!({ "account": account }),
        )
        .await
    }
}
