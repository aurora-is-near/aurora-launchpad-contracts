use aurora_launchpad_types::InvestmentAmount;
use aurora_launchpad_types::config::{LaunchpadConfig, Mechanics};
use aurora_launchpad_types::discount::Discount;

/// Post withdraw state modification, adjusting the weight and discount if adjusted.
pub fn post_withdraw(
    investment: &mut InvestmentAmount,
    amount: u128,
    total_deposited: &mut u128,
    total_sold_tokens: &mut u128,
    config: &LaunchpadConfig,
    timestamp: u64,
) -> Result<(), &'static str> {
    match config.mechanics {
        Mechanics::FixedPrice { .. } => {
            if amount != investment.amount {
                return Err("Wrong FixedPrice amount to withdraw");
            }
            investment.amount = 0;
            // Reset weight to zero, as we are withdrawing all funds
            investment.weight = 0;
        }
        Mechanics::PriceDiscovery => {
            // Decrease user investment amount
            investment.amount = investment.amount.saturating_sub(amount);

            let weight = investment.weight;
            // If discount is applied, we need to adjust the weight accordingly
            if investment.weight != investment.amount {
                // Recalculate the weight according to the current discount
                investment.weight = Discount::get_weight(config, investment.amount, timestamp)?;
            }
            // Recalculate the total sold tokens
            if weight >= investment.weight {
                // If discount decreased
                *total_sold_tokens = total_sold_tokens.saturating_sub(weight - investment.weight);
            } else {
                // If the discount was increased - we don't change the user weight and `total_sold_tokens`
                investment.weight = weight;
            }
        }
    }

    // Decrease total investment amount
    *total_deposited = total_deposited.saturating_sub(amount);

    Ok(())
}

pub const fn validate_amount(
    investment: &InvestmentAmount,
    amount: u128,
    config: &LaunchpadConfig,
) -> Result<(), &'static str> {
    match config.mechanics {
        Mechanics::FixedPrice { .. } => {
            if amount != investment.amount {
                return Err("Partial withdrawal is allowed only in Price Discovery");
            }
        }
        Mechanics::PriceDiscovery => {
            if amount > investment.amount {
                return Err("Not enough funds to withdraw");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::mechanics::claim::available_for_claim;
    use crate::mechanics::withdraw::{post_withdraw, validate_amount};
    use crate::tests::utils::{NOW, TEN_DAYS, fixed_price_config, price_discovery_config};
    use aurora_launchpad_types::InvestmentAmount;
    use aurora_launchpad_types::discount::Discount;

    #[test]
    fn test_validate_amount_fixed_price() {
        let config = fixed_price_config();
        let investment = InvestmentAmount {
            amount: 2 * 10u128.pow(25),
            weight: 2 * 10u128.pow(25),
            claimed: 0,
        };
        let withdraw_amount = 3 * 10u128.pow(24);

        let result = validate_amount(&investment, withdraw_amount, &config);

        assert_eq!(
            result.unwrap_err(),
            "Partial withdrawal is allowed only in Price Discovery"
        );
    }

    #[test]
    fn test_withdraw_fixed_price() {
        let config = fixed_price_config();
        let deposit_amount = 2 * 10u128.pow(25);
        let weight_amount = deposit_amount;
        let mut investment = InvestmentAmount {
            amount: deposit_amount,
            weight: weight_amount,
            claimed: 0,
        };
        let mut total_deposited = deposit_amount;
        let mut total_sold_tokens = weight_amount;
        let withdraw_amount = 3 * 10u128.pow(24);

        let result = post_withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
        );

        let expected_deposit = deposit_amount;
        let expected_weight = weight_amount;
        assert_eq!(result.unwrap_err(), "Wrong FixedPrice amount to withdraw");
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, weight_amount);
        assert_eq!(total_deposited, expected_deposit);
        assert_eq!(total_sold_tokens, expected_weight);

        // Check claim with fake `total_tokens_sold`
        total_sold_tokens *= 3;
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, weight_amount);
    }

    #[test]
    fn test_withdraw_price_discovery_large_amount() {
        let config = price_discovery_config();
        let deposit_amount = 2 * 10u128.pow(25);
        let weight_amount = 2 * 10u128.pow(25);
        let investment = InvestmentAmount {
            amount: deposit_amount,
            weight: weight_amount,
            claimed: 0,
        };
        let withdraw_amount = 2 * 10u128.pow(25) + 1;

        let result = validate_amount(&investment, withdraw_amount, &config);
        assert_eq!(result.unwrap_err(), "Not enough funds to withdraw");
    }

    #[test]
    fn test_withdraw_price_discovery_no_discount() {
        let config = price_discovery_config();
        let deposit_amount = 2 * 10u128.pow(25);
        let weight_amount = deposit_amount;
        let mut investment = InvestmentAmount {
            amount: deposit_amount,
            weight: weight_amount,
            claimed: 0,
        };
        let mut total_deposited = deposit_amount;
        let mut total_sold_tokens = weight_amount;
        let withdraw_amount = 3 * 10u128.pow(24);

        let result = post_withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
        );

        let expected_deposit = deposit_amount - withdraw_amount;
        let expected_weight = weight_amount - withdraw_amount;
        assert!(result.is_ok());
        assert_eq!(investment.amount, expected_deposit);
        assert_eq!(investment.weight, expected_weight);
        assert_eq!(total_deposited, expected_deposit);
        assert_eq!(total_sold_tokens, expected_weight);

        // Check claim with fake `total_tokens_sold`
        total_sold_tokens *= 3;
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, config.sale_amount.0 / 3);
    }

    #[test]
    fn test_withdraw_price_discovery_no_discount_for_deposit_with_discount() {
        let config = price_discovery_config();
        let deposit_amount = 2 * 10u128.pow(25);
        // Weight with discount
        let weight_amount = deposit_amount * 125 / 100;
        let mut investment = InvestmentAmount {
            amount: deposit_amount,
            weight: weight_amount,
            claimed: 0,
        };
        let mut total_deposited = deposit_amount;
        let mut total_sold_tokens = weight_amount;
        let withdraw_amount = 3 * 10u128.pow(24);

        let result = post_withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
        );

        let expected_deposit = deposit_amount - withdraw_amount;
        // Discount was lost and now equal to withdraw amount
        let expected_weight = deposit_amount - withdraw_amount;
        assert!(result.is_ok());
        assert_eq!(investment.amount, expected_deposit);
        assert_eq!(investment.weight, expected_weight);
        assert_eq!(total_deposited, expected_deposit);
        assert_eq!(total_sold_tokens, expected_weight);

        // Check claim with fake `total_tokens_sold`
        total_sold_tokens *= 3;
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, config.sale_amount.0 / 3);
    }

    #[test]
    fn test_withdraw_price_discovery_with_normal_discount() {
        let mut config = price_discovery_config();
        config.discounts.push(Discount {
            start_date: NOW,
            end_date: NOW + TEN_DAYS,
            percentage: 2500, // 25%
        });
        let deposit_amount = 2 * 10u128.pow(25);
        // Weight with discount 25%
        let weight_amount = deposit_amount * 125 / 100;
        let mut investment = InvestmentAmount {
            amount: deposit_amount,
            weight: weight_amount,
            claimed: 0,
        };
        let mut total_deposited = deposit_amount;
        let mut total_sold_tokens = weight_amount;
        let withdraw_amount = 3 * 10u128.pow(24);

        let result = post_withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
        );

        assert!(result.is_ok());
        let expected_deposit = deposit_amount - withdraw_amount;
        // Same discount
        let expected_weight = weight_amount - withdraw_amount * 125 / 100;
        assert!(result.is_ok());
        assert_eq!(investment.amount, expected_deposit);
        assert_eq!(investment.weight, expected_weight);
        assert_eq!(total_deposited, expected_deposit);
        assert_eq!(total_sold_tokens, expected_weight);

        // Check claim with fake `total_tokens_sold`
        total_sold_tokens *= 3;
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, config.sale_amount.0 / 3);
    }

    #[test]
    fn test_withdraw_price_discovery_with_less_discount() {
        let mut config = price_discovery_config();
        config.discounts.push(Discount {
            start_date: NOW,
            end_date: NOW + TEN_DAYS,
            percentage: 1000, // 10%
        });
        let deposit_amount = 2 * 10u128.pow(25);
        // Weight with discount 25%
        let weight_amount = deposit_amount * 125 / 100;
        let mut investment = InvestmentAmount {
            amount: deposit_amount,
            weight: weight_amount,
            claimed: 0,
        };
        let mut total_deposited = deposit_amount;
        let mut total_sold_tokens = weight_amount;
        let withdraw_amount = 3 * 10u128.pow(24);

        let result = post_withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
        );

        assert!(result.is_ok());
        let expected_deposit = deposit_amount - withdraw_amount;
        // Same discount
        let expected_weight = (deposit_amount - withdraw_amount) * 110 / 100;
        assert!(result.is_ok());
        let expected_weight_from_amount = investment.amount * 110 / 100;
        assert_eq!(investment.amount, expected_deposit);
        assert_eq!(investment.weight, expected_weight);
        assert_eq!(investment.weight, expected_weight_from_amount);
        assert_eq!(total_deposited, expected_deposit);
        assert_eq!(total_sold_tokens, expected_weight);

        // Check claim with fake `total_tokens_sold`
        total_sold_tokens *= 3;
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, config.sale_amount.0 / 3);
    }

    #[test]
    fn test_withdraw_price_discovery_with_greater_discount() {
        // NOTE: this test case is unusual and in common sense unexpected  when discount increased.
        // When discount increased
        let mut config = price_discovery_config();
        config.discounts.push(Discount {
            start_date: NOW,
            end_date: NOW + TEN_DAYS,
            percentage: 7000, // 70%
        });
        let deposit_amount = 2 * 10u128.pow(25);
        // Weight with discount 25%
        let weight_amount = deposit_amount * 125 / 100;
        let mut investment = InvestmentAmount {
            amount: deposit_amount,
            weight: weight_amount,
            claimed: 0,
        };
        let mut total_deposited = deposit_amount;
        let mut total_sold_tokens = weight_amount;
        let withdraw_amount = 3 * 10u128.pow(24);

        let result = post_withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
        );

        assert!(result.is_ok());
        let expected_deposit = deposit_amount - withdraw_amount;
        // Same weight for increase discount
        let expected_weight = weight_amount;
        assert!(result.is_ok());
        assert_eq!(investment.amount, expected_deposit);
        assert_eq!(investment.weight, expected_weight);
        assert_eq!(total_deposited, expected_deposit);
        assert_eq!(total_sold_tokens, expected_weight);

        // Check claim with fake `total_tokens_sold`
        total_sold_tokens *= 3;
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, config.sale_amount.0 / 3);
    }
}
