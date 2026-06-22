#![allow(dead_code)]
use aurora_launchpad_types::config::TokenId;
use defuse::core::crypto::PublicKey;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, PromiseOrValue, ext_contract};

pub const MAX_FT_RESULT_LENGTH: usize = r#""+340282366920938463463374607431768211455""#.len(); // u128::MAX
// In the case of NEP-245, we operate with a single token_id, so the vector could ultimately contain a single value.
pub const MAX_MT_RESULT_LENGTH: usize = r#"["+340282366920938463463374607431768211455"]"#.len(); // vec![u128::MAX]

#[ext_contract(ext_ft)]
trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    ) -> PromiseOrValue<U128>;
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

#[ext_contract(ext_defuse)]
trait Defuse {
    fn has_public_key(&mut self, account_id: AccountId, public_key: &PublicKey) -> bool;
}
