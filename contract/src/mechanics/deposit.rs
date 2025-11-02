use alloy_primitives::ruint::aliases::U256;
use aurora_launchpad_types::InvestmentAmount;
use aurora_launchpad_types::config::{LaunchpadConfig, Mechanics};
use aurora_launchpad_types::discount::DepositDistribution;
use aurora_launchpad_types::utils::to_u128;

/// Deposits an amount into the investment, applying the current discount if available.
/// 1. For `FixedPrice`, the weight is calculated based on the price and current discount.
///    If the total sold tokens exceed the total sale amount, it adjusts the investment and returns
///    the excess amount.
/// 2. For `PriceDiscovery`, the weight is calculated based on the current discount.
pub fn deposit(
    investment: &mut InvestmentAmount,
    amount: u128,
    total_deposited: &mut u128,
    total_sold_tokens: &mut u128,
    config: &LaunchpadConfig,
    deposit_distribution: &DepositDistribution,
) -> Result<u128, &'static str> {
    // Calculate the weight based on the deposit distribution.
    let (weight, refund) = match deposit_distribution {
        DepositDistribution::WithDiscount {
            phase_weights,
            public_sale_weight,
            refund,
        } => (
            DepositDistribution::discount_weight_sum(phase_weights, *public_sale_weight),
            *refund,
        ),
        DepositDistribution::WithoutDiscount(weight) => {
            if let Mechanics::FixedPrice {
                deposit_token,
                sale_token,
            } = config.mechanics
            {
                let assets =
                    calculate_amount_of_sale_tokens(*weight, deposit_token.0, sale_token.0)?;
                let exceed = total_sold_tokens
                    .saturating_add(assets)
                    .saturating_sub(config.sale_amount.0);

                if exceed > 0 {
                    let refund =
                        calculate_weight_from_sale_tokens(exceed, deposit_token.0, sale_token.0)?;
                    (weight.saturating_sub(refund), refund)
                } else {
                    (*weight, 0)
                }
            } else {
                (*weight, 0)
            }
        }
        DepositDistribution::Refund(refund) => return Ok(*refund),
    };

    let deposit = amount.saturating_sub(refund);
    // For a fixed price, we consider weight as the number of sale tokens purchased for the deposit,
    // plus any discount. For price discovery, we should consider the weight as a deposit,
    // plus any discount if available, since we can't calculate the exact number of sale tokens
    // before the sale finishes.
    let weight = if let Mechanics::FixedPrice {
        deposit_token,
        sale_token,
    } = config.mechanics
    {
        calculate_amount_of_sale_tokens(weight, deposit_token.0, sale_token.0)?
    } else {
        weight
    };

    investment.amount = investment.amount.saturating_add(deposit);
    investment.weight = investment.weight.saturating_add(weight);

    *total_deposited = total_deposited.saturating_add(deposit);
    *total_sold_tokens = total_sold_tokens.saturating_add(weight);

    Ok(refund)
}

/// Calculates the number of sale tokens based on the amount of deposit and price fraction.
pub fn calculate_amount_of_sale_tokens(
    amount: u128,
    deposit_token: u128,
    sale_token: u128,
) -> Result<u128, &'static str> {
    U256::from(amount)
        .checked_mul(U256::from(sale_token))
        .ok_or("Multiplication overflow")
        .map(|result| result / U256::from(deposit_token))
        .and_then(to_u128)
}

/// Calculates the deposit(weight) based on the amount of sale tokens and price fraction.
pub fn calculate_weight_from_sale_tokens(
    amount: u128,
    deposit_token: u128,
    sale_token: u128,
) -> Result<u128, &'static str> {
    U256::from(amount)
        .checked_mul(U256::from(deposit_token))
        .ok_or("Multiplication overflow")
        .map(|result| result / U256::from(sale_token))
        .and_then(to_u128)
}

#[cfg(test)]
mod tests {
    use crate::mechanics::claim::available_for_claim;
    use crate::mechanics::deposit::deposit;
    use crate::tests::utils::{NOW, fixed_price_config, price_discovery_config};
    use aurora_launchpad_types::InvestmentAmount;
    use aurora_launchpad_types::discount::DepositDistribution;

    #[test]
    fn test_deposit_price_discovery_no_discount() {
        let config = price_discovery_config();
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 10u128.pow(29); // 100k tokens
        let deposit_distribution = DepositDistribution::WithoutDiscount(deposit_amount);

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &deposit_distribution,
        );

        let expected_weight = deposit_amount;
        assert_eq!(result, Ok(0));
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, expected_weight);
        assert_eq!(total_deposited, deposit_amount);
        assert_eq!(total_sold_tokens, expected_weight);

        // Check claim with fake `total_tokens_sold`
        total_sold_tokens *= 3;
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, config.sale_amount.0 / 3);
    }

    #[test]
    fn test_deposit_price_discovery_with_discount() {
        let config = price_discovery_config();
        let deposit_amount = 10u128.pow(29); // 100k tokens
        let expected_weight = deposit_amount * 120 / 100; // 120k tokens
        let deposit_distribution = DepositDistribution::WithDiscount {
            phase_weights: vec![(1, expected_weight)],
            public_sale_weight: 0,
            refund: 0,
        };
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &deposit_distribution,
        );

        assert_eq!(result, Ok(0));
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, expected_weight);
        assert_eq!(total_deposited, deposit_amount);
        assert_eq!(total_sold_tokens, expected_weight);

        // Check claim with fake `total_tokens_sold`
        total_sold_tokens += 2 * deposit_amount;
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, 1125 * deposit_amount / 100_000_000);
    }

    #[test]
    fn test_deposit_fixed_price_no_discount_simple() {
        let config = fixed_price_config();
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 10u128.pow(29); // 100k tokens
        let deposit_distribution = DepositDistribution::WithoutDiscount(deposit_amount);

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &deposit_distribution,
        );

        let expected_weight = 10u128.pow(18) * 20 * 100_000;
        assert_eq!(result, Ok(0));
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, expected_weight);
        assert_eq!(total_deposited, deposit_amount);
        assert_eq!(total_sold_tokens, expected_weight);

        // Check claim
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, expected_weight);
    }

    #[test]
    fn test_deposit_fixed_price_with_discount() {
        let config = fixed_price_config();
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 10u128.pow(29); // 100k tokens
        let expected_weight = deposit_amount * 125 / 100;
        let deposit_distribution = DepositDistribution::WithDiscount {
            phase_weights: vec![(1, expected_weight)],
            public_sale_weight: 0,
            refund: 0,
        };

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &deposit_distribution,
        );

        let expected_assets = 10u128.pow(18) * 20 * 100_000 * 125 / 100;
        assert_eq!(result, Ok(0));
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, expected_assets);
        assert_eq!(total_deposited, deposit_amount);
        assert_eq!(total_sold_tokens, expected_assets);

        // Check claim
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, expected_assets);
    }

    #[test]
    fn fixed_price_reached_sale_amount_no_discount() {
        let config = fixed_price_config();
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 2 * 10u128.pow(29); // 200k tokens
        let deposit_distribution = DepositDistribution::WithoutDiscount(deposit_amount);

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &deposit_distribution,
        )
        .unwrap();

        let expected_assets = config.sale_amount;
        assert_eq!(result, 5 * 10u128.pow(28)); // 50k tokens
        assert_eq!(investment.amount, deposit_amount - result);
        assert_eq!(investment.weight, expected_assets.0);
        assert_eq!(total_deposited, deposit_amount - result);
        assert_eq!(total_sold_tokens, expected_assets.0);

        // Check claim
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, expected_assets.0);
    }

    #[test]
    fn test_deposit_fixed_price_reached_sale_amount_with_discount() {
        let config = fixed_price_config();
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 2 * 10u128.pow(29); // 200k tokens
        // 2 Million tokens - with discount 25%: price * deposit * discount - sale_amount
        let assets_exceeded =
            (20 * deposit_amount * 125 / 100) - 10u128.pow(6) * config.sale_amount.0;
        let expected_weight = config.sale_amount.0 * 50000;
        // Exclude discount from assets excess and dived to token price 20
        let expected_refund = (assets_exceeded * 100 / 125) / 20;
        let deposit_distribution = DepositDistribution::WithDiscount {
            phase_weights: vec![(1, expected_weight)],
            public_sale_weight: 0,
            refund: expected_refund,
        };

        let refund = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &deposit_distribution,
        )
        .unwrap();

        let expected_assets = config.sale_amount;
        assert_eq!(refund, expected_refund);

        assert_eq!(refund, 8 * 10u128.pow(28)); // 80k tokens
        assert_eq!(investment.amount, deposit_amount - refund);
        assert_eq!(investment.weight, expected_assets.0);
        assert_eq!(total_deposited, deposit_amount - refund);
        assert_eq!(total_sold_tokens, expected_assets.0);

        // Check claim
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, expected_assets.0);
    }

    #[test]
    fn test_deposit_fixed_price_sale_exactly_at_sale_amount() {
        let config = fixed_price_config();
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 15 * 10u128.pow(28); // 150k tokens
        let deposit_distribution = DepositDistribution::WithoutDiscount(deposit_amount);

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            &deposit_distribution,
        );

        let expected_assets = config.sale_amount.0;
        assert_eq!(result, Ok(0));
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, expected_assets);
        assert_eq!(total_deposited, deposit_amount);
        assert_eq!(total_sold_tokens, expected_assets);

        // Check claim
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, expected_assets);
    }
}

#[cfg(test)]
mod tests_calculate_assets {
    use super::*;
    use alloy_primitives::ruint::aliases::U256;

    const DECIMALS: u32 = 24;
    const TOKEN_SCALE: u128 = 10u128.pow(DECIMALS);

    #[test]
    fn test_normal_case() {
        let amount = 10 * TOKEN_SCALE;
        let deposit_token = 10;
        let sale_token = 5;

        let result = calculate_amount_of_sale_tokens(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 5 * TOKEN_SCALE);
    }

    #[test]
    fn test_small_fraction_result() {
        let amount = 1;
        let deposit_token = 10u128.pow(24);
        let sale_token = 1;

        let result = calculate_amount_of_sale_tokens(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_price_is_one_token_scale() {
        let amount = 42;
        let deposit_token = 1;
        let sale_token = 1;

        let result = calculate_amount_of_sale_tokens(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_multiplication_overflow() {
        let amount = (u128::MAX / 10u128.pow(24)) + 1;
        let deposit_token = 1;
        let sale_token = 10u128.pow(24);

        let result = calculate_amount_of_sale_tokens(amount, deposit_token, sale_token);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_when_price_is_less_then_amount() {
        let amount = 10 * 10u128.pow(24);
        let deposit_token = 31;
        let sale_token = 10u128.pow(4);

        let result = calculate_amount_of_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = U256::from(amount) * U256::from(sale_token) / U256::from(deposit_token);
        assert_eq!(result, 3_225_806_451_612_903_225_806_451_612);
        assert_eq!(result, to_u128(expected).unwrap());
    }

    #[test]
    fn test_when_decimals_24_18() {
        let amount = 10 * 10u128.pow(24);
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_amount_of_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = 10u128.pow(19) / 3;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18_low_amount() {
        let amount = 10 * 10u128.pow(6);
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_amount_of_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = 10u128.pow(1) / 3;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18_too_small_amount() {
        let amount = 10 * 10u128.pow(5);
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_amount_of_sale_tokens(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_when_decimals_18_24() {
        let amount = 10 * 10u128.pow(18);
        let deposit_token = 3;
        let sale_token = 10u128.pow(6);

        let result = calculate_amount_of_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = 10u128.pow(25) / 3;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_18_24_low_amount() {
        let amount = 10;
        let deposit_token = 3;
        let sale_token = 10u128.pow(6);

        let result = calculate_amount_of_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = 10u128.pow(7) / 3;
        assert_eq!(result, expected);
    }
}

#[cfg(test)]
mod tests_calculate_assets_revert {
    use super::*;

    #[test]
    fn test_normal_case() {
        let amount = 5;
        let deposit_token = 2;
        let sale_token = 1;

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_price_less_than_token_scale() {
        let amount = 5;
        let deposit_token = 1;
        let sale_token = 2;

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_zero_amount() {
        let amount = 0;
        let deposit_token = 2;
        let sale_token = 1;

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_division_truncates_fraction() {
        let amount = 1;
        let deposit_token = 10u128.pow(24) + 10u128.pow(24) / 2;
        let sale_token = 10u128.pow(24);

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 1); // floor(1.5)
    }

    #[test]
    fn test_multiplication_overflow_should_fail() {
        let amount = u128::MAX / 2 + 1;
        let deposit_token = 2;
        let sale_token = 1;

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_large_valid_multiplication_no_overflow() {
        // Should be OK just under an overflow threshold
        let amount = u128::MAX / 2;
        let deposit_token = 2;
        let sale_token = 1;

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token);
        assert!(result.is_ok());
    }

    #[test]
    fn test_when_price_is_less_then_amount() {
        let amount = 10u128.pow(25);
        let deposit_token = 31;
        let sale_token = 10u128.pow(6);

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = amount * 31 / 10u128.pow(6);
        assert_eq!(result, 10 * 31 * 10u128.pow(18));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18() {
        let amount = 10 * 10u128.pow(24);
        let deposit_token = 3;
        let sale_token = 10u128.pow(6);

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = 3 * 10u128.pow(19);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_18_24() {
        let amount = 10 * 10u128.pow(18);
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = 3 * 10u128.pow(25);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_18_24_revert() {
        let amount = 10u128.pow(19) / 3;
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = (10u128.pow(19) - 1) * 10u128.pow(6);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_18_24_low_amount_revert() {
        let amount = 10u128.pow(1) / 3;
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = 9 * 10u128.pow(6);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18_revert() {
        let amount = 10u128.pow(25) / 3;
        let deposit_token = 3;
        let sale_token = 10u128.pow(6);

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = 10 * 10u128.pow(18) - 1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18_low_amount_revert() {
        let amount = 10u128.pow(7) / 3;
        let deposit_token = 3;
        let sale_token = 10u128.pow(6);

        let result = calculate_weight_from_sale_tokens(amount, deposit_token, sale_token).unwrap();
        let expected = 10 - 1;
        assert_eq!(result, expected);
    }
}
