#![allow(dead_code)]
use aurora_launchpad_types::config::TokenId;
use defuse::core::crypto::PublicKey;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, PromiseError, PromiseOrValue, env, ext_contract};

/// Maximum byte length of a NEP-141 promise result accepted by [`read_ft_result`].
///
/// The result is a JSON-quoted `U128`, e.g. `"123"`. The longest possible value is
/// `u128::MAX` = `340282366920938463463374607431768211455` (39 digits), so the longest
/// canonical encoding is `"` + 39 digits + `"` = 41 bytes. The bounded read rejects
/// longer payloads (oversized or non-canonical, e.g., sign-prefixed or zero-padded).
pub const MAX_FT_RESULT_LENGTH: usize = 41;

/// Maximum byte length of a NEP-245 promise result accepted by [`read_mt_result`].
///
/// We always transfer a single `token_id`, so the result is a one-element `Vec<U128>`,
/// e.g. `["123"]`. With `u128::MAX` (39 digits) the longest canonical encoding is
/// `[` + `"` + 39 digits + `"` + `]` = 43 bytes. Longer payloads are rejected by the bounded read.
pub const MAX_MT_RESULT_LENGTH: usize = 43;

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
/// and returns the consumed amount. Returns `0` when the promise failed (nothing was delivered, so
/// callers roll back or refund in full). Returns the original `amount` (fail closed, treated as
/// fully consumed) when the result exceeded the bound or the payload did not parse as a `U128`,
/// since the transfer may have been delivered and such results are considered malicious.
#[must_use]
pub fn read_ft_result(index: u64, amount: u128) -> u128 {
    match env::promise_result_checked(index, MAX_FT_RESULT_LENGTH) {
        Ok(bytes) => {
            near_sdk::serde_json::from_slice::<U128>(&bytes).map_or(amount, |amount| amount.0)
        } // No refund if bytes are present but not a valid U128.
        Err(PromiseError::TooLong(len)) => {
            near_sdk::log!(
                "Promise result at index {index} exceeded max length of {MAX_FT_RESULT_LENGTH} bytes: {len} bytes"
            );
            amount // No refund if bytes are present but too long.
        }
        Err(_) => {
            near_sdk::log!("Promise result at index {index} failed");
            0
        }
    }
}

/// Reads promise result `index`, bounded to the maximum length of a NEP-245 single-`token_id`
/// `Vec<U128>` result, and returns the single consumed amount. Returns `0` when the promise failed
/// (nothing was delivered, so callers roll back or refund in full). Returns the original `amount`
/// (fail closed, treated as fully consumed) when the result exceeded the bound, did not parse, or
/// the array did not contain exactly one element, since the transfer may have been delivered and
/// such results are considered malicious.
#[must_use]
pub fn read_mt_result(index: u64, amount: u128) -> u128 {
    match env::promise_result_checked(index, MAX_MT_RESULT_LENGTH) {
        Ok(bytes) => near_sdk::serde_json::from_slice::<Vec<U128>>(&bytes)
            .ok()
            .and_then(|amounts| {
                if amounts.len() == 1 {
                    amounts.first().copied()
                } else {
                    None
                }
            })
            .map_or(amount, |amount| amount.0), // No refund if bytes are present but not a valid Vec<U128>.
        Err(PromiseError::TooLong(len)) => {
            near_sdk::log!(
                "Promise result at index {index} exceeded max length of {MAX_MT_RESULT_LENGTH} bytes: {len} bytes"
            );
            amount // No refund if bytes are present but too long.
        }
        Err(_) => {
            near_sdk::log!("Promise result at index {index} failed");
            0
        }
    }
}
