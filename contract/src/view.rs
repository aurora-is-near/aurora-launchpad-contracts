use aurora_launchpad_types::IntentsAccount;
use aurora_launchpad_types::config::{
    DepositToken, DistributionProportions, LaunchpadConfig, LaunchpadStatus, Mechanics,
    VestingSchedule,
};
use near_sdk::json_types::U128;
use near_sdk::{AccountId, env, near};

use crate::{AuroraLaunchpadContract, AuroraLaunchpadContractExt, VERSION};

#[near]
impl AuroraLaunchpadContract {
    /// Return `true` if the contract is not initialized.
    pub fn is_not_initialized(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::NotInitialized)
    }

    /// Return `true` if the contract is not started yet.
    pub fn is_not_started(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::NotStarted)
    }

    /// Return `true` if the contract is ongoing.
    pub fn is_ongoing(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::Ongoing)
    }

    /// Return `true` if the contract is in the pre-TGE period.
    pub fn is_pre_tge_period(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::PreTGE)
    }

    /// Return `true` if the contract is successful.
    pub fn is_success(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::Success)
    }

    /// Return `true` if the contract is failed.
    pub fn is_failed(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::Failed)
    }

    /// Return `true` if the contract is locked.
    pub fn is_locked(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::Locked)
    }

    /// Return the current status of the launchpad.
    pub fn get_status(&self) -> LaunchpadStatus {
        if !self.is_sale_token_set {
            return LaunchpadStatus::NotInitialized;
        }

        if self.is_locked {
            return LaunchpadStatus::Locked;
        }

        let current_timestamp = env::block_timestamp();

        if current_timestamp < self.config.start_date {
            LaunchpadStatus::NotStarted
        } else if current_timestamp >= self.config.start_date
            && current_timestamp < self.config.end_date
        {
            if self.total_sold_tokens >= self.config.sale_amount.0
                && matches!(self.config.mechanics, Mechanics::FixedPrice { .. })
            {
                // TGE must always be greater than the config.end_date;
                // it means that we can skip the check that current_timestamp < tge.
                // So, check that TGE is present would be enough.
                if self.config.tge.is_some() {
                    LaunchpadStatus::PreTGE
                } else {
                    LaunchpadStatus::Success
                }
            } else {
                LaunchpadStatus::Ongoing
            }
        } else if current_timestamp >= self.config.end_date
            && self.total_deposited >= self.config.soft_cap.0
        {
            if self.config.tge.is_some_and(|tge| current_timestamp < tge) {
                LaunchpadStatus::PreTGE
            } else {
                LaunchpadStatus::Success
            }
        } else {
            LaunchpadStatus::Failed
        }
    }

    /// Return the launchpad configuration.
    pub fn get_config(&self) -> LaunchpadConfig {
        self.config.clone()
    }

    /// Return the number of unique participants in the launchpad.
    pub const fn get_participants_count(&self) -> u64 {
        self.participants_count
    }

    /// Return the total number of tokens deposited by all participants.
    pub fn get_total_deposited(&self) -> U128 {
        self.total_deposited.into()
    }

    /// Return the total number of deposited tokens for a given account.
    pub fn get_investments(&self, account: &IntentsAccount) -> Option<U128> {
        self.investments.get(account).map(|s| U128(s.amount))
    }

    /// Return configuration of the distribution proportions.
    pub fn get_distribution_proportions(&self) -> DistributionProportions {
        self.config.distribution_proportions.clone()
    }

    /// Return a timestamp of the start sale.
    pub const fn get_start_date(&self) -> u64 {
        self.config.start_date
    }

    /// Return a timestamp of the end sale.
    pub const fn get_end_date(&self) -> u64 {
        self.config.end_date
    }

    /// The threshold or minimum deposited tokens needed to conclude the sale successfully.
    pub const fn get_soft_cap(&self) -> U128 {
        self.config.soft_cap
    }

    /// Maximum (in the case of `FixedPrice`) and total (in the case of `PriceDiscovery`) number
    /// of sale tokens used for the sale.
    pub const fn get_sale_amount(&self) -> U128 {
        self.config.sale_amount
    }

    /// Return the total number of tokens sold during the launchpad.
    pub fn get_sold_amount(&self) -> U128 {
        self.total_sold_tokens.into()
    }

    /// Return the sale token account ID.
    pub fn get_sale_token_account_id(&self) -> AccountId {
        self.config.sale_token_account_id.clone()
    }

    /// Return the total number of tokens that should be sold during the launchpad.
    pub const fn get_total_sale_amount(&self) -> U128 {
        self.config.total_sale_amount
    }

    /// Return the token allocation for the solver.
    pub const fn get_solver_allocation(&self) -> U128 {
        self.config.distribution_proportions.solver_allocation
    }

    /// Return current mechanics of the launchpad.
    pub const fn get_mechanics(&self) -> Mechanics {
        self.config.mechanics
    }

    /// Return the vesting schedule, if any.
    pub const fn get_vesting_schedule(&self) -> Option<VestingSchedule> {
        self.config.vesting_schedule
    }

    /// Return the deposit token account ID.
    pub fn get_deposit_token_account_id(&self) -> DepositToken {
        self.config.deposit_token.clone()
    }

    /// Return the TGE.
    pub fn get_tge(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        let tge_nanoseconds = self.config.tge?;

        i64::try_from(tge_nanoseconds)
            .ok()
            .map(chrono::DateTime::from_timestamp_nanos)
    }

    /// Return the version of the contract.
    #[must_use]
    pub const fn get_version() -> &'static str {
        VERSION
    }
}
