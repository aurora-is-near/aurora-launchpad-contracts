#![allow(dead_code)]
use aurora_launchpad_types::config::TokenId;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, PromiseOrValue, ext_contract};

#[ext_contract(ext_ft)]
trait FungibleToken {
    fn ft_transfer(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    ) -> PromiseOrValue<Vec<U128>>;
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    ) -> PromiseOrValue<Vec<U128>>;
    /// Returns the balance of a specific account.
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
}

#[ext_contract(ext_mt)]
trait MultiToken {
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
    );
    fn mt_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<(AccountId, u64)>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;
    /// Returns the balance of a specific token for a given account.
    fn mt_balance_of(&self, account_id: AccountId, token_id: TokenId) -> U128;
}
