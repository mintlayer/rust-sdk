// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wallet DEX order RPC methods.

use super::{
    ActiveOrder, Client, ConcludeOrderParams, CreateOrderParams, Error, FillOrderParams,
    FreezeOrderParams, ListOrdersParams, OrderCreated, OwnOrder, SendResult,
};

impl Client {
    /// Creates a new DEX order (`order_create`).
    pub async fn create_order(&self, params: CreateOrderParams) -> Result<OrderCreated, Error> {
        self.call("order_create", &params).await
    }

    /// Concludes an order (`order_conclude`).
    pub async fn conclude_order(&self, params: ConcludeOrderParams) -> Result<SendResult, Error> {
        self.call("order_conclude", &params).await
    }

    /// Fills an existing order (`order_fill`).
    pub async fn fill_order(&self, params: FillOrderParams) -> Result<SendResult, Error> {
        self.call("order_fill", &params).await
    }

    /// Freezes an order (`order_freeze`).
    pub async fn freeze_order(&self, params: FreezeOrderParams) -> Result<SendResult, Error> {
        self.call("order_freeze", &params).await
    }

    /// Lists the orders owned by an account (`order_list_own`).
    pub async fn list_own_orders(&self, account: u32) -> Result<Vec<OwnOrder>, Error> {
        self.call("order_list_own", &serde_json::json!({ "account": account })).await
    }

    /// Lists all active orders, optionally filtered by currency
    /// (`order_list_all_active`).
    pub async fn list_all_active_orders(
        &self,
        params: ListOrdersParams,
    ) -> Result<Vec<ActiveOrder>, Error> {
        self.call("order_list_all_active", &params).await
    }
}
