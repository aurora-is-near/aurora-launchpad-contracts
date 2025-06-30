#![allow(clippy::doc_lazy_continuation)]

use aurora_launchpad_types::config::{
    DepositToken, DistributionProportions, LaunchpadConfig, LaunchpadStatus, Mechanics, TokenId,
    VestingSchedule,
};
use aurora_launchpad_types::{IntentAccount, InvestmentAmount, WithdrawalAccount};
use near_plugins::{AccessControlRole, AccessControllable, Pausable, access_control, pause};
use near_sdk::borsh::BorshDeserialize;
use near_sdk::json_types::U128;
use near_sdk::store::{LazyOption, LookupMap};
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseOrValue, PromiseResult, env,
    ext_contract, near, require,
};

use crate::mechanics::claim::available_for_claim;
use crate::utils::parse_accounts;

mod mechanics;
mod utils;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas::from_tgas(70);
const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(5);
const GAS_FOR_FINISH_CLAIM: Gas = Gas::from_tgas(2);
const GAS_FOR_FINISH_DISTRIBUTION: Gas = Gas::from_tgas(2);

const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

// For some reason, the clippy lints are not working properly in that macro
#[allow(dead_code)]
#[ext_contract(ext_ft)]
trait FungibleToken {
    fn ft_transfer(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    ) -> PromiseOrValue<Vec<U128>>;
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    ) -> PromiseOrValue<Vec<U128>>;
}

#[ext_contract(ext_mt)]
pub trait MultiToken {
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
}

#[derive(AccessControlRole, Clone, Copy)]
#[near(serializers = [json])]
enum Role {
    PauseManager,
    UnpauseManager,
}

#[derive(PanicOnDefault, Pausable)]
#[access_control(role_type(Role))]
#[pausable(pause_roles(Role::PauseManager), unpause_roles(Role::UnpauseManager))]
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
    /// Accounts relationship NEAR AccountId to IntentAccount
    pub accounts: LookupMap<AccountId, IntentAccount>,
    /// Flag indicating whether the sale token was transferred to the contract
    pub is_sale_token_set: bool,
    /// Flag indicating whether the assets distributed
    pub is_distributed: bool,
}

#[near]
impl AuroraLaunchpadContract {
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(config: LaunchpadConfig) -> Self {
        config
            .validate()
            .unwrap_or_else(|err| env::panic_str(&format!("Invalid config: {err}")));

        Self {
            config,
            participants_count: 0,
            total_deposited: 0,
            investments: LookupMap::new(b"investments".to_vec()),
            vesting_start_timestamp: LazyOption::new(b"vesting_start_timestamp".to_vec(), None),
            vestings: LookupMap::new(b"vestings".to_vec()),
            accounts: LookupMap::new(b"accounts".to_vec()),
            is_sale_token_set: false,
            is_distributed: false,
            total_sold_tokens: 0,
        }
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

    fn is_paused(&self) -> bool {
        self.pa_is_paused("__PAUSED__".to_string())
    }

    /// Returns the current status of the launchpad.
    pub fn get_status(&self) -> LaunchpadStatus {
        if !self.is_sale_token_set {
            return LaunchpadStatus::NotStarted;
        }
        if self.is_paused() {
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

    /// Returns configuration of the distribution proportions.
    pub fn get_distribution_proportions(&self) -> DistributionProportions {
        self.config.distribution_proportions.clone()
    }

    /// Start timestamp of the sale.
    pub const fn get_start_date(&self) -> u64 {
        self.config.start_date
    }

    /// End timestamp of the sale.
    pub const fn get_end_date(&self) -> u64 {
        self.config.end_date
    }

    /// The threshold or minimum deposited tokens needed to conclude the sale successfully.
    pub const fn get_soft_cap(&self) -> U128 {
        self.config.soft_cap
    }

    /// Maximum (in case of `FixedPrice`) and total (in case of `PriceDiscovery`) number of sale
    /// tokens used for the sale.
    pub const fn get_sale_amount(&self) -> U128 {
        self.config.sale_amount
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
        // available_for_claim - claimed
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

    /// Returns the version of the contract.
    #[must_use]
    pub const fn get_version() -> &'static str {
        VERSION
    }

    pub fn claim(&mut self, withdrawal_account: &WithdrawalAccount) -> Promise {
        require!(
            self.is_success(),
            "Claim can be called only if the launchpad finishes with success status"
        );

        let intent_account_id = match withdrawal_account {
            WithdrawalAccount::Intents(account) => account,
            WithdrawalAccount::Near(account_id) => self
                .accounts
                .get(account_id)
                .unwrap_or_else(|| env::panic_str("No intent account found")),
        };

        let Some(investment) = self.investments.get_mut(intent_account_id) else {
            env::panic_str("No deposits found for the intent account");
        };
        let assets_amount = match available_for_claim(
            investment,
            self.total_sold_tokens,
            &self.config,
            env::block_timestamp(),
        ) {
            Ok(amount) => amount,
            Err(err) => env::panic_str(&format!("Claim failed: {err}")),
        };

        match withdrawal_account {
            WithdrawalAccount::Intents(intent_account_id) => {
                ext_ft::ext(self.config.sale_token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                    .ft_transfer_call(
                        self.config.intents_account_id.clone(),
                        assets_amount.into(),
                        intent_account_id.as_ref().to_string(),
                        None,
                    )
            }
            WithdrawalAccount::Near(account_id) => {
                ext_ft::ext(self.config.sale_token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_FT_TRANSFER)
                    .ft_transfer(account_id.clone(), assets_amount.into(), None)
            }
        }
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_FINISH_CLAIM)
                .finish_claim(intent_account_id, assets_amount),
        )
    }

    #[private]
    pub fn finish_claim(&mut self, intent_account_id: &IntentAccount, assets_amount: u128) {
        // Check if the intent account exists
        require!(
            env::promise_results_count() == 1,
            "Expected one promise result"
        );

        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                let Some(investment) = self.investments.get_mut(intent_account_id) else {
                    env::panic_str("No deposits found for the intent account");
                };
                // Increase claimed assets
                investment.claimed = investment.claimed.saturating_add(assets_amount);
            }
            PromiseResult::Failed => {
                env::panic_str("Claim transfer failed");
            }
        }
    }

    pub fn withdraw(&mut self, amount: u128) -> Promise {
        let status = self.get_status();
        let is_price_discovery_ongoing = matches!(self.config.mechanics, Mechanics::PriceDiscovery)
            && matches!(status, LaunchpadStatus::Ongoing);
        let is_withdrawal_allowed = is_price_discovery_ongoing
            || matches!(status, LaunchpadStatus::Failed)
            || matches!(status, LaunchpadStatus::Locked);
        require!(is_withdrawal_allowed, "Withdraw not allowed");
        let Some(intent_account) = self.accounts.get(&env::predecessor_account_id()) else {
            env::panic_str("Intent account isn't found for the user");
        };

        let Some(investment) = self.investments.get_mut(intent_account) else {
            env::panic_str("No deposits found for the intent account");
        };

        let mut amount = amount;
        // For Price Discovery mechanics, we allow partial withdrawal
        if matches!(self.config.mechanics, Mechanics::PriceDiscovery) {
            if let Err(err) = mechanics::withdraw::withdraw(
                investment,
                amount,
                &mut self.total_deposited,
                &mut self.total_sold_tokens,
                &self.config,
                env::block_timestamp(),
            ) {
                env::panic_str(&format!("Withdraw failed: {err}"));
            }
        } else {
            // For other mechanics, we withdraw the full amount
            amount = investment.amount;
            // Withdraw all funds from the investment
            investment.amount = 0;
            // Reset weight to zero, as we are withdrawing all funds
            investment.weight = 0;
        }

        // ToDo: We need to add a possibility to withdraw tokens to Intents as well.
        // In this case we have to use `ft_transfer_call` or `mt_transfer_call` methods instead.

        match &self.config.deposit_token {
            DepositToken::Nep141(account_id) => ext_ft::ext(account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                .ft_transfer(env::predecessor_account_id(), amount.into(), None),
            DepositToken::Nep245((account_id, token_id)) => ext_mt::ext(account_id.clone())
                .with_attached_deposit(ONE_YOCTO)
                .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                .mt_transfer(
                    env::predecessor_account_id(),
                    token_id.clone(),
                    amount.into(),
                    None,
                    None,
                ),
        }
    }

    pub fn distribute_tokens(&mut self) -> Promise {
        require!(
            self.is_success(),
            "Distribution can be called only if the launchpad finishes with success status"
        );
        require!(!self.is_distributed, "Tokens already distributed");
        // TODO: Check permission to distribute tokens
        // require!(env.predecessor_account_id() == ?, "Permission denied");

        // Distribute to solver
        let promise_res = ext_ft::ext(self.config.sale_token_account_id.clone())
            .with_attached_deposit(ONE_YOCTO)
            .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
            .ft_transfer_call(
                self.config.intents_account_id.clone(),
                self.config.distribution_proportions.solver_allocation,
                self.config
                    .distribution_proportions
                    .solver_account_id
                    .as_ref()
                    .to_string(),
                None,
            );

        self.config
            .distribution_proportions
            .stakeholder_proportions
            .iter()
            .fold(promise_res, |promise, proportion| {
                promise.and(
                    ext_ft::ext(self.config.sale_token_account_id.clone())
                        .with_attached_deposit(ONE_YOCTO)
                        .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
                        .ft_transfer_call(
                            self.config.intents_account_id.clone(),
                            proportion.allocation,
                            proportion.account.as_ref().to_string(),
                            None,
                        ),
                )
            })
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_FINISH_DISTRIBUTION)
                    .finish_distribution(),
            )
    }

    #[private]
    pub fn finish_distribution(&mut self) {
        require!(
            env::promise_results_count() > 0,
            "Expected at least one promise result"
        );

        // Check if all promises were successful
        for i in 0..env::promise_results_count() {
            match env::promise_result(i) {
                PromiseResult::Successful(_) => {}
                PromiseResult::Failed => env::panic_str("Distribution failed"),
            }
        }

        // Set distributed flag
        self.is_distributed = true;
    }

    #[pause]
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    ) -> PromiseOrValue<U128> {
        let _ = (sender_id, memo);
        let token_account_id = env::predecessor_account_id();

        if token_account_id == self.config.sale_token_account_id {
            self.init_contract(amount)
        } else if self.is_nep141_deposit_token(&token_account_id) {
            self.handle_deposit(amount, &msg)
        } else {
            env::panic_str("Unsupported NEP-141 token");
        }
    }

    #[pause]
    pub fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        let _ = (sender_id, previous_owner_ids);
        require!(
            self.is_nep245_deposit_token(&env::predecessor_account_id(), &token_ids),
            "Wrong NEP-245 deposit token"
        );

        match self.handle_deposit(amounts[0], &msg) {
            PromiseOrValue::Promise(promise) => PromiseOrValue::Promise(promise),
            PromiseOrValue::Value(value) => PromiseOrValue::Value(vec![value]),
        }
    }

    fn init_contract(&mut self, amount: U128) -> PromiseOrValue<U128> {
        if self.is_sale_token_set {
            env::panic_str("The contract is already initialized");
        }

        require!(
            amount == self.config.total_sale_amount,
            "Wrong total sale amount"
        );

        self.is_sale_token_set = true;
        PromiseOrValue::Value(0.into())
    }

    fn handle_deposit(&mut self, amount: U128, msg: &str) -> PromiseOrValue<U128> {
        require!(self.is_ongoing(), "Launchpad is not ongoing");
        // Get NEAR and IntentAccount from the message
        let (near_account_id, intent_account_id) =
            parse_accounts(msg).unwrap_or_else(|err| env::panic_str(err));
        // Insert IntentAccount to the accounts map if it doesn't exist
        self.accounts.entry(near_account_id).or_insert_with(|| {
            self.participants_count += 1;
            intent_account_id.clone()
        });

        near_sdk::log!("Depositing amount: {} for: {intent_account_id}", amount.0);

        let investments = self.investments.entry(intent_account_id).or_default();

        let deposit_result = mechanics::deposit::deposit(
            investments,
            amount.0,
            &mut self.total_deposited,
            &mut self.total_sold_tokens,
            &self.config,
            env::block_timestamp(),
        );
        let remain = match deposit_result {
            Ok(val) => val,
            Err(err) => env::panic_str(&format!("Deposit failed: {err}")),
        };

        PromiseOrValue::Value(remain.into())
    }

    fn is_nep141_deposit_token(&self, predecessor_account_id: &AccountId) -> bool {
        matches!(&self.config.deposit_token, DepositToken::Nep141(account_id) if account_id == predecessor_account_id)
    }

    fn is_nep245_deposit_token(
        &self,
        predecessor_account_id: &AccountId,
        token_ids: &[TokenId],
    ) -> bool {
        require!(
            token_ids.len() == 1,
            "Only one token_id is allowed for deposit"
        );
        matches!(&self.config.deposit_token, DepositToken::Nep245((account_id, token_id)) if account_id == predecessor_account_id && token_id == &token_ids[0])
    }
}
