use alloy_primitives::ruint::aliases::U256;
use aurora_launchpad_types::InvestmentAmount;
use aurora_launchpad_types::config::{LaunchpadConfig, Mechanics};

/// Deposits an amount into the investment, applying the current discount if available.
/// 1. For `FixedPrice`, the weight is calculated based on the price and current discount.
/// If the total sold tokens exceed the total sale amount, it adjusts the investment and returns
/// the excess amount.
/// 2. For Price Discovery, the weight is calculated based on the current discount.
pub fn deposit(
    investment: &mut InvestmentAmount,
    amount: u128,
    total_deposited: &mut u128,
    total_sold_tokens: &mut u128,
    config: &LaunchpadConfig,
    timestamp: u64,
) -> Result<u128, &'static str> {
    // Get the current discount based on the timestamp
    let discount = config.get_current_discount(timestamp);
    // Calculate the weight based on the discount
    let weight = match discount {
        Some(disc) => {
            amount
                .checked_mul(u128::from(disc.percentage))
                .ok_or("Multiplication overflow")?
                / 100
        }
        None => amount,
    };
    investment.amount = investment.amount.saturating_add(amount);
    *total_deposited = total_deposited.saturating_add(amount);

    // For fixed price mechanics, we need to calculate the assets based on the weight and price
    // and validate a total sale amount.
    if let Mechanics::FixedPrice {
        deposit_token,
        sale_token,
    } = config.mechanics
    {
        // Calculate the assets based on the weight and price
        // We use U256 to handle large numbers and avoid overflow
        let assets = calculate_assets(weight, deposit_token.0, sale_token.0)?;
        investment.weight = investment.weight.saturating_add(assets);
        *total_sold_tokens = total_sold_tokens.saturating_add(assets);

        // Check if the total sold tokens exceed the sale amount
        if *total_sold_tokens > config.sale_amount.0 {
            // Recalculate the excess assets based on token price
            let assets_excess = *total_sold_tokens - config.sale_amount.0;
            // Calculate how much to revert from the investment
            let remain = calculate_assets_revert(deposit_token.0, assets_excess, sale_token.0)?;

            // Remain recalculation logic based on the discount
            let remain = match discount {
                Some(disc) => {
                    remain.checked_mul(100).ok_or("Multiplication overflow")?
                        / u128::from(disc.percentage)
                }
                None => remain,
            };

            investment.amount = investment.amount.saturating_sub(remain);
            investment.weight = investment.weight.saturating_sub(assets_excess);
            *total_deposited = total_deposited.saturating_sub(remain);
            *total_sold_tokens = total_sold_tokens.saturating_sub(assets_excess);

            return Ok(remain);
        }
    } else {
        investment.weight = investment.weight.saturating_add(weight);
        *total_sold_tokens = total_sold_tokens.saturating_add(weight);
    }

    Ok(0)
}

/// Withdraws an amount from the investment, adjusting the weight and discount if adjusted.
/// Applicable only for Price Discovery mechanics.
pub fn withdraw(
    investment: &mut InvestmentAmount,
    amount: u128,
    total_deposited: &mut u128,
    total_sold_tokens: &mut u128,
    config: &LaunchpadConfig,
    timestamp: u64,
) -> Result<(), &'static str> {
    if !matches!(config.mechanics, Mechanics::PriceDiscovery) {
        return Err("Partial withdrawal is allowed only in Price Discovery");
    }

    if amount > investment.amount {
        return Err("Insufficient funds to withdraw");
    }

    // Decrease user investment amount
    investment.amount = investment.amount.saturating_sub(amount);
    // Decrease total investment amount
    *total_deposited = total_deposited.saturating_sub(amount);

    let weight = investment.weight;
    // If discount is applied, we need to adjust the weight accordingly
    if investment.weight != investment.amount {
        // Recalculate the weight according to the current discount
        if let Some(current_discount) = config.get_current_discount(timestamp) {
            investment.weight = investment
                .amount
                .checked_mul(u128::from(current_discount.percentage))
                .ok_or("Discount multiplication overflow")?
                / 100;
        } else {
            // If no discount is applied, the weight is simply the amount
            // And it means we delete the whole discounts from the investment
            investment.weight = investment.amount;
        }
    }
    // Recalculate the total sold tokens
    if weight >= investment.weight {
        // If discount decreased
        *total_sold_tokens = total_sold_tokens.saturating_sub(weight - investment.weight);
    } else {
        // If discount increased - we don't changed user weight and `total_sold_tokens`
        investment.weight = weight;
    };

    Ok(())
}

/// Calculates the available assets for claim based on the mechanics and vesting schedule.
pub fn available_for_claim(
    investment: &InvestmentAmount,
    total_sold_tokens: u128,
    config: &LaunchpadConfig,
    timestamp: u64,
) -> Result<u128, &'static str> {
    if let Some(vesting) = &config.vesting_schedule {
        if vesting.cliff_period > timestamp {
            return Ok(0);
        }
        todo!("Implement vesting schedule logic for P2 phase");
    }
    match config.mechanics {
        Mechanics::FixedPrice { .. } => Ok(investment.weight),
        Mechanics::PriceDiscovery => {
            if investment.weight == 0 || total_sold_tokens == 0 {
                return Ok(0);
            }
            let weight_log10 = total_sold_tokens.ilog10();
            let asset_log10 = config.sale_amount.0.ilog10();

            let assets = U256::from(investment.weight)
                .checked_mul(U256::from(config.sale_amount.0))
                .ok_or("Multiplication overflow")
                .map(|result| result / U256::from(total_sold_tokens))
                .and_then(to_u128)?;

            // Dimensional reduction
            if weight_log10 > asset_log10 {
                return Ok(assets * 10u128.pow(weight_log10 - asset_log10));
            }
            // Dimensional expansion
            if weight_log10 < asset_log10 {
                return Ok(assets / 10u128.pow(asset_log10 - weight_log10));
            }
            Ok(assets)
        }
    }
}

/// Calculates the assets based on the amount and price fraction.
fn calculate_assets(
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

/// Reverts the asset calculation to get the amount based on the price fraction.
fn calculate_assets_revert(
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

/// Converts a U256 value to u128, ensuring it fits within the range of u128.
fn to_u128(value: U256) -> Result<u128, &'static str> {
    let limbs = value.as_limbs();
    if limbs[2] != 0 || limbs[3] != 0 {
        return Err("Value is too large to fit in u128");
    }
    Ok(u128::from(limbs[0]) | (u128::from(limbs[1]) << 64))
}

mod test_utils {
    #![allow(dead_code, clippy::wildcard_imports)]

    use super::*;
    use crate::{DepositToken, DistributionProportions, IntentAccount};
    use aurora_launchpad_types::config::StakeholderProportion;
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
}

#[cfg(test)]
mod tests_claim {
    use super::*;
    use crate::mechanics::test_utils::{NOW, price_discovery_config};
    use near_sdk::json_types::U128;

    #[test]
    fn test_zero_weight() {
        let config = price_discovery_config();
        let investment = InvestmentAmount::default();
        let total_sold_tokens = 1000;

        let res = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
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

        let res = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
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

        let res = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
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

        let res = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
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

        let res = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        let expected = (120 * 2 * 10u128.pow(18) / 220) * 10u128.pow(6);
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

        let res = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        let expected = 120 * 2 * 10u128.pow(24) / (220 * 10u128.pow(6));
        assert_eq!(res, expected);
    }
}

#[cfg(test)]
mod tests_deposit {
    #![allow(clippy::wildcard_imports)]
    use super::test_utils::{NOW, TEN_DAYS, fixed_price_config, price_discovery_config};
    use super::*;
    use aurora_launchpad_types::config::Discount;

    #[test]
    fn test_deposit_price_discovery_no_discount() {
        let config = price_discovery_config();
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 10u128.pow(29); // 100k tokens

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
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
        assert_eq!(for_claim, total_sold_tokens / 3);
    }

    #[test]
    fn test_deposit_price_discovery_with_discount() {
        let mut config = price_discovery_config();
        config.discounts.push(Discount {
            start_date: NOW,
            end_date: NOW + TEN_DAYS,
            percentage: 120, // 20%
        });
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 10u128.pow(29); // 100k tokens

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
        );

        let expected_weight = deposit_amount * 120 / 100; // 120k tokens
        assert_eq!(result, Ok(0));
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, expected_weight);
        assert_eq!(total_deposited, deposit_amount);
        assert_eq!(total_sold_tokens, expected_weight);

        // Check claim with fake `total_tokens_sold`
        total_sold_tokens += 2 * deposit_amount;
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, 1125 * deposit_amount / 1000);
    }

    #[test]
    fn test_deposit_fixed_price_no_discount_simple() {
        let config = fixed_price_config();
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 10u128.pow(29); // 100k tokens

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
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
        let mut config = fixed_price_config();
        config.discounts.push(Discount {
            start_date: NOW,
            end_date: NOW + TEN_DAYS,
            percentage: 125, // 25%
        });
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 10u128.pow(29); // 100k tokens

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
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
    fn _fixed_price_reached_sale_amount_no_discount() {
        let config = fixed_price_config();
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 2 * 10u128.pow(29); // 200k tokens

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
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
        let mut config = fixed_price_config();
        config.discounts.push(Discount {
            start_date: NOW,
            end_date: NOW + TEN_DAYS,
            percentage: 125, // 25%
        });
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 2 * 10u128.pow(29); // 200k tokens

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
        )
        .unwrap();

        // 2 Million tokens - with discount 25%: price * deposit * discount - sale_amount
        let assets_excess =
            (20 * deposit_amount * 125 / 100) - 10u128.pow(6) * config.sale_amount.0;
        // Exclude discount from assets excess and dived to token price 20
        let expected_result = (assets_excess * 100 / 125) / 20;
        let expected_assets = config.sale_amount;
        assert_eq!(result, expected_result);

        assert_eq!(result, 8 * 10u128.pow(28)); // 80k tokens
        assert_eq!(investment.amount, deposit_amount - result);
        assert_eq!(investment.weight, expected_assets.0);
        assert_eq!(total_deposited, deposit_amount - result);
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

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
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

        let result = calculate_assets(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 5 * TOKEN_SCALE);
    }

    #[test]
    fn test_small_fraction_result() {
        let amount = 1;
        let deposit_token = 10u128.pow(24);
        let sale_token = 1;

        let result = calculate_assets(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_price_is_one_token_scale() {
        let amount = 42;
        let deposit_token = 1;
        let sale_token = 1;

        let result = calculate_assets(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_multiplication_overflow() {
        let amount = (u128::MAX / 10u128.pow(24)) + 1;
        let deposit_token = 1;
        let sale_token = 10u128.pow(24);

        let result = calculate_assets(amount, deposit_token, sale_token);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_when_price_is_less_then_amount() {
        let amount = 10 * 10u128.pow(24);
        let deposit_token = 31;
        let sale_token = 10u128.pow(4);

        let result = calculate_assets(amount, deposit_token, sale_token).unwrap();
        let expected = U256::from(amount) * U256::from(sale_token) / U256::from(deposit_token);
        assert_eq!(result, 3_225_806_451_612_903_225_806_451_612);
        assert_eq!(result, to_u128(expected).unwrap());
    }

    #[test]
    fn test_when_decimals_24_18() {
        let amount = 10 * 10u128.pow(24);
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_assets(amount, deposit_token, sale_token).unwrap();
        let expected = 10u128.pow(19) / 3;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18_low_amount() {
        let amount = 10 * 10u128.pow(6);
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_assets(amount, deposit_token, sale_token).unwrap();
        let expected = 10u128.pow(1) / 3;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18_too_small_amount() {
        let amount = 10 * 10u128.pow(5);
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_assets(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_when_decimals_18_24() {
        let amount = 10 * 10u128.pow(18);
        let deposit_token = 3;
        let sale_token = 10u128.pow(6);

        let result = calculate_assets(amount, deposit_token, sale_token).unwrap();
        let expected = 10u128.pow(25) / 3;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_18_24_low_amount() {
        let amount = 10;
        let deposit_token = 3;
        let sale_token = 10u128.pow(6);

        let result = calculate_assets(amount, deposit_token, sale_token).unwrap();
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

        let result = calculate_assets_revert(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_price_less_than_token_scale() {
        let amount = 5;
        let deposit_token = 1;
        let sale_token = 2;

        let result = calculate_assets_revert(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_zero_amount() {
        let amount = 0;
        let deposit_token = 2;
        let sale_token = 1;

        let result = calculate_assets_revert(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_division_truncates_fraction() {
        let amount = 1;
        let deposit_token = 10u128.pow(24) + 10u128.pow(24) / 2;
        let sale_token = 10u128.pow(24);

        let result = calculate_assets_revert(amount, deposit_token, sale_token).unwrap();
        assert_eq!(result, 1); // floor(1.5)
    }

    #[test]
    fn test_multiplication_overflow_should_fail() {
        let amount = u128::MAX / 2 + 1;
        let deposit_token = 2;
        let sale_token = 1;

        let result = calculate_assets_revert(amount, deposit_token, sale_token);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_large_valid_multiplication_no_overflow() {
        // Should be OK just under an overflow threshold
        let amount = u128::MAX / 2;
        let deposit_token = 2;
        let sale_token = 1;

        let result = calculate_assets_revert(amount, deposit_token, sale_token);
        assert!(result.is_ok());
    }

    #[test]
    fn test_when_price_is_less_then_amount() {
        let amount = 10u128.pow(25);
        let deposit_token = 31;
        let sale_token = 10u128.pow(6);

        let result = calculate_assets_revert(amount, deposit_token, sale_token).unwrap();
        let expected = amount * 31 / 10u128.pow(6);
        assert_eq!(result, 10 * 31 * 10u128.pow(18));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18() {
        let amount = 10 * 10u128.pow(24);
        let deposit_token = 3;
        let sale_token = 10u128.pow(6);

        let result = calculate_assets_revert(amount, deposit_token, sale_token).unwrap();
        let expected = 3 * 10u128.pow(19);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_18_24() {
        let amount = 10 * 10u128.pow(18);
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_assets_revert(amount, deposit_token, sale_token).unwrap();
        let expected = 3 * 10u128.pow(25);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_18_24_revert() {
        let amount = 10u128.pow(19) / 3;
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_assets_revert(amount, deposit_token, sale_token).unwrap();
        let expected = (10u128.pow(19) - 1) * 10u128.pow(6);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_18_24_low_amount_revert() {
        let amount = 10u128.pow(1) / 3;
        let deposit_token = 3 * 10u128.pow(6);
        let sale_token = 1;

        let result = calculate_assets_revert(amount, deposit_token, sale_token).unwrap();
        let expected = 9 * 10u128.pow(6);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18_revert() {
        let amount = 10u128.pow(25) / 3;
        let deposit_token = 3;
        let sale_token = 10u128.pow(6);

        let result = calculate_assets_revert(amount, deposit_token, sale_token).unwrap();
        let expected = 10 * 10u128.pow(18) - 1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18_low_amount_revert() {
        let amount = 10u128.pow(7) / 3;
        let deposit_token = 3;
        let sale_token = 10u128.pow(6);

        let result = calculate_assets_revert(amount, deposit_token, sale_token).unwrap();
        let expected = 10 - 1;
        assert_eq!(result, expected);
    }
}

#[cfg(test)]
mod tests_to_u128 {
    use super::*;
    use alloy_primitives::ruint::aliases::U256;

    #[test]
    fn test_zero() {
        let value = U256::from(0u128);
        let result = to_u128(value);
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_max_u128() {
        let value = U256::from(u128::MAX);
        let result = to_u128(value);
        assert_eq!(result.unwrap(), u128::MAX);
    }

    #[test]
    fn test_mid_value() {
        let val: u128 = 123_456_789_000_000_000_000_000;
        let value = U256::from(val);
        let result = to_u128(value);
        assert_eq!(result.unwrap(), val);
    }

    #[test]
    fn test_exactly_2_u64_limbs_set() {
        // Set lower 64 bits and upper 64 bits within u128 range
        let low = u64::MAX;
        let high = 42u64;
        let value = U256::from_limbs([low, high, 0, 0]);
        let expected = (u128::from(high) << 64) | u128::from(low);
        let result = to_u128(value);
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_overflow_in_third_limb() {
        let value = U256::from_limbs([0, 0, 1, 0]);
        let result = to_u128(value);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_overflow_in_fourth_limb() {
        let value = U256::from_limbs([0, 0, 0, 1]);
        let result = to_u128(value);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_overflow_both_third_and_fourth_limbs() {
        let value = U256::from_limbs([1, 1, u64::MAX, u64::MAX]);
        let result = to_u128(value);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }
}

#[cfg(test)]
mod tests_withdraw {
    #![allow(clippy::wildcard_imports)]
    use super::test_utils::{NOW, TEN_DAYS, fixed_price_config, price_discovery_config};
    use super::*;
    use aurora_launchpad_types::config::Discount;

    #[test]
    fn test_withdraw_fixed_price() {
        let config = fixed_price_config();
        let mut investment = InvestmentAmount {
            amount: 2 * 10u128.pow(25),
            weight: 2 * 10u128.pow(25),
            claimed: 0,
        };
        let mut total_deposited = 2 * 10u128.pow(25);
        let mut total_sold_tokens = 2 * 10u128.pow(25);
        let withdraw_amount = 3 * 10u128.pow(24);

        let result = withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
        );

        assert_eq!(
            result.unwrap_err(),
            "Partial withdrawal is allowed only in Price Discovery"
        );
    }

    #[test]
    fn test_withdraw_price_discovery_large_amount() {
        let config = price_discovery_config();
        let deposit_amount = 2 * 10u128.pow(25);
        let weight_amount = 2 * 10u128.pow(25);
        let mut investment = InvestmentAmount {
            amount: deposit_amount,
            weight: weight_amount,
            claimed: 0,
        };
        let mut total_deposited = 2 * 10u128.pow(25);
        let mut total_sold_tokens = 2 * 10u128.pow(25);
        let withdraw_amount = 2 * 10u128.pow(25) + 1;

        let result = withdraw(
            &mut investment,
            withdraw_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
        );

        assert_eq!(result.unwrap_err(), "Insufficient funds to withdraw");
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, weight_amount);
        assert_eq!(total_deposited, deposit_amount);
        assert_eq!(total_sold_tokens, weight_amount);

        // Check claim with fake `total_tokens_sold`
        total_sold_tokens *= 3;
        println!("{:?}", total_sold_tokens);
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, 10u128.pow(25));
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
        assert_eq!(for_claim, 10u128.pow(25));
    }

    #[test]
    fn test_withdraw_price_discovery_no_discount_for_discount_deposit() {
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

        let result = withdraw(
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
        println!("{:?}", total_sold_tokens);
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, 10u128.pow(25));
    }

    #[test]
    fn test_withdraw_price_discovery_with_normal_discount() {
        let mut config = price_discovery_config();
        config.discounts.push(Discount {
            start_date: NOW,
            end_date: NOW + TEN_DAYS,
            percentage: 125, // 25%
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

        let result = withdraw(
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
        println!("{:?}", total_sold_tokens);
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, 10u128.pow(25));
    }

    #[test]
    fn test_withdraw_price_discovery_with_less_discount() {
        let mut config = price_discovery_config();
        config.discounts.push(Discount {
            start_date: NOW,
            end_date: NOW + TEN_DAYS,
            percentage: 110, // 10%
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

        let result = withdraw(
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
        println!("{:?}", total_sold_tokens);
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, 10u128.pow(25));
    }

    #[test]
    fn test_withdraw_price_discovery_with_greater_discount() {
        // NOTE: this test case is unusual and in common sense unexpected  when discount increased.
        // When discount increased
        let mut config = price_discovery_config();
        config.discounts.push(Discount {
            start_date: NOW,
            end_date: NOW + TEN_DAYS,
            percentage: 170, // 70%
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

        let result = withdraw(
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
        println!("{:?}", total_sold_tokens);
        let for_claim = available_for_claim(&investment, total_sold_tokens, &config, NOW).unwrap();
        assert_eq!(for_claim, 10u128.pow(25));
    }
}
