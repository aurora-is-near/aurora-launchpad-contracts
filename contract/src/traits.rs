#![allow(dead_code)]
use aurora_launchpad_types::config::TokenId;
use defuse::core::crypto::PublicKey;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, PromiseOrValue, env, ext_contract};

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

/// Reads promise result `index`, bounded to the maximum length of a single NEP-141 `U128` amount,
/// and returns the parsed amount. Returns `None` when the promise failed, its result exceeded the
/// bound, or the payload did not parse as a `U128`, so callers supply their own default for a
/// missing amount.
#[must_use]
pub fn read_ft_result(index: u64) -> Option<u128> {
    env::promise_result_checked(index, MAX_FT_RESULT_LENGTH)
        .ok()
        .and_then(|bytes| near_sdk::serde_json::from_slice::<U128>(&bytes).ok())
        .map(|amount| amount.0)
}

/// Reads promise result `index`, bounded to the maximum length of a NEP-245 single-`token_id`
/// `Vec<U128>` result, and returns the first amount. Returns `None` when the promise failed, its
/// result exceeded the bound, did not parse, or the array was empty.
#[must_use]
pub fn read_mt_result(index: u64) -> Option<u128> {
    env::promise_result_checked(index, MAX_MT_RESULT_LENGTH)
        .ok()
        .and_then(|bytes| near_sdk::serde_json::from_slice::<Vec<U128>>(&bytes).ok())
        .and_then(|amounts| amounts.first().copied())
        .map(|amount| amount.0)
}
