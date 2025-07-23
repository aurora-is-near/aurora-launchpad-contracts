use aurora_launchpad_types::config::{
    DepositToken, DistributionProportions, LaunchpadConfig, LaunchpadStatus, Mechanics,
    VestingSchedule,
};
use aurora_launchpad_types::{IntentAccount, InvestmentAmount, WithdrawDirection};
use near_plugins::{AccessControlRole, AccessControllable, Pausable, Upgradable, access_control};
use near_sdk::borsh::BorshDeserialize;
use near_sdk::json_types::U128;
use near_sdk::store::{LazyOption, LookupMap};
use near_sdk::{AccountId, Gas, NearToken, PanicOnDefault, env, near};

use crate::mechanics::claim::{available_for_claim, user_allocation};
use crate::storage_key::StorageKey;

mod admin_withdraw;
mod claim;
mod deposit;
mod distribute;
mod lock;
mod mechanics;
mod storage_key;
#[cfg(test)]
mod tests;
mod traits;
mod utils;
mod withdraw;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas::from_tgas(35);
const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(3);
const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

#[derive(AccessControlRole, Clone, Copy)]
#[near(serializers = [json])]
enum Role {
    Admin,
    PauseManager,
    UnpauseManager,
}

#[derive(PanicOnDefault, Pausable, Upgradable)]
#[access_control(role_type(Role))]
#[upgradable(access_control_roles(
    code_stagers(Role::Admin),
    code_deployers(Role::Admin),
    duration_initializers(Role::Admin),
    duration_update_stagers(Role::Admin),
    duration_update_appliers(Role::Admin),
))]
#[pausable(
    pause_roles(Role::Admin, Role::PauseManager),
    unpause_roles(Role::Admin, Role::UnpauseManager)
)]
#[near(contract_state)]
pub struct AuroraLaunchpadContract {
    /// Launchpad configuration
    pub config: LaunchpadConfig,
    /// Number of unique participants in the launchpad
    pub participants_count: u64,
    /// The total number of deposit tokens received from the users.
    pub total_deposited: u128,
    /// The total number of sale tokens sold during the launchpad
    total_sold_tokens: u128,
    /// User investments in the launchpad
    pub investments: LookupMap<IntentAccount, InvestmentAmount>,
    /// Start timestamp of the vesting period, if applicable
    pub vesting_start_timestamp: LazyOption<u64>,
    /// Vesting users state with claimed amounts
    pub vestings: LookupMap<IntentAccount, u128>,
    pub individual_vesting_claimed: LookupMap<IntentAccount, u128>,
    /// Accounts relationship NEAR AccountId to IntentAccount
    pub accounts: LookupMap<AccountId, IntentAccount>,
    /// Flag indicating whether the sale token was transferred to the contract
    pub is_sale_token_set: bool,
    /// Flag indicating whether the assets distributed
    pub is_distributed: bool,
    /// Flag indicating whether the launchpad is locked or not.
    is_locked: bool,
}

#[near]
impl AuroraLaunchpadContract {
    /// Initializes the contract with the provided configuration and admin account.
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(config: LaunchpadConfig, admin: Option<AccountId>) -> Self {
        config
            .validate()
            .unwrap_or_else(|err| env::panic_str(&format!("Invalid config: {err}")));

        let mut contract = Self {
            config,
            participants_count: 0,
            total_deposited: 0,
            investments: LookupMap::new(StorageKey::Investments),
            vesting_start_timestamp: LazyOption::new(StorageKey::VestingStartTimestamp, None),
            vestings: LookupMap::new(StorageKey::Vestings),
            individual_vesting_claimed: LookupMap::new(StorageKey::IndividualVestingClaimed),
            accounts: LookupMap::new(StorageKey::Accounts),
            is_sale_token_set: false,
            is_distributed: false,
            total_sold_tokens: 0,
            is_locked: false,
        };

        let mut acl = contract.acl_get_or_init();

        if let Some(admin_account_id) = admin {
            acl.add_super_admin_unchecked(&admin_account_id);
            acl.grant_role_unchecked(Role::Admin, &admin_account_id);
        } else {
            acl.add_super_admin_unchecked(&env::signer_account_id());
            acl.grant_role_unchecked(Role::Admin, &env::current_account_id());
        }

        contract
    }

    pub fn is_not_initialized(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::NotInitialized)
    }

    pub fn is_not_started(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::NotStarted)
    }

    pub fn is_ongoing(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::Ongoing)
    }

    pub fn is_success(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::Success)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::Failed)
    }

    pub fn is_locked(&self) -> bool {
        matches!(self.get_status(), LaunchpadStatus::Locked)
    }

    /// Returns the current status of the launchpad.
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
            LaunchpadStatus::Ongoing
        } else if current_timestamp >= self.config.end_date
            && self.total_deposited >= self.config.soft_cap.0
        {
            LaunchpadStatus::Success
        } else {
            LaunchpadStatus::Failed
        }
    }

    /// Returns the launchpad configuration.
    pub fn get_config(&self) -> LaunchpadConfig {
        self.config.clone()
    }

    /// Returns the number of unique participants in the launchpad.
    pub const fn get_participants_count(&self) -> u64 {
        self.participants_count
    }

    /// Returns the total number of tokens deposited by all participants.
    pub fn get_total_deposited(&self) -> U128 {
        self.total_deposited.into()
    }

    /// Returns the total number of deposited tokens for a given account.
    pub fn get_investments(&self, account: &IntentAccount) -> Option<U128> {
        self.investments.get(account).map(|s| U128(s.amount))
    }

    /// Returns the total number of claimed tokens for a given account.
    pub fn get_claimed(&self, account: &IntentAccount) -> Option<U128> {
        self.investments.get(account).map(|s| U128(s.claimed))
    }

    /// Returns configuration of the distribution proportions.
    pub fn get_distribution_proportions(&self) -> DistributionProportions {
        self.config.distribution_proportions.clone()
    }

    /// Returns a timestamp of the start sale.
    pub const fn get_start_date(&self) -> u64 {
        self.config.start_date
    }

    /// Returns a timestamp of the end sale.
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

    /// Returns the total number of tokens sold during the launchpad.
    pub fn get_sold_amount(&self) -> U128 {
        self.total_sold_tokens.into()
    }

    /// Returns the sale token account ID.
    pub fn get_sale_token_account_id(&self) -> AccountId {
        self.config.sale_token_account_id.clone()
    }

    /// Returns the total number of tokens that should be sold during the launchpad.
    pub const fn get_total_sale_amount(&self) -> U128 {
        self.config.total_sale_amount
    }

    /// Returns the token allocation for the solver.
    pub const fn get_solver_allocation(&self) -> U128 {
        self.config.distribution_proportions.solver_allocation
    }

    /// Returns current mechanics of the launchpad.
    pub fn get_mechanics(&self) -> Mechanics {
        self.config.mechanics.clone()
    }

    /// Returns the vesting schedule, if any.
    pub fn get_vesting_schedule(&self) -> Option<VestingSchedule> {
        self.config.vesting_schedule.clone()
    }

    /// Returns the deposit token account ID.
    pub fn get_deposit_token_account_id(&self) -> DepositToken {
        self.config.deposit_token.clone()
    }

    /// Returns the number of tokens available for claim for the given intent account.
    pub fn get_available_for_claim(&self, account: &IntentAccount) -> U128 {
        let Some(investment) = self.investments.get(account) else {
            return U128(0);
        };

        available_for_claim(
            investment,
            self.total_sold_tokens,
            &self.config,
            env::block_timestamp(),
        )
        .unwrap_or_default()
        .saturating_sub(investment.claimed)
        .into()
    }

    /// Returns the allocation of tokens for a specific user account.
    pub fn get_user_allocation(&self, account: &IntentAccount) -> U128 {
        let Some(investment) = self.investments.get(account) else {
            return U128(0);
        };
        user_allocation(investment.weight, self.total_sold_tokens, &self.config)
            .unwrap_or_default()
            .into()
    }

    /// Calculates and returns the remaining vesting amount for a given account.
    pub fn get_remaining_vesting(&self, account: &IntentAccount) -> U128 {
        let Some(investment) = self.investments.get(account) else {
            return U128(0);
        };
        let available_for_claim = available_for_claim(
            investment,
            self.total_sold_tokens,
            &self.config,
            env::block_timestamp(),
        )
        .unwrap_or_default();
        let user_allocation =
            user_allocation(investment.weight, self.total_sold_tokens, &self.config)
                .unwrap_or_default();

        user_allocation.saturating_sub(available_for_claim).into()
    }

    /// Returns the version of the contract.
    #[must_use]
    pub const fn get_version() -> &'static str {
        VERSION
    }

    fn get_intents_account_id(
        &self,
        withdraw_direction: &WithdrawDirection,
        predecessor_account_id: &AccountId,
    ) -> IntentAccount {
        match withdraw_direction {
            WithdrawDirection::Intents(intent_account) => intent_account.clone(),
            WithdrawDirection::Near => self
                .accounts
                .get(predecessor_account_id)
                .cloned()
                .unwrap_or_else(|| {
                    env::panic_str("Intent account isn't found for the NEAR account id")
                }),
        }
    }
}
