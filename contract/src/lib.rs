use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{U64, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::store::{LazyOption, LookupMap};
use near_sdk::{AccountId, NearSchema, PanicOnDefault, PromiseOrValue, env, near, require};

#[derive(
    NearSchema,
    Debug,
    Eq,
    PartialEq,
    Clone,
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
)]
#[abi(borsh, json)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct LaunchpadConfig {
    pub deposit_token_account_id: AccountId,
    pub start_date: U64,
    pub end_date: U64,
    pub soft_cap: U128,
    pub mechanics: Mechanics,
    pub vesting_schedule: Option<VestingSchedule>,
    pub distribution_proportions: DistributionProportions,
}

#[derive(
    NearSchema,
    Debug,
    Eq,
    PartialEq,
    Clone,
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
)]
#[abi(borsh, json)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub enum Mechanics {
    FixedPrice,
}

#[derive(
    NearSchema,
    Debug,
    Eq,
    PartialEq,
    Clone,
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
)]
#[abi(borsh, json)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub enum DistributionProportions {
    FixedPrice,
}

#[derive(
    NearSchema,
    Debug,
    Eq,
    PartialEq,
    Clone,
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
)]
#[abi(borsh, json)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub enum VestingSchedule {
    Scheme1,
}

#[derive(
    NearSchema,
    Debug,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Clone,
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
)]
#[abi(borsh, json)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct IntentAccount(String);

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct AuroraLaunchpadContract {
    pub config: LaunchpadConfig,
    pub token_account_id: LazyOption<AccountId>,
    pub participants_count: u64,
    pub total_deposited: u128,
    pub investments: LookupMap<IntentAccount, u128>,
}

#[near]
impl AuroraLaunchpadContract {
    #[init]
    #[must_use]
    pub fn new(config: LaunchpadConfig) -> Self {
        Self {
            config,
            token_account_id: LazyOption::new(b"token_account_id".to_vec(), None),
            participants_count: 0,
            total_deposited: 0,
            investments: LookupMap::new(b"investments".to_vec()),
        }
    }

    pub fn is_launchpad_leave(&self) -> bool {
        env::block_timestamp() >= self.config.start_date.0
            && env::block_timestamp() < self.config.end_date.0
    }

    pub fn is_success(&self) -> bool {
        env::block_timestamp() >= self.config.end_date.0
            && self.total_deposited >= self.config.soft_cap.0
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn ft_on_transfer(
        &mut self,
        _sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        // Get Intent account from the message
        require!(!msg.is_empty(), "Invalid transfer token message format");
        let account = IntentAccount(msg.to_string());

        self.investments
            .entry(account)
            .and_modify(|x| *x += amount.0);
        self.total_deposited += amount.0;

        PromiseOrValue::Value(0.into())
    }

    pub fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<AccountId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let _ = (sender_id, previous_owner_ids, token_ids, amounts, msg);
        PromiseOrValue::Value(0.into())
    }
}
