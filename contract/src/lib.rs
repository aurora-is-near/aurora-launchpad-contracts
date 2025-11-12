use aurora_launchpad_types::admin_withdraw::WithdrawnUnsoldTokens;
use aurora_launchpad_types::config::{
    DepositToken, DistributionAccount, DistributionProportions, LaunchpadConfig, LaunchpadStatus,
    Mechanics, VestingSchedule,
};
use aurora_launchpad_types::distribution::DepositsDistribution;
use aurora_launchpad_types::{IntentsAccount, InvestmentAmount};
use near_plugins::{
    AccessControlRole, AccessControllable, Pausable, Upgradable, access_control, access_control_any,
};
use near_sdk::borsh::BorshDeserialize;
use near_sdk::json_types::U128;
use near_sdk::store::{LookupMap, LookupSet};
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, Promise, PublicKey, assert_one_yocto, env, near,
};

use crate::discount::DiscountState;
use crate::storage_key::StorageKey;

mod admin_withdraw;
mod claim;
mod deposit;
mod discount;
mod distribute;
mod lock;
mod mechanics;
mod storage_key;
#[cfg(test)]
mod tests;
mod traits;
mod withdraw;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas::from_tgas(35);
const GAS_FOR_MT_TRANSFER_CALL: Gas = Gas::from_tgas(40);
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
    pub investments: LookupMap<IntentsAccount, InvestmentAmount>,
    /// Vesting users state with claimed amounts
    pub vestings: LookupMap<IntentsAccount, u128>,
    /// Individual vesting claimed amounts for each stakeholder
    pub individual_vesting_claimed: LookupMap<DistributionAccount, u128>,
    /// Flag indicating whether the sale token was transferred to the contract
    pub is_sale_token_set: bool,
    /// Flag indicating whether the launchpad is locked or not.
    is_locked: bool,
    /// Already distributed accounts and their fully or partly distributed amounts
    /// and statuses to prevent double distributions.
    pub distributed_accounts: LookupMap<DistributionAccount, (u128, bool)>,
    /// Set of accounts that have withdrawal in progress in the locked state.
    pub locked_withdraw: LookupSet<IntentsAccount>,
    /// Deposits distribution to solver and fee accounts, if any.
    pub deposits_distribution: DepositsDistribution,
    /// The number of unsold tokens withdrawn by the admin and status of withdrawal.
    withdrawn_unsold_tokens: WithdrawnUnsoldTokens,
    /// The discounts state includes state for every discount phase.
    discount_state: Option<DiscountState>,
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

        let discount_state = config.discounts.as_ref().map(DiscountState::init);
        let mut contract = Self {
            config,
            participants_count: 0,
            total_deposited: 0,
            investments: LookupMap::new(StorageKey::Investments),
            vestings: LookupMap::new(StorageKey::Vestings),
            individual_vesting_claimed: LookupMap::new(StorageKey::IndividualVestingClaimed),
            is_sale_token_set: false,
            total_sold_tokens: 0,
            is_locked: false,
            distributed_accounts: LookupMap::new(StorageKey::DistributedAccounts),
            locked_withdraw: LookupSet::new(StorageKey::LockedWithdraw),
            deposits_distribution: DepositsDistribution::default(),
            withdrawn_unsold_tokens: WithdrawnUnsoldTokens::default(),
            discount_state,
        };

        let admin_account_id = admin.unwrap_or_else(env::signer_account_id);
        contract.grant_roles(&admin_account_id);

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
            if self.total_deposited >= self.config.soft_cap.0
                && matches!(self.config.mechanics, Mechanics::FixedPrice { .. })
            {
                LaunchpadStatus::Success
            } else {
                LaunchpadStatus::Ongoing
            }
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
    pub fn get_investments(&self, account: &IntentsAccount) -> Option<U128> {
        self.investments.get(account).map(|s| U128(s.amount))
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
    pub const fn get_mechanics(&self) -> Mechanics {
        self.config.mechanics
    }

    /// Returns the vesting schedule, if any.
    pub const fn get_vesting_schedule(&self) -> Option<VestingSchedule> {
        self.config.vesting_schedule
    }

    /// Returns the deposit token account ID.
    pub fn get_deposit_token_account_id(&self) -> DepositToken {
        self.config.deposit_token.clone()
    }

    /// Returns the version of the contract.
    #[must_use]
    pub const fn get_version() -> &'static str {
        VERSION
    }

    #[payable]
    #[access_control_any(roles(Role::Admin))]
    pub fn add_full_access_key(&mut self, public_key: PublicKey) -> Promise {
        assert_one_yocto();
        Promise::new(env::current_account_id()).add_full_access_key(public_key)
    }

    fn grant_roles(&mut self, admin_account_id: &AccountId) {
        let mut acl = self.acl_get_or_init();
        acl.add_super_admin_unchecked(admin_account_id);

        acl.add_admin_unchecked(Role::Admin, admin_account_id);
        acl.add_admin_unchecked(Role::PauseManager, admin_account_id);
        acl.add_admin_unchecked(Role::UnpauseManager, admin_account_id);

        acl.grant_role_unchecked(Role::Admin, admin_account_id);
        acl.grant_role_unchecked(Role::PauseManager, admin_account_id);
        acl.grant_role_unchecked(Role::UnpauseManager, admin_account_id);
    }
}
