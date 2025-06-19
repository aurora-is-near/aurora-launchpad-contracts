use aurora_launchpad_types::config::{
    DistributionProportions, LaunchpadConfig, LaunchpadStatus, LaunchpadToken, Mechanics,
    VestingSchedule,
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
use primitive_types::U256;

const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(5);
const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

// For some reason, the clippy lints are not working properly in that macro
#[allow(dead_code)]
#[ext_contract(ext_ft)]
trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
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
    /// Assets of the launchpad, used for distributions and users rewards
    pub token_account_id: LazyOption<AccountId>,
    /// Number of unique participants in the launchpad
    pub participants_count: u64,
    /// Total amount of tokens deposited by users
    pub total_deposited: u128,
    /// User investments in the launchpad
    pub investments: LookupMap<IntentAccount, InvestmentAmount>,
    /// Start timestamp of the vesting period, if applicable
    pub vesting_start_timestamp: LazyOption<u64>,
    /// Vesting users state with claimed amounts
    pub vestings: LookupMap<IntentAccount, u128>,
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
            vesting_start_timestamp: LazyOption::new(b"vesting_start_timestamp".to_vec(), None),
            vestings: LookupMap::new(b"vestings".to_vec()),
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

    pub fn get_token_account_id(&self) -> Option<AccountId> {
        self.token_account_id.get().clone()
    }

    pub const fn get_participants_count(&self) -> u64 {
        self.participants_count
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

    pub fn get_distribution_proportions(&self) -> Vec<DistributionProportions> {
        self.config.distribution_proportions.clone()
    }

    pub fn get_deposit_token_account_id(&self) -> AccountId {
        self.config.deposit_token_account_id.clone()
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

        ext_ft::ext(self.config.deposit_token_account_id.clone())
            .with_attached_deposit(ONE_YOCTO)
            .with_static_gas(GAS_FOR_FT_TRANSFER)
            .ft_transfer(intent_account_id, 0.into(), Some(account.0))
    }

    pub fn withdraw(&mut self, account: &IntentAccount) -> PromiseOrValue<U128> {
        require!(
            self.is_failed(),
            "Withdraw can be called only if the launchpad finishes with fail status"
        );
        let _ = account;
        // Withdraw only if Status is `Fail`
        // Check permission to withdraw
        // require!( WE_SHOULD_DECIDE_HOW_TO_WITHDRAW, "Permission denied" );
        // - transfer all user deposited assets to the user Intent account
        todo!()
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
    ) -> PromiseOrValue<U128> {
        require!(self.is_ongoing(), "Launchpad is not ongoing");
        require!(
            self.config.deposit_token_account_id == env::predecessor_account_id(),
            "Wrong investment token"
        );
        // Get IntentAccount from the message
        require!(!msg.is_empty(), "Invalid transfer token message format");
        let account = IntentAccount(msg);

        let current_timestamp = env::block_timestamp();
        // Find first discount item that is active at the current timestamp.
        // It's allowed only one discount item to be active at the same time.
        let discount = self
            .config
            .discounts
            .iter()
            .find(|d| d.start_date <= current_timestamp && current_timestamp < d.end_date);

        // Apply discount if it exists
        let mut remain = 0;
        match self.config.mechanics {
            Mechanics::FixedPrice { price } => {
                self.investments.entry(account).and_modify(|x| {
                    // NOTE: we do not calculate assets by formula: `amount / price` to avoid fractional assets
                    // We just calculating soft cap threshold and return the remaining amount to the sender
                    // To coplete logic fo FixedPrice we calculating assets in the `claim` stage
                    x.amount += amount.0;
                    let mut assets = if let Some(discount) = discount {
                        // To avoid overflow, we use U256 for calculations
                        (U256::from(x.amount) * U256::from(discount.percentage) / U256::from(100))
                    } else {
                        U256::from(amount.0)
                    };
                    // Check discount and apply it
                    self.total_deposited += assets;
                    // To avoid fractional assets, we multiply the soft cap by the price
                    let soft_cap = U256::from(self.config.soft_cap.0) * U256::from(price);
                    let total_deposited = U256::from(self.total_deposited);
                    // Check if the soft cap is reached
                    if soft_cap < total_deposited {
                        // Calculate the amount of assets remaining to reach the soft cap
                        let assets_remain = soft_cap - total_deposited;
                        // Decrease user assets to the soft cap threshold
                        assets -= assets_remain;
                        // Calculate the amount that should be returned to the sender including discount logic
                        remain = if let Some(discount) = discount {
                            (assets_remain * U256::from(100) / U256::from(discount)).as_u128()
                        } else {
                            assets_remain.as_u128()
                        };
                        // Return the amount that should be returned to the sender
                        x.amount -= remain;
                    }
                    // Increase user assets
                    x.assets += assets.as_u128();
                });
            }
            Mechanics::PriceDiscovery => {
                self.investments.entry(account).and_modify(|x| {
                    x.amount += amount.0;
                    let mut assets = if let Some(discount) = discount {
                        // To avoid overflow, we use U256 for calculations
                        (U256::from(x.amount) * U256::from(discount.percentage) / U256::from(100))
                            .as_u128()
                    } else {
                        amount.0
                    };
                    // Check discount and apply it
                    self.total_deposited += assets;
                    // Check if the soft cap is reached
                    if self.config.soft_cap.0 < self.total_deposited {
                        // Calculate the amount of assets remaining to reach the soft cap
                        let assets_remain = self.config.soft_cap.0 - self.total_deposited;
                        // Decrease user assets to the soft cap threshold
                        assets -= assets_remain;
                        // Calculate the amount that should be returned to the sender including discount logic
                        remain = if let Some(discount) = discount {
                            (U256::from(assets_remain) * U256::from(100) / U256::from(discount))
                                .as_u128()
                        } else {
                            assets_remain
                        };
                        // Return the amount that should be returned to the sender
                        x.amount -= remain;
                    }
                    // Increase user assets
                    x.assets += assets;
                });
            }
        }
        if remain > 0 {
            // If the soft cap is reached, return the remaining amount to the sender
            PromiseOrValue::Promise(
                // It should return back to user Intent account
                // TODO: check is receiver correct
                ext_ft::ext(self.config.deposit_token_account_id.clone())
                    .with_attached_deposit(ONE_YOCTO)
                    .with_static_gas(GAS_FOR_FT_TRANSFER)
                    .ft_transfer(sender_id, remain.into(), Some(account.0.clone())),
            )
        } else {
            // Otherwise, just return 0
            PromiseOrValue::Value(0.into())
        }
    }

    #[pause]
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
