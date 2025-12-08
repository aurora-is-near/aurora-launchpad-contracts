use aurora_launchpad_types::admin_withdraw::WithdrawnUnsoldTokens;
use aurora_launchpad_types::config::{DistributionAccount, LaunchpadConfig};
use aurora_launchpad_types::distribution::DepositsDistribution;
use aurora_launchpad_types::{IntentsAccount, InvestmentAmount};
use near_plugins::{AccessControlRole, AccessControllable, Pausable, Upgradable, access_control};
use near_sdk::borsh::BorshDeserialize;
use near_sdk::store::{LookupMap, LookupSet};
use near_sdk::{AccountId, Gas, NearToken, PanicOnDefault, env, near};

use crate::discount::DiscountState;
use crate::storage_key::StorageKey;

mod admin;
mod claim;
mod deposit;
mod discount;
mod distribute;
mod mechanics;
mod storage_key;
#[cfg(test)]
mod tests;
mod traits;
mod view;
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
