use crate::utils::parse_accounts;
use aurora_launchpad_types::config::{
    DepositToken, DistributionProportions, LaunchpadConfig, LaunchpadStatus, LaunchpadToken,
    Mechanics, TokenId, VestingSchedule,
};
use aurora_launchpad_types::{IntentAccount, InvestmentAmount};
use near_plugins::{AccessControlRole, AccessControllable, Pausable, access_control, pause};
use near_sdk::borsh::BorshDeserialize;
use near_sdk::json_types::U128;
use near_sdk::store::{LazyOption, LookupMap};
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseOrValue, env, ext_contract, near,
    require,
};

mod mechanics;
mod utils;

const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas::from_tgas(70);
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
}

#[near]
impl AuroraLaunchpadContract {
    #[init]
    #[must_use]
    #[allow(clippy::use_self)]
    pub fn new(config: LaunchpadConfig) -> Self {
        Self {
            config,
            participants_count: 0,
            total_deposited: 0,
            investments: LookupMap::new(b"investments".to_vec()),
            vesting_start_timestamp: LazyOption::new(b"vesting_start_timestamp".to_vec(), None),
            vestings: LookupMap::new(b"vestings".to_vec()),
            accounts: LookupMap::new(b"accounts".to_vec()),
            is_sale_token_set: false,
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

    pub fn get_status(&self) -> LaunchpadStatus {
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

    pub fn get_config(&self) -> LaunchpadConfig {
        self.config.clone()
    }

    pub const fn get_participants_count(&self) -> u64 {
        self.participants_count
    }

    pub fn get_total_deposited(&self) -> U128 {
        self.total_deposited.into()
    }

    pub fn get_investments(&self, account: &IntentAccount) -> Option<U128> {
        self.investments.get(account).map(|s| U128(s.amount))
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

    pub const fn get_sale_amount(&self) -> U128 {
        self.config.sale_amount
    }

    pub fn get_sale_token_account(&self) -> AccountId {
        self.config.sale_token_account_id.clone()
    }

    pub const fn get_total_sale_amount(&self) -> U128 {
        self.config.total_sale_amount
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

    pub fn get_distribution_proportions(&self) -> Vec<DistributionProportions> {
        self.config.distribution_proportions.clone()
    }

    pub fn get_deposit_token_account_id(&self) -> DepositToken {
        self.config.deposit_token.clone()
    }

    pub fn claim(&mut self, account: IntentAccount) -> Promise {
        use std::str::FromStr;

        require!(
            self.is_success(),
            "Claim can be called only if the launchpad finishes with success status"
        );
        // Transfer all assets to Intents account with message containing User id:
        //   - according rules of vesting schedule (if any) to the user Intent account
        //   - according deposit weight related to specified Mechanics
        //   - Launchpad assets to the user Intent account
        let Ok(intent_account_id) = AccountId::from_str("intents.near") else {
            env::panic_str("Invalid account id");
        };

        ext_ft::ext(self.config.sale_token_account_id.clone())
            .with_attached_deposit(ONE_YOCTO)
            .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
            .ft_transfer_call(intent_account_id, 0.into(), account.0, None)
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

        if let Err(err) = mechanics::withdraw(
            investment,
            amount,
            &mut self.total_deposited,
            &self.config,
            env::block_timestamp(),
        ) {
            env::panic_str(&format!("Withdraw failed: {err}"));
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

    pub fn distribute_tokens(&mut self) {
        require!(
            self.is_success(),
            "Claim can be called only if the launchpad finishes with success status"
        );
        // Check permission to distribute tokens
        // require!(env.predecessor_account_id() == ?, "Permission denied");
        // - Method should be called only when status is success
        // - Method called only once
        // - All assets should be transferred to the Pool account
        todo!()
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
        if !self.is_sale_token_set {
            require!(
                env::predecessor_account_id() == self.config.sale_token_account_id,
                "Contract not initialized or sale token account is wrong"
            );
            require!(
                amount == self.config.total_sale_amount,
                "Wrong total sale amount"
            );

            self.is_sale_token_set = true;
            return PromiseOrValue::Value(0.into());
        }

        require!(self.is_ongoing(), "Launchpad is not ongoing");
        require!(
            self.is_nep141_deposit_token(&env::predecessor_account_id()),
            "Wrong deposit token"
        );

        // Get NEAR and IntentAccount from the message
        let (near_account_id, intent_account_id) =
            parse_accounts(&msg).unwrap_or_else(|err| env::panic_str(err));
        // Insert IntentAccount to the accounts map if it doesn't exist
        self.accounts
            .entry(near_account_id)
            .or_insert(intent_account_id.clone());

        // Apply discount if it exists
        let mut remain: u128 = 0;

        self.investments
            .entry(intent_account_id)
            .and_modify(|investment| {
                let deposit_result = mechanics::deposit(
                    investment,
                    amount.0,
                    &mut self.total_deposited,
                    &mut self.total_sold_tokens,
                    &self.config,
                    env::block_timestamp(),
                );
                remain = match deposit_result {
                    Ok(val) => val,
                    Err(err) => env::panic_str(&format!("Deposit failed: {err}")),
                };
            });

        PromiseOrValue::Value(remain.into())
    }

    #[pause]
    pub fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let _ = (sender_id, previous_owner_ids);

        require!(self.is_ongoing(), "Launchpad is not ongoing");
        require!(
            self.is_nep245_deposit_token(&env::predecessor_account_id(), &token_ids),
            "Wrong deposit token"
        );

        // Get NEAR and IntentAccount from the message
        let (near_account_id, intent_account_id) =
            parse_accounts(&msg).unwrap_or_else(|err| env::panic_str(err));
        // Insert IntentAccount to the accounts map if it doesn't exist
        self.accounts
            .entry(near_account_id)
            .or_insert(intent_account_id.clone());

        // Apply discount if it exists
        let mut remain: u128 = 0;

        self.investments
            .entry(intent_account_id)
            .and_modify(|investment| {
                remain = mechanics::deposit(
                    investment,
                    amounts[0].0,
                    &mut self.total_deposited,
                    &mut self.total_sold_tokens,
                    &self.config,
                    env::block_timestamp(),
                )
                .unwrap_or_else(|err| panic!("Deposit failed: {err}"));
            });

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
