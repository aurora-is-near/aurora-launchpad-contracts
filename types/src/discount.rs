use alloy_primitives::ruint::aliases::U256;
use near_sdk::near;

use crate::config::LaunchpadConfig;
use crate::date_time;
use crate::utils::to_u128;

/// Represents a discount that can be applied to the launchpad sale for a period.
#[derive(Debug, Eq, PartialEq, Clone)]
#[near(serializers = [borsh, json])]
pub struct Discount {
    /// The start date of the discount period in nanoseconds.
    #[serde(with = "date_time")]
    pub start_date: u64,
    /// The end date of the discount period in nanoseconds.
    #[serde(with = "date_time")]
    pub end_date: u64,
    /// The percentage of the discount, represented as percent * 10, so min percent is 0.1 %.
    pub percentage: u16,
}

impl Discount {
    const MULTIPLIER: u16 = 10_000;

    pub fn get_weight(
        config: &LaunchpadConfig,
        amount: u128,
        timestamp: u64,
    ) -> Result<u128, &'static str> {
        use alloy_primitives::ruint::aliases::U256;

        config
            .get_current_discount(timestamp)
            .map_or(Ok(amount), |disc| {
                // Overflow impossible as percentage is u16 and the amount is u128
                let res = U256::from(amount)
                    * U256::from(Self::MULTIPLIER.saturating_add(disc.percentage))
                    / U256::from(Self::MULTIPLIER);

                to_u128(res)
            })
    }

    pub fn get_funds_without_discount(
        config: &LaunchpadConfig,
        amount: u128,
        timestamp: u64,
    ) -> Result<u128, &'static str> {
        config
            .get_current_discount(timestamp)
            .map_or(Ok(amount), |disc| {
                let res = U256::from(amount) * U256::from(Self::MULTIPLIER)
                    / U256::from(Self::MULTIPLIER.saturating_add(disc.percentage));

                to_u128(res)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IntentAccount;
    use crate::config::{DepositToken, DistributionProportions, Mechanics, StakeholderProportion};
    use near_sdk::json_types::U128;

    pub const DEPOSIT_TOKEN_ID: &str = "wrap.near";
    pub const SALE_TOKEN_ID: &str = "sale.token.near";
    pub const INTENTS_ACCOUNT_ID: &str = "intents.near";
    pub const SOLVER_ACCOUNT_ID: &str = "solver.near";
    pub const NOW: u64 = 1_000_000_000;
    pub const TEN_DAYS: u64 = 10 * 24 * 60 * 60;

    pub fn base_config() -> LaunchpadConfig {
        let mechanics = Mechanics::PriceDiscovery;
        LaunchpadConfig {
            deposit_token: DepositToken::Nep141(DEPOSIT_TOKEN_ID.parse().unwrap()),
            sale_token_account_id: SALE_TOKEN_ID.parse().unwrap(),
            intents_account_id: INTENTS_ACCOUNT_ID.parse().unwrap(),
            start_date: NOW,
            end_date: NOW + TEN_DAYS,
            soft_cap: U128(10u128.pow(30)), // 1 Million tokens
            mechanics,
            // 18 decimals
            sale_amount: U128(3 * 10u128.pow(24)), // 3 Million tokens
            // 18 decimals
            total_sale_amount: U128(10u128.pow(25)), // 10 Million tokens
            vesting_schedule: None,
            distribution_proportions: DistributionProportions {
                solver_account_id: IntentAccount(SOLVER_ACCOUNT_ID.to_string()),
                solver_allocation: U128(5 * 10u128.pow(24)), // 5 Million tokens
                stakeholder_proportions: vec![StakeholderProportion {
                    allocation: U128(2 * 10u128.pow(24)), // 2 Million tokens
                    account: IntentAccount("team.near".to_string()),
                }],
            },
            discounts: vec![],
        }
    }

    #[test]
    fn test_get_weight_no_discount() {
        let config = base_config();
        let result = Discount::get_weight(&config, 1_000, 0);
        assert_eq!(result.unwrap(), 1_000);
    }

    #[test]
    fn test_get_weight_with_discount() {
        let mut config = base_config();
        config.discounts.push(Discount {
            start_date: 0,
            end_date: 1000,
            percentage: 2_000, // 20%
        });
        let result = Discount::get_weight(&config, 1_000, 500);
        assert_eq!(result.unwrap(), 1_200);
    }

    #[test]
    fn test_get_weight_overflow_u128() {
        let mut config = base_config();
        config.discounts.push(Discount {
            start_date: 0,
            end_date: 1000,
            percentage: u16::MAX,
        });
        let result = Discount::get_weight(&config, u128::MAX, 500);
        println!("{result:?}");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_weight_with_double_discount_for_same_period() {
        let mut config = base_config();
        config.discounts = vec![
            Discount {
                start_date: 400,
                end_date: 1400,
                percentage: 2_000, // 20%
            },
            Discount {
                start_date: 0,
                end_date: 1000,
                percentage: 1_000, // 10%
            },
        ];
        let result = Discount::get_weight(&config, 1_000, 500);
        assert_eq!(result.unwrap(), 1_200);
    }

    #[test]
    fn test_get_weight_with_double_discount_for_different_period() {
        let mut config = base_config();
        config.discounts = vec![
            Discount {
                start_date: 550,
                end_date: 1550,
                percentage: 2_000, // 20%
            },
            Discount {
                start_date: 0,
                end_date: 1000,
                percentage: 1_000, // 10%
            },
        ];
        let result = Discount::get_weight(&config, 1_000, 500);
        assert_eq!(result.unwrap(), 1_100);
    }

    #[test]
    fn test_get_weight_with_double_discount_check_start_date() {
        let mut config = base_config();
        config.discounts = vec![
            Discount {
                start_date: 550,
                end_date: 1550,
                percentage: 2_000, // 20%
            },
            Discount {
                start_date: 200,
                end_date: 1000,
                percentage: 1_000, // 10%
            },
        ];
        let result = Discount::get_weight(&config, 1_000, 100);
        assert_eq!(result.unwrap(), 1_000);
    }

    #[test]
    fn test_get_weight_with_double_discount_check_end_date() {
        let mut config = base_config();
        config.discounts = vec![
            Discount {
                start_date: 550,
                end_date: 1550,
                percentage: 2_000, // 20%
            },
            Discount {
                start_date: 0,
                end_date: 1000,
                percentage: 1_000, // 10%
            },
        ];
        let result = Discount::get_weight(&config, 1_000, 1550);
        assert_eq!(result.unwrap(), 1_000);
    }

    #[test]
    fn test_get_funds_no_discount() {
        let config = base_config();
        let result = Discount::get_funds_without_discount(&config, 1_000, 0);
        assert_eq!(result.unwrap(), 1_000);
    }

    #[test]
    fn test_get_funds_with_discount() {
        let mut config = base_config();
        config.discounts.push(Discount {
            start_date: 0,
            end_date: 1000,
            percentage: 2_000, // 20%
        });
        let result = Discount::get_funds_without_discount(&config, 1_200, 500);
        assert_eq!(result.unwrap(), 1_000);
    }

    #[test]
    fn test_get_funds_no_overflow_u128() {
        // Overflow impossible as percentage
        let mut config = base_config();
        config.discounts.push(Discount {
            start_date: 0,
            end_date: 1000,
            percentage: u16::MAX,
        });
        let result = Discount::get_funds_without_discount(&config, u128::MAX, 500);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_funds_with_double_discount_for_same_period() {
        let mut config = base_config();
        config.discounts = vec![
            Discount {
                start_date: 400,
                end_date: 1400,
                percentage: 2_000, // 20%
            },
            Discount {
                start_date: 0,
                end_date: 1000,
                percentage: 1_000, // 10%
            },
        ];
        let result = Discount::get_funds_without_discount(&config, 1_200, 500);
        assert_eq!(result.unwrap(), 1_000);
    }

    #[test]
    fn test_get_funds_with_double_discount_for_different_period() {
        let mut config = base_config();
        config.discounts = vec![
            Discount {
                start_date: 550,
                end_date: 1550,
                percentage: 2_000, // 20%
            },
            Discount {
                start_date: 0,
                end_date: 1000,
                percentage: 1_000, // 10%
            },
        ];
        let result = Discount::get_funds_without_discount(&config, 1_100, 500);
        assert_eq!(result.unwrap(), 1_000);
    }

    #[test]
    fn test_get_funds_with_double_discount_check_start_date() {
        let mut config = base_config();
        config.discounts = vec![
            Discount {
                start_date: 550,
                end_date: 1550,
                percentage: 2_000, // 20%
            },
            Discount {
                start_date: 200,
                end_date: 1000,
                percentage: 1_000, // 10%
            },
        ];
        let result = Discount::get_funds_without_discount(&config, 1_000, 100);
        assert_eq!(result.unwrap(), 1_000);
    }

    #[test]
    fn test_get_funds_with_double_discount_check_end_date() {
        let mut config = base_config();
        config.discounts = vec![
            Discount {
                start_date: 550,
                end_date: 1550,
                percentage: 2_000, // 20%
            },
            Discount {
                start_date: 0,
                end_date: 1000,
                percentage: 1_000, // 10%
            },
        ];
        let result = Discount::get_funds_without_discount(&config, 1_000, 1550);
        assert_eq!(result.unwrap(), 1_000);
    }
}
