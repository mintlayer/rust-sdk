// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Wallet token RPC methods: issuance, minting, freezing, authority.

use super::{
    ChangeAuthorityParams, Client, Error, FreezeParams, IssueNftParams, IssueTokenParams,
    IssueTokenResult, LockSupplyParams, MintParams, SendResult, UnfreezeParams, UnmintParams,
};

impl Client {
    /// Issues a new fungible token (`token_issue_new`).
    pub async fn issue_token(&self, params: IssueTokenParams) -> Result<IssueTokenResult, Error> {
        self.call("token_issue_new", &params).await
    }

    /// Issues a new NFT (`token_nft_issue_new`).
    pub async fn issue_nft(&self, params: IssueNftParams) -> Result<IssueTokenResult, Error> {
        self.call("token_nft_issue_new", &params).await
    }

    /// Mints tokens of an existing token (`token_mint`).
    pub async fn mint_tokens(&self, params: MintParams) -> Result<SendResult, Error> {
        self.call("token_mint", &params).await
    }

    /// Unmints tokens (`token_unmint`).
    pub async fn unmint_tokens(&self, params: UnmintParams) -> Result<SendResult, Error> {
        self.call("token_unmint", &params).await
    }

    /// Permanently locks the supply of a token (`token_lock_supply`).
    pub async fn lock_token_supply(&self, params: LockSupplyParams) -> Result<SendResult, Error> {
        self.call("token_lock_supply", &params).await
    }

    /// Freezes a token (`token_freeze`).
    pub async fn freeze_token(&self, params: FreezeParams) -> Result<SendResult, Error> {
        self.call("token_freeze", &params).await
    }

    /// Unfreezes a token (`token_unfreeze`).
    pub async fn unfreeze_token(&self, params: UnfreezeParams) -> Result<SendResult, Error> {
        self.call("token_unfreeze", &params).await
    }

    /// Transfers the token authority to a new address
    /// (`token_change_authority`).
    pub async fn change_token_authority(
        &self,
        params: ChangeAuthorityParams,
    ) -> Result<SendResult, Error> {
        self.call("token_change_authority", &params).await
    }
}
