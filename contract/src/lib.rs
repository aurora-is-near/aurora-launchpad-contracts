use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{U64, U128};
use near_sdk::store::LookupMap;
use near_sdk::{AccountId, PanicOnDefault, PromiseOrValue, env, near, require};

mod launch_token;

#[derive(Debug, Eq, PartialEq, Clone, BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]

pub enum Mechanics {
    FixedPrice,
}

#[derive(Debug, Eq, PartialEq, Clone, BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub enum DistributionProportions {
    FixedPrice,
}

#[derive(Debug, Eq, PartialEq, Clone, BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub enum VestingSchedule {
    Scheme1,
}

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct AuroraLaunchpadContract {
    pub token_account_id: AccountId,
    pub deposit_token_account_id: AccountId,
    pub start_date: u64,
    pub end_date: u64,
    pub soft_cap: u128,
    pub mechanics: Mechanics,
    pub vesting_schedule: Option<VestingSchedule>,
    pub distribution_proportions: DistributionProportions,
    pub participants_count: u64,
    pub total_deposited: u128,
    pub investments: LookupMap<AccountId, u128>,
}

#[near]
impl AuroraLaunchpadContract {
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(
        token_account_id: AccountId,
        deposit_token_account_id: AccountId,
        start_date: U64,
        end_date: U64,
        soft_cap: U128,
    ) -> Self {
        Self {
            token_account_id,
            deposit_token_account_id,
            start_date: start_date.0,
            end_date: end_date.0,
            soft_cap: soft_cap.0,
            // TODO: fix after launchpad mechanics are implemented
            mechanics: Mechanics::FixedPrice,
            // TODO: fix after launchpad vesting schedule is implemented
            vesting_schedule: None,
            // TODO: fix after launchpad distribution proportions are implemented
            distribution_proportions: DistributionProportions::FixedPrice,
            participants_count: 0,
            total_deposited: 0,
            investments: LookupMap::new(b"investments".to_vec()),
        }
    }

    pub fn is_launchpad_leave(&self) -> bool {
        env::block_timestamp() >= self.start_date && env::block_timestamp() < self.end_date
    }

    pub fn is_success(&self) -> bool {
        env::block_timestamp() >= self.end_date && self.total_deposited >= self.soft_cap
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        require!(
            env::predecessor_account_id() == self.token_account_id,
            "Incorrect token account id"
        );

        let _ = msg;

        self.investments
            .entry(sender_id)
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
