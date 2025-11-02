use aurora_launchpad_types::config::{
    DepositToken, DistributionAccount, DistributionProportions, LaunchpadConfig, Mechanics,
    StakeholderProportion,
};
use near_sdk::json_types::U128;

pub const DEPOSIT_TOKEN_ID: &str = "wrap.near";
pub const SALE_TOKEN_ID: &str = "sale.token.near";
pub const INTENTS_ACCOUNT_ID: &str = "intents.near";
pub const SOLVER_ACCOUNT_ID: &str = "solver.near";
pub const NOW: u64 = 1_000_000_000;
pub const TEN_DAYS: u64 = 10 * 24 * 60 * 60;

const MULTIPLIER_18: u128 = 10u128.pow(18);
const MULTIPLIER_24: u128 = 10u128.pow(24);

pub fn base_config(mechanics: Mechanics) -> LaunchpadConfig {
    LaunchpadConfig {
        deposit_token: DepositToken::Nep141(DEPOSIT_TOKEN_ID.parse().unwrap()),
        min_deposit: 100_000.into(),
        sale_token_account_id: SALE_TOKEN_ID.parse().unwrap(),
        intents_account_id: INTENTS_ACCOUNT_ID.parse().unwrap(),
        start_date: NOW,
        end_date: NOW + TEN_DAYS,
        // 24 decimals - for deposited tokens
        soft_cap: U128(1_000_000 * MULTIPLIER_24), // 1 Million tokens
        mechanics,
        sale_amount: U128(3_000_000 * MULTIPLIER_18),
        total_sale_amount: U128(10_000_000 * MULTIPLIER_18),
        vesting_schedule: None,
        distribution_proportions: DistributionProportions {
            solver_account_id: DistributionAccount::new_near(SOLVER_ACCOUNT_ID).unwrap(),
            solver_allocation: U128(5_000_000 * MULTIPLIER_18),
            stakeholder_proportions: vec![StakeholderProportion {
                allocation: U128(2_000_000 * MULTIPLIER_18),
                account: DistributionAccount::new_near("team.near").unwrap(),
                vesting: None,
            }],
            deposits: None,
        },
        discounts: None,
    }
}

pub fn price_discovery_config() -> LaunchpadConfig {
    base_config(Mechanics::PriceDiscovery)
}

pub fn fixed_price_config() -> LaunchpadConfig {
    base_config(Mechanics::FixedPrice {
        // Deposit - 24 decimals
        deposit_token: U128(50_000),
        // Deposit - 18 decimals
        sale_token: U128(1),
    })
}
