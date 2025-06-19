use crate::IntentAccount;
use near_sdk::json_types::U128;
use near_sdk::{AccountId, near};

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct LaunchpadConfig {
    /// Launchpad token configuration.
    pub token: LaunchpadToken,
    /// The NEP-141 account of the token accepted for deposits. E.g. wrap.near
    pub deposit_token_account_id: AccountId,
    /// Start timestamp of the sale.
    pub start_date: u64,
    /// End timestamp of the sale.
    pub end_date: u64,
    /// The threshold or minimum sale amount denominated in the deposit token.
    pub soft_cap: U128,
    /// Sale mechanics, which can be either fixed price or price discovery etc.
    pub mechanics: Mechanics,
    /// Maximum (in case of fixed price) and total (in case of price discovery) amount of tokens used for the sale.
    pub sale_amount: Option<U128>,
    /// The account of the Solver dedicated to the token sale.
    pub solver_account_id: AccountId,
    /// The amount of tokens that should be matched against a portion of the sale liquidity and put into the TEE-based solver
    pub solver_allocation: U128,
    /// An optional vesting schedule.
    pub vesting_schedule: Option<VestingSchedule>,
    /// An array of distributions between different accounts, including specific amounts and accounts.
    pub distribution_proportions: Vec<DistributionProportions>,
    /// An optional array of discounts defined for the sale.
    pub discounts: Vec<Discount>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct LaunchpadToken {
    pub total_supply: U128,
    pub name: String,
    pub symbol: String,
    pub icon: String,
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub enum Mechanics {
    FixedPrice { price: U128 },
    PriceDiscovery,
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct DistributionProportions {
    account: IntentAccount,
    allocation: u16,
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct VestingSchedule {
    /// Vesting cliff period in seconds (for example 6 months)
    pub cliff_period: u64,
    /// Vesting period in seconds (fro example 18 months)
    pub vesting_period: u64,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [json])]
pub enum LaunchpadStatus {
    NotStarted,
    Ongoing,
    Success,
    Failed,
    Locked,
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct Discount {
    pub start_date: u64,
    pub end_date: u64,
    pub percentage: u16,
}
