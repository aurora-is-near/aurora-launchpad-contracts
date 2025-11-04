use aurora_launchpad_types::InvestmentAmount;
use aurora_launchpad_types::config::{LaunchpadConfig, Mechanics};
use aurora_launchpad_types::discount::DepositDistribution;
use near_sdk::require;

/// Withdraw state modification, adjusting the weight and discount if adjusted.
pub fn withdraw(
    investment: &mut InvestmentAmount,
    amount: u128,
    total_deposited: &mut u128,
    total_sold_tokens: &mut u128,
    config: &LaunchpadConfig,
    deposit_distribution: &DepositDistribution,
) -> Result<(), &'static str> {
    match config.mechanics {
        Mechanics::FixedPrice { .. } => {
            if amount != investment.amount {
                return Err("Wrong FixedPrice amount to withdraw");
            }
            investment.amount = 0;
            *total_sold_tokens = total_sold_tokens.saturating_sub(investment.weight);
            // Reset weight to zero, as we are withdrawing all funds
            investment.weight = 0;
        }
        Mechanics::PriceDiscovery => {
            if amount > investment.amount {
                return Err("Not enough funds to withdraw");
            }

            // Decrease user investment amount
            investment.amount = investment.amount.saturating_sub(amount);

            let weight = investment.weight;
            // Recalculate the weight according to the current discount distribution
            investment.weight = recalculate_weight(deposit_distribution);
            // Recalculate the total sold tokens
            if weight > investment.weight {
                // If the discount decreased
                *total_sold_tokens = total_sold_tokens.saturating_sub(weight - investment.weight);
            } else {
                // If the discount was increased - we don't change the user weight and `total_sold_tokens`
                investment.weight = weight;
            }
        }
    }

    // Decrease the total investment amount
    *total_deposited = total_deposited.saturating_sub(amount);

    Ok(())
}

fn recalculate_weight(deposit_distribution: &DepositDistribution) -> u128 {
    match deposit_distribution {
        DepositDistribution::WithDiscount {
            phase_weights,
            public_sale_weight,
            refund,
        } => {
            require!(
                *refund == 0,
                "Refund in withdrawal with mechanic PriceDiscovery is not supported"
            );
            phase_weights
                .iter()
                .map(|(_, weight)| *weight)
                .sum::<u128>()
                .saturating_add(*public_sale_weight)
        }
        DepositDistribution::WithoutDiscount(weight) => *weight,
        DepositDistribution::Refund(_) => {
            near_sdk::env::panic_str("Refund in withdrawal is not supported")
        }
    }
}

#[cfg(test)]
mod tests {
    use aurora_launchpad_types::InvestmentAmount;
    use aurora_launchpad_types::discount::DepositDistribution;

    use crate::mechanics::claim::available_for_claim;
    use crate::mechanics::withdraw::withdraw;
    use crate::tests::utils::{NOW, fixed_price_config, price_discovery_config};

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

        let result = withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &DepositDistribution::WithoutDiscount(deposit_amount - withdraw_amount),
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

        let result = withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &DepositDistribution::WithoutDiscount(deposit_amount - withdraw_amount),
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
        let expected_weight = deposit_amount - withdraw_amount; // no discount while withdrawing
        let deposit_distribution = DepositDistribution::WithoutDiscount(expected_weight);

        let result = withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &deposit_distribution,
        );

        let expected_deposit = deposit_amount - withdraw_amount;
        // Discount was lost and now equal to the withdrawal amount
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
        let config = price_discovery_config();
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
        let expected_weight = (deposit_amount - withdraw_amount) * 125 / 100;
        let deposit_distribution = DepositDistribution::WithDiscount {
            phase_weights: vec![(1, expected_weight)],
            public_sale_weight: 0,
            refund: 0,
        };

        let result = withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &deposit_distribution,
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
        let config = price_discovery_config();
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
        let expected_weight = (deposit_amount - withdraw_amount) * 110 / 100;
        let deposit_distribution = DepositDistribution::WithDiscount {
            phase_weights: vec![(1, expected_weight)],
            public_sale_weight: 0,
            refund: 0,
        };

        let result = withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &deposit_distribution,
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
        // NOTE: This test case is unusual and in common sense unexpected when a discount is increased
        let config = price_discovery_config();
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
        let expected_weight = (deposit_amount - withdraw_amount) * 170 / 100;
        let deposit_distribution = DepositDistribution::WithDiscount {
            phase_weights: vec![(1, expected_weight)],
            public_sale_weight: 0,
            refund: 0,
        };

        let result = withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &deposit_distribution,
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
