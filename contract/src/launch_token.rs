use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::{AccountId, BorshStorageKey, NearToken, PanicOnDefault, env, log, near, require};

#[derive(BorshSerialize, BorshDeserialize, Debug, Eq, PartialEq, Ord, PartialOrd, Clone)]
#[borsh(crate = "near_sdk::borsh")]
pub struct LaunchToken {
    pub total_supply: U128,
    pub symbol: String,
}
