use near_sdk::json_types::{U64, U128};
use near_sdk::store::{LazyOption, LookupMap};
use near_sdk::{AccountId, PanicOnDefault, PromiseOrValue, env, near, require};

use crate::config::{
    DistributionProportions, IntentAccount, LaunchpadConfig, LaunchpadStatus, LaunchpadToken,
    Mechanics, VestingSchedule,
};

mod config;

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct AuroraLaunchpadContract {
    pub config: LaunchpadConfig,
    pub token_account_id: LazyOption<AccountId>,
    pub participants_count: u64,
    pub total_deposited: u128,
    pub investments: LookupMap<IntentAccount, u128>,
    pub is_paused: bool,
}

#[near]
impl AuroraLaunchpadContract {
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(config: LaunchpadConfig) -> Self {
        Self {
            config,
            token_account_id: LazyOption::new(b"token_account_id".to_vec(), None),
            participants_count: 0,
            total_deposited: 0,
            investments: LookupMap::new(b"investments".to_vec()),
            is_paused: false,
        }
    }

    pub fn is_not_started(&self) -> bool {
        env::block_timestamp() < self.config.start_date
    }

    pub fn is_ongoing(&self) -> bool {
        !self.is_paused
            || env::block_timestamp() >= self.config.start_date
                && env::block_timestamp() < self.config.end_date
    }

    pub fn is_success(&self) -> bool {
        !self.is_paused
            || env::block_timestamp() >= self.config.end_date
                && self.total_deposited >= self.config.soft_cap.0
    }

    pub fn is_failed(&self) -> bool {
        self.is_paused
            || env::block_timestamp() >= self.config.end_date
                && self.total_deposited < self.config.soft_cap.0
    }

    pub fn get_status(&self) -> LaunchpadStatus {
        if self.is_not_started() {
            LaunchpadStatus::NotStarted
        } else if self.is_ongoing() {
            LaunchpadStatus::Ongoing
        } else if self.is_success() {
            LaunchpadStatus::Success
        } else {
            LaunchpadStatus::Failed
        }
    }

    pub fn get_config(&self) -> LaunchpadConfig {
        self.config.clone()
    }

    pub fn get_token_account_id(&self) -> Option<AccountId> {
        self.token_account_id.get().clone()
    }

    pub fn get_participants_count(&self) -> U64 {
        self.participants_count.into()
    }

    pub fn get_total_deposited(&self) -> U128 {
        self.total_deposited.into()
    }

    pub fn get_investments(&self, account: &IntentAccount) -> Option<U128> {
        self.investments.get(account).map(|s| U128(*s))
    }

    pub fn get_token(&self) -> LaunchpadToken {
        self.config.token.clone()
    }

    pub const fn get_start_date(&self) -> u64 {
        self.config.start_date
    }

    pub const fn get_end_date(&self) -> u64 {
        self.config.end_date
    }

    pub const fn get_soft_cap(&self) -> U128 {
        self.config.soft_cap
    }

    pub const fn get_sale_amount(&self) -> Option<U128> {
        self.config.sale_amount
    }

    pub const fn get_solver_allocation(&self) -> U128 {
        self.config.solver_allocation
    }

    pub fn get_mechanics(&self) -> Mechanics {
        self.config.mechanics.clone()
    }

    pub fn get_vesting_schedule(&self) -> Option<VestingSchedule> {
        self.config.vesting_schedule.clone()
    }

    pub fn get_distribution_proportions(&self) -> DistributionProportions {
        self.config.distribution_proportions.clone()
    }

    pub fn get_deposit_token_account_id(&self) -> AccountId {
        self.config.deposit_token_account_id.clone()
    }

    pub const fn set_paused(&mut self, paused: bool) {
        // Check permission to pause/unpause
        // require!(env.predecessor_account_id() == ?, "Permission denied");
        self.is_paused = paused;
    }

    pub fn claim(
        &mut self,
        #[allow(clippy::used_underscore_binding)] _account: IntentAccount,
    ) -> PromiseOrValue<U128> {
        // Withdraw only if Status is `Success`
        // Check permission to withdraw
        // require!( WE_SHOULD_DECIDE_HOW_TO_WITHDRAW, "Permission denied" );
        // - transfer amount:
        //   - according rules of vesting schedule (if any) to the user Intent account
        //   - according deposit weight related to specified Mechanics
        //   - Launchpad assets to the user Intent account
        todo!()
    }

    pub fn withdraw(&mut self, account: &IntentAccount) -> PromiseOrValue<U128> {
        let _ = account;
        // Withdraw only if Status is `Fail`
        // Check permission to withdraw
        // require!( WE_SHOULD_DECIDE_HOW_TO_WITHDRAW, "Permission denied" );
        // - transfer all user deposited assets to the user Intent account
        todo!()
    }

    pub fn distribute_tokens(&mut self) {
        // Check permission to distribute tokens
        // require!(env.predeecessor_account_id() == ?, "Permission denied");
        // - Method should be called only when status is success
        // - Method called only once
        // - All assets should be transferred to the Pool account
        todo!()
    }

    pub fn ft_on_transfer(
        &mut self,
        #[allow(clippy::used_underscore_binding)] _sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        // Get Intent account from the message
        require!(!msg.is_empty(), "Invalid transfer token message format");
        let account = IntentAccount(msg);

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
