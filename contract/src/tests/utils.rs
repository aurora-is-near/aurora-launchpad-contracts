use crate::{DepositToken, DistributionProportions, IntentAccount};
use aurora_launchpad_types::config::{LaunchpadConfig, Mechanics, StakeholderProportion};
use near_sdk::json_types::U128;

pub const DEPOSIT_TOKEN_ID: &str = "wrap.near";
pub const SALE_TOKEN_ID: &str = "sale.token.near";
pub const INTENTS_ACCOUNT_ID: &str = "intents.near";
pub const SOLVER_ACCOUNT_ID: &str = "solver.near";
pub const NOW: u64 = 1_000_000_000;
pub const TEN_DAYS: u64 = 10 * 24 * 60 * 60;

pub fn base_config(mechanics: Mechanics) -> LaunchpadConfig {
    LaunchpadConfig {
        deposit_token: DepositToken::Nep141(DEPOSIT_TOKEN_ID.parse().unwrap()),
        sale_token_account_id: SALE_TOKEN_ID.parse().unwrap(),
        intents_account_id: INTENTS_ACCOUNT_ID.parse().unwrap(),
        start_date: NOW,
        end_date: NOW + TEN_DAYS,
        // 24 decimals - for deposited tokens
        soft_cap: U128(10u128.pow(30)), // 1 Million tokens
        mechanics,
        // 18 decimals
        sale_amount: U128(3 * 10u128.pow(24)), // 3 Million tokens
        // 18 decimals
        total_sale_amount: U128(10u128.pow(25)), // 10 Million tokens
        vesting_schedule: None,
        distribution_proportions: DistributionProportions {
            solver_account_id: IntentAccount(SOLVER_ACCOUNT_ID.to_string()),
            // 18 decimals
            solver_allocation: U128(5 * 10u128.pow(24)), // 5 Million tokens
            stakeholder_proportions: vec![StakeholderProportion {
                // 18 decimals
                allocation: U128(2 * 10u128.pow(24)), // 2 Million tokens
                account: IntentAccount("team.near".to_string()),
                vesting_schedule: None,
            }],
        },
        discounts: vec![],
    }
}

pub fn price_discovery_config() -> LaunchpadConfig {
    base_config(Mechanics::PriceDiscovery)
}

pub fn fixed_price_config() -> LaunchpadConfig {
    // 20 sale tokens = 1 deposit token
    base_config(Mechanics::FixedPrice {
        // Deposit - 24 decimals
        deposit_token: U128(100_000),
        // Deposit - 18 decimals
        sale_token: U128(2),
    })
}
