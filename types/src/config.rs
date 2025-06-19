use near_sdk::json_types::U128;
use near_sdk::{AccountId, near};

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct LaunchpadConfig {
    pub token: LaunchpadToken,
    pub deposit_token_account_id: AccountId,
    pub start_date: u64,
    pub end_date: u64,
    pub soft_cap: U128,
    pub mechanics: Mechanics,
    // Maximum (in case of fixed price) and total (in case of price discovery)
    pub sale_amount: Option<U128>,
    pub solver_allocation: U128,
    pub vesting_schedule: Option<VestingSchedule>,
    pub distribution_proportions: DistributionProportions,
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
pub enum DistributionProportions {
    FixedPrice,
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub enum VestingSchedule {
    Scheme1,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers = [json])]
pub enum LaunchpadStatus {
    NotStarted,
    Ongoing,
    Success,
    Failed,
}
