use alloy_primitives::ruint::aliases::U256;
use aurora_launchpad_types::InvestmentAmount;
use aurora_launchpad_types::config::{LaunchpadConfig, Mechanics, VestingSchedule};
use aurora_launchpad_types::utils::to_u128;

/// Calculates the total assets for user allocation based on the mechanics and vesting schedule.
pub fn user_allocation(
    weight: u128,
    total_sold_tokens: u128,
    config: &LaunchpadConfig,
) -> Result<u128, &'static str> {
    match config.mechanics {
        Mechanics::FixedPrice { .. } => Ok(weight),
        Mechanics::PriceDiscovery => {
            if weight == 0 || total_sold_tokens == 0 {
                return Ok(0);
            }

            U256::from(weight)
                .checked_mul(U256::from(config.sale_amount.0))
                .ok_or("Multiplication overflow")
                .map(|result| result / U256::from(total_sold_tokens))
                .and_then(to_u128)
        }
    }
}

/// Calculates the available assets for claim based on the mechanics and vesting schedule.
/// Notice that the function doesn't subtract already claimed tokens.
pub fn available_for_claim(
    investment: &InvestmentAmount,
    total_sold_tokens: u128,
    config: &LaunchpadConfig,
    timestamp: u64,
) -> Result<u128, &'static str> {
    let total_assets = user_allocation(investment.weight, total_sold_tokens, config)?;

    if let Some(vesting) = &config.vesting_schedule {
        let vesting_start = config.end_date;

        if timestamp < vesting_start + vesting.cliff_period.as_nanos() {
            let assets_for_instant_claim = vesting.get_instant_claim_amount(total_assets)?;
            return Ok(assets_for_instant_claim);
        } else if timestamp >= vesting_start + vesting.vesting_period.as_nanos() {
            return Ok(total_assets);
        }

        let elapsed = timestamp.saturating_sub(vesting_start);

        U256::from(total_assets)
            .checked_mul(U256::from(elapsed))
            .ok_or("Multiplication overflow")
            .map(|result| {
                result
                    .checked_div(U256::from(vesting.vesting_period.as_nanos()))
                    .unwrap_or_default()
            })
            .and_then(to_u128)
    } else {
        Ok(total_assets)
    }
}

/// Returns the available assets for individual vesting claim based on the allocation and vesting
/// schedule. Notice that the function doesn't subtract already claimed tokens.
pub fn available_for_individual_vesting_claim(
    allocation: u128,
    vesting: Option<&VestingSchedule>,
    vesting_start: u64,
    timestamp: u64,
) -> Result<u128, &'static str> {
    if let Some(vesting) = &vesting {
        if timestamp < vesting_start + vesting.cliff_period.as_nanos() {
            let assets_for_instant_claim = vesting.get_instant_claim_amount(allocation)?;
            return Ok(assets_for_instant_claim);
        } else if timestamp >= vesting_start + vesting.vesting_period.as_nanos() {
            return Ok(allocation);
        }

        let elapsed = timestamp.saturating_sub(vesting_start);

        U256::from(allocation)
            .checked_mul(U256::from(elapsed))
            .ok_or("Multiplication overflow")
            .map(|result| {
                result
                    .checked_div(U256::from(vesting.vesting_period.as_nanos()))
                    .unwrap_or_default()
            })
            .and_then(to_u128)
    } else {
        Ok(allocation)
    }
}

#[cfg(test)]
mod tests {
    use crate::mechanics::claim::{
        available_for_claim, available_for_individual_vesting_claim, user_allocation,
    };
    use crate::tests::utils::price_discovery_config;
    use aurora_launchpad_types::InvestmentAmount;
    use aurora_launchpad_types::config::VestingSchedule;
    use near_sdk::json_types::U128;

    #[test]
    fn test_zero_weight() {
        let config = price_discovery_config();
        let investment = InvestmentAmount::default();
        let total_sold_tokens = 1000;

        let res = user_allocation(investment.weight, total_sold_tokens, &config).unwrap();
        assert_eq!(res, 0);
    }

    #[test]
    fn test_zero_total_sold_tokens() {
        let config = price_discovery_config();
        let investment = InvestmentAmount {
            amount: 10,
            weight: 10,
            claimed: 0,
        };
        let total_sold_tokens = 0;

        let res = user_allocation(investment.weight, total_sold_tokens, &config).unwrap();
        assert_eq!(res, 0);
    }

    #[test]
    fn test_zero_amount_eq_weight() {
        let mut config = price_discovery_config();
        config.sale_amount = U128(2 * 10u128.pow(24));
        let investment = InvestmentAmount {
            amount: 10u128.pow(24),
            weight: 10u128.pow(24),
            claimed: 0,
        };
        let total_sold_tokens = 2 * 10u128.pow(24);

        let res = user_allocation(investment.weight, total_sold_tokens, &config).unwrap();
        assert_eq!(res, 10u128.pow(24));
    }

    #[test]
    fn test_zero_amount_weight_125_simple() {
        let mut config = price_discovery_config();
        config.sale_amount = U128(2 * 10u128.pow(24));
        let investment = InvestmentAmount {
            amount: 10u128.pow(24),
            weight: 120 * 10u128.pow(22),
            claimed: 0,
        };
        let total_sold_tokens = 220 * 10u128.pow(22);

        let res = user_allocation(investment.weight, total_sold_tokens, &config).unwrap();
        let expected = 120 * 2 * 10u128.pow(24) / 220;
        assert_eq!(res, expected);
    }

    #[test]
    fn test_sale_amount_has_less_decimals() {
        let mut config = price_discovery_config();
        config.sale_amount = U128(2 * 10u128.pow(18));
        let investment = InvestmentAmount {
            amount: 10u128.pow(18),
            weight: 120 * 10u128.pow(22),
            claimed: 0,
        };
        let total_sold_tokens = 220 * 10u128.pow(22);

        let res = user_allocation(investment.weight, total_sold_tokens, &config).unwrap();
        let expected = 120 * config.sale_amount.0 / 220;
        assert_eq!(res, expected);
    }

    #[test]
    fn test_sale_amount_has_more_decimals() {
        let mut config = price_discovery_config();
        config.sale_amount = U128(2 * 10u128.pow(24));
        let investment = InvestmentAmount {
            amount: 10u128.pow(24),
            weight: 120 * 10u128.pow(16),
            claimed: 0,
        };
        let total_sold_tokens = 220 * 10u128.pow(16);

        let res = user_allocation(investment.weight, total_sold_tokens, &config).unwrap();
        let expected = 120 * config.sale_amount.0 / 220;
        assert_eq!(res, expected);
    }

    #[test]
    fn test_sale_amount_has_more_decimals_less_10() {
        let mut config = price_discovery_config();
        config.sale_amount = 200_000.into();
        let investment = InvestmentAmount {
            amount: 80_000,
            weight: 80_000,
            claimed: 0,
        };
        let total_sold_tokens = 92_000;

        let res = user_allocation(investment.weight, total_sold_tokens, &config).unwrap();
        let expected = 173_913;
        assert_eq!(res, expected);
    }

    #[test]
    fn test_sale_amount_has_more_decimals_greater_1000() {
        let mut config = price_discovery_config();
        config.sale_amount = 200_000.into();
        let investment = InvestmentAmount {
            amount: 80_000,
            weight: 80_000_000,
            claimed: 0,
        };
        let total_sold_tokens = 92_000_000;

        let res = user_allocation(investment.weight, total_sold_tokens, &config).unwrap();
        let expected = 173_913;
        let expected_calc = 80_000_000 * config.sale_amount.0 / total_sold_tokens;
        assert_eq!(res, expected);
        assert_eq!(res, expected_calc);
    }

    #[test]
    fn test_vesting_schedule_inside_cliff_period() {
        let mut config = price_discovery_config();
        let vesting_period = 2_000_000.into();
        config.vesting_schedule = Some(VestingSchedule {
            cliff_period: 500_000.into(),
            vesting_period,
            instant_claim: None,
        });
        let investment = InvestmentAmount {
            amount: 80_000,
            weight: 80_000_000,
            claimed: 0,
        };
        let total_sold_tokens = 92_000_000;
        let current_timestamp = config.end_date + 100_000; // Before cliff period ends

        let res = available_for_claim(&investment, total_sold_tokens, &config, current_timestamp)
            .unwrap();
        let expected = 0;
        assert_eq!(res, expected);
    }

    #[test]
    fn test_vesting_schedule_exactly_after_cliff_period() {
        let mut config = price_discovery_config();
        config.sale_amount = 200_000.into();
        let vesting_period = 2_000_000.into();
        config.vesting_schedule = Some(VestingSchedule {
            cliff_period: 500_000.into(),
            vesting_period,
            instant_claim: None,
        });
        let investment = InvestmentAmount {
            amount: 80_000,
            weight: 80_000_000,
            claimed: 0,
        };
        let total_sold_tokens = 92_000_000;
        let current_timestamp = config.end_date + 500_000;

        let res = available_for_claim(&investment, total_sold_tokens, &config, current_timestamp)
            .unwrap();
        let expected = 43478; // 80_000 * 500_000 / 2_000_000
        let expected_calc = (80_000_000 * config.sale_amount.0 / total_sold_tokens)
            * (u128::from(current_timestamp - config.end_date))
            / u128::from(vesting_period.as_nanos());
        assert_eq!(res, expected);
        assert_eq!(res, expected_calc);
    }

    #[test]
    fn test_vesting_schedule_exactly_halfway_through_vesting_period() {
        let mut config = price_discovery_config();
        config.sale_amount = 200_000.into();
        let vesting_period = 2_000_000.into();
        config.vesting_schedule = Some(VestingSchedule {
            cliff_period: 500_000.into(),
            vesting_period,
            instant_claim: None,
        });
        let investment = InvestmentAmount {
            amount: 80_000,
            weight: 80_000_000,
            claimed: 0,
        };
        let total_sold_tokens = 92_000_000;
        let current_timestamp = config.end_date + vesting_period.as_nanos() / 2; // Halfway through vesting period

        let res = available_for_claim(&investment, total_sold_tokens, &config, current_timestamp)
            .unwrap();
        let expected = 86956;
        let expected_calc = (80_000_000 * config.sale_amount.0 / total_sold_tokens)
            * (u128::from(current_timestamp - config.end_date))
            / u128::from(vesting_period.as_nanos());
        assert_eq!(res, expected);
        assert_eq!(res, expected_calc);
    }

    #[test]
    fn test_vesting_schedule_exactly_at_vesting_period_end() {
        let mut config = price_discovery_config();
        config.sale_amount = 200_000.into();
        let vesting_period = 2_000_000.into();
        config.vesting_schedule = Some(VestingSchedule {
            cliff_period: 500_000.into(),
            vesting_period,
            instant_claim: None,
        });
        let investment = InvestmentAmount {
            amount: 80_000,
            weight: 80_000_000,
            claimed: 0,
        };
        let total_sold_tokens = 92_000_000;
        let current_timestamp = config.end_date + 2_000_000; // Exactly at the end of vesting period

        let res = available_for_claim(&investment, total_sold_tokens, &config, current_timestamp)
            .unwrap();
        let expected = 173_913;
        assert_eq!(res, expected);
    }

    #[test]
    fn test_vesting_schedule_after_vesting_period_end() {
        let mut config = price_discovery_config();
        config.sale_amount = 200_000.into();
        let vesting_period = 2_000_000.into();
        config.vesting_schedule = Some(VestingSchedule {
            cliff_period: 500_000.into(),
            vesting_period,
            instant_claim: None,
        });
        let investment = InvestmentAmount {
            amount: 80_000,
            weight: 80_000_000,
            claimed: 0,
        };
        let total_sold_tokens = 92_000_000;
        let current_timestamp = config.end_date + 2_000_000; // Exactly at the end of vesting period

        let res = available_for_claim(&investment, total_sold_tokens, &config, current_timestamp)
            .unwrap();
        let expected = 173_913;
        assert_eq!(res, expected);
    }

    #[test]
    fn test_vesting_schedule_instant_claim() {
        let mut config = price_discovery_config();
        config.sale_amount = 200_000.into();
        let vesting_period = 2_000_000.into();
        config.vesting_schedule = Some(VestingSchedule {
            cliff_period: 500_000.into(),
            vesting_period,
            instant_claim: Some(1_100), // 11%
        });
        let investment = InvestmentAmount {
            amount: 80_000,
            weight: 80_000_000,
            claimed: 0,
        };
        let total_sold_tokens = 92_000_000;
        let assets_for_claim = 80_000_000 * config.sale_amount.0 / total_sold_tokens;
        // Exactly before cliff period ends
        let cliff_end_timestamp = config.end_date + 500_000 - 1;
        let instant_claim_res =
            available_for_claim(&investment, total_sold_tokens, &config, cliff_end_timestamp)
                .unwrap();
        let expected_calc_instant_claim = assets_for_claim * 11 / 100;
        assert_eq!(instant_claim_res, expected_calc_instant_claim);

        let current_timestamp = config.end_date + vesting_period.as_nanos() / 2; // Halfway through vesting period
        let res = available_for_claim(&investment, total_sold_tokens, &config, current_timestamp)
            .unwrap();
        let expected = 86956;
        let expected_calc = assets_for_claim * (u128::from(current_timestamp - config.end_date))
            / u128::from(vesting_period.as_nanos());
        assert_eq!(res, expected);
        assert_eq!(res, expected_calc);
    }

    #[test]
    fn test_individual_vesting_schedule_inside_cliff_period() {
        let config = price_discovery_config();
        let vesting_period = 2_000_000.into();
        let vesting_schedule = Some(VestingSchedule {
            cliff_period: 500_000.into(),
            vesting_period,
            instant_claim: None,
        });
        let allocation = 80_000_000;
        let current_timestamp = config.end_date + 100_000; // Before cliff period ends

        let res = available_for_individual_vesting_claim(
            allocation,
            vesting_schedule.as_ref(),
            config.end_date,
            current_timestamp,
        )
        .unwrap();
        let expected = 0;
        assert_eq!(res, expected);
    }

    #[test]
    fn test_individual_vesting_schedule_exactly_after_cliff_period() {
        let mut config = price_discovery_config();
        config.sale_amount = 200_000.into();
        let vesting_period = 2_000_000.into();
        let vesting_schedule = Some(VestingSchedule {
            cliff_period: 500_000.into(),
            vesting_period,
            instant_claim: None,
        });

        let allocation = 80_000_000;
        let current_timestamp = config.end_date + 500_000;

        let res = available_for_individual_vesting_claim(
            allocation,
            vesting_schedule.as_ref(),
            config.end_date,
            current_timestamp,
        )
        .unwrap();
        let expected = 20_000_000;
        let expected_calc = allocation * (u128::from(current_timestamp - config.end_date))
            / u128::from(vesting_period.as_nanos());
        assert_eq!(res, expected);
        assert_eq!(res, expected_calc);
    }

    #[test]
    fn test_individual_vesting_schedule_exactly_halfway_through_vesting_period() {
        let mut config = price_discovery_config();
        config.sale_amount = 200_000.into();
        let vesting_period = 2_000_000.into();
        let vesting_schedule = Some(VestingSchedule {
            cliff_period: 500_000.into(),
            vesting_period,
            instant_claim: None,
        });

        let allocation = 80_000_000;
        let current_timestamp = config.end_date + vesting_period.as_nanos() / 2; // Halfway through vesting period

        let res = available_for_individual_vesting_claim(
            allocation,
            vesting_schedule.as_ref(),
            config.end_date,
            current_timestamp,
        )
        .unwrap();
        let expected = 40_000_000;
        let expected_calc = allocation * (u128::from(current_timestamp - config.end_date))
            / u128::from(vesting_period.as_nanos());
        assert_eq!(res, expected);
        assert_eq!(res, expected_calc);
    }

    #[test]
    fn test_individual_vesting_schedule_exactly_at_vesting_period_end() {
        let mut config = price_discovery_config();
        config.sale_amount = 200_000.into();
        let vesting_period = 2_000_000.into();
        let vesting_schedule = Some(VestingSchedule {
            cliff_period: 500_000.into(),
            vesting_period,
            instant_claim: None,
        });

        let allocation = 80_000_000;
        let current_timestamp = config.end_date + 2_000_000; // Exactly at the end of vesting period

        let res = available_for_individual_vesting_claim(
            allocation,
            vesting_schedule.as_ref(),
            config.end_date,
            current_timestamp,
        )
        .unwrap();
        assert_eq!(res, allocation);
    }

    #[test]
    fn test_individual_vesting_schedule_after_vesting_period_end() {
        let mut config = price_discovery_config();
        config.sale_amount = 200_000.into();
        let vesting_period = 2_000_000.into();
        let vesting_schedule = Some(VestingSchedule {
            cliff_period: 500_000.into(),
            vesting_period,
            instant_claim: None,
        });

        let allocation = 80_000_000;
        let current_timestamp = config.end_date + 2_000_000; // Exactly at the end of vesting period

        let res = available_for_individual_vesting_claim(
            allocation,
            vesting_schedule.as_ref(),
            config.end_date,
            current_timestamp,
        )
        .unwrap();
        assert_eq!(res, allocation);
    }
}
