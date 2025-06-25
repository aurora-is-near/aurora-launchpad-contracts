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
    investment.amount += amount;
    *total_deposited += amount;

    // For fixed price mechanics, we need to calculate the assets based on the weight and price
    // and validate a total sale amount.
    if let Mechanics::FixedPrice {
        price,
        deposit_token_decimals,
        sale_token_decimals,
        price_token_decimals,
    } = config.mechanics
    {
        if price.0 == 0 {
            return Err("A price must be greater than zero");
        }
        // Calculate the assets based on the weight and price
        // We use U256 to handle large numbers and avoid overflow
        let assets = calculate_assets(
            weight,
            price.0,
            deposit_token_decimals,
            sale_token_decimals,
            price_token_decimals,
        )?;
        investment.weight += assets;
        *total_sold_tokens += assets;

        // Check if the total sold tokens exceed the total sale amount
        if *total_sold_tokens > config.total_sale_amount.0 {
            // Recalculate the excess assets based on token price
            let assets_excess = calculate_assets_revert(
                *total_sold_tokens - config.total_sale_amount.0,
                price.0,
                deposit_token_decimals,
                sale_token_decimals,
                price_token_decimals,
            )?;
            // Remain recalculation logic based on the discount
            let remain = match discount {
                Some(disc) => {
                    assets_excess
                        .checked_mul(100)
                        .ok_or("Multiplication overflow")?
                        / u128::from(disc.percentage)
                }
                None => assets_excess,
            };
            investment.amount -= remain;
            investment.weight -= assets_excess;
            *total_deposited -= remain;
            *total_sold_tokens -= assets_excess;

            return Ok(remain);
        }
    } else {
        investment.weight += weight;
        *total_sold_tokens += weight;
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
    investment.amount -= amount;
    // Decrease total investment amount
    *total_deposited -= amount;

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
    *total_sold_tokens -= weight - investment.weight;

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
            let assets = U256::from(investment.weight)
                .checked_mul(U256::from(total_sold_tokens))
                .ok_or("Multiplication overflow")?
                / U256::from(config.sale_amount.0);
            Ok(to_u128(assets)?)
        }
    }
}

/// Calculates the assets based on the amount and price. Actually, it is swapping the amount.
fn calculate_assets(
    amount: u128,
    price: u128,
    deposit_token_decimals: u32,
    sale_token_decimals: u32,
    price_token_decimals: u32,
) -> Result<u128, &'static str> {
    let (decimals, price_div) =
        if price_token_decimals + sale_token_decimals >= deposit_token_decimals {
            (
                price_token_decimals + sale_token_decimals - deposit_token_decimals,
                1_u128,
            )
        } else {
            (
                0,
                u128::from(deposit_token_decimals - (price_token_decimals + sale_token_decimals)),
            )
        };
    let factor = 10u128.pow(decimals);
    let res = U256::from(amount)
        .checked_mul(U256::from(factor))
        .ok_or("Multiplication overflow")
        .map(|result| result / U256::from(price))
        .and_then(to_u128)?;
    Ok(if price_div > 1 { res / price_div } else { res })
}

/// Reverts the asset calculation to get the amount based on the price.
fn calculate_assets_revert(
    amount: u128,
    price: u128,
    sale_token_decimals: u32,
    deposit_token_decimals: u32,
    price_token_decimals: u32,
) -> Result<u128, &'static str> {
    if price_token_decimals + sale_token_decimals >= deposit_token_decimals {
        let tokens_denominator =
            10u128.pow(price_token_decimals + sale_token_decimals - deposit_token_decimals);
        U256::from(amount)
            .checked_mul(U256::from(price))
            .ok_or("Multiplication overflow")
            .map(|result| result / U256::from(tokens_denominator))
            .and_then(to_u128)
    } else {
        let tokens_denominator =
            10u128.pow(deposit_token_decimals - (price_token_decimals + sale_token_decimals));
        U256::from(amount)
            .checked_mul(U256::from(price))
            .ok_or("Multiplication overflow")?
            .checked_mul(U256::from(tokens_denominator))
            .ok_or("Multiplication overflow")
            .and_then(to_u128)
    }
}

/// Converts a U256 value to u128, ensuring it fits within the range of u128.
fn to_u128(value: U256) -> Result<u128, &'static str> {
    let limbs = value.as_limbs();
    if limbs[2] != 0 || limbs[3] != 0 {
        return Err("Value is too large to fit in u128");
    }
    Ok(u128::from(limbs[0]) | (u128::from(limbs[1]) << 64))
}

#[cfg(test)]
mod tests_deposit {
    use super::*;
    use crate::{DepositToken, DistributionProportions, IntentAccount};
    use aurora_launchpad_types::config::{Discount, StakeholderProportion};
    use near_sdk::json_types::U128;

    const DEPOSIT_TOKEN_ID: &str = "wrap.near";
    const SALE_TOKEN_ID: &str = "sale.token.near";
    const INTENTS_ACCOUNT_ID: &str = "intents.near";
    const SOLVER_ACCOUNT_ID: &str = "solver.near";
    const NOW: u64 = 1_000_000_000;
    const TEN_DAYS: u64 = 10 * 24 * 60 * 60;

    fn base_config(mechanics: Mechanics) -> LaunchpadConfig {
        LaunchpadConfig {
            deposit_token: DepositToken::Nep141(DEPOSIT_TOKEN_ID.parse().unwrap()),
            sale_token_account_id: SALE_TOKEN_ID.parse().unwrap(),
            intents_account_id: INTENTS_ACCOUNT_ID.parse().unwrap(),
            start_date: NOW,
            end_date: NOW + TEN_DAYS,
            soft_cap: U128(1_000_000),
            mechanics,
            sale_amount: U128(10u128.pow(30)), // 1 Million tokens
            total_sale_amount: U128(10u128.pow(31)), // 10 Million tokens
            vesting_schedule: None,
            distribution_proportions: DistributionProportions {
                solver_account_id: IntentAccount(SOLVER_ACCOUNT_ID.to_string()),
                solver_allocation: U128(5 * 10u128.pow(30)), // 5 Million tokens
                stakeholder_proportions: vec![StakeholderProportion {
                    allocation: U128(4 * 10u128.pow(30)), // 4 Million tokens
                    account: IntentAccount("team.near".to_string()),
                }],
            },
            discounts: vec![],
        }
    }

    fn price_discovery_config() -> LaunchpadConfig {
        base_config(Mechanics::PriceDiscovery)
    }

    fn fixed_price_config(
        price: u128,
        deposit_token_decimals: u32,
        sale_token_decimals: u32,
        price_token_decimals: u32,
    ) -> LaunchpadConfig {
        base_config(Mechanics::FixedPrice {
            price: U128(price),
            deposit_token_decimals,
            sale_token_decimals,
            price_token_decimals,
        })
    }

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

        assert_eq!(result, Ok(0));
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, deposit_amount);
        assert_eq!(total_deposited, deposit_amount);
        assert_eq!(total_sold_tokens, deposit_amount);
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
    }

    #[test]
    fn test_deposit_fixed_price_no_discount_simple() {
        // price = 0.5 USD
        let price = 5 * 10u128.pow(5);
        let config = fixed_price_config(price, 18, 18, 6);
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

        let expected_weight = deposit_amount * 2;
        assert_eq!(result, Ok(0));
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, expected_weight);
        assert_eq!(total_deposited, deposit_amount);
        assert_eq!(total_sold_tokens, expected_weight);
    }

    #[test]
    fn test_deposit_fixed_price_no_discount_decimals_d24_s18_p6() {
        // price = 0.5 USD
        let price = 5 * 10u128.pow(5);
        let config = fixed_price_config(price, 24, 18, 6);
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

        let expected_assets = deposit_amount / price;
        assert_eq!(result, Ok(0));
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, expected_assets);
        assert_eq!(total_deposited, deposit_amount);
        assert_eq!(total_sold_tokens, expected_assets);
    }

    #[test]
    fn test_deposit_fixed_price_no_discount_decimals_d18_s24_p6() {
        // price = 0.5 USD
        let price = 5 * 10u128.pow(5);
        let config = fixed_price_config(price, 18, 24, 6);
        let mut investment = InvestmentAmount::default();
        let mut total_deposited = 0;
        let mut total_sold_tokens = 0;
        let deposit_amount = 10u128.pow(23); // 100k tokens

        let result = deposit(
            &mut investment,
            deposit_amount,
            &mut total_deposited,
            &mut total_sold_tokens,
            &config,
            NOW + 1,
        );

        let expected_assets = 10u128.pow(24 + 6 - 18) * (deposit_amount / price);
        assert_eq!(result, Ok(0));
        assert_eq!(investment.amount, deposit_amount);
        assert_eq!(investment.weight, expected_assets);
        assert_eq!(total_deposited, deposit_amount);
        assert_eq!(total_sold_tokens, expected_assets);
    }

    /*
              #[test]
              fn test_deposit_fixed_price_with_discount_decimals_d24_s18_p6() {
           // price = 0.5 USD
           let price = 5 * 10u128.pow(5);
                  let mut config = fixed_price_config(price, 24, 18, 6);
                  config.discounts.push(Discount {
                      start_date: NOW,
                      end_date: NOW + TEN_DAYS,
                      percentage: 125,// 25%
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

                  let discounted_amount = deposit_amount * 125 / 100;
                  let expected_assets = (U256::from(discounted_amount) / U256::from(2_000_000)).as_u128();
                  assert_eq!(expected_assets, 625 * 10u128.pow(18));

                  assert_eq!(result, Ok(0));
                  assert_eq!(investment.amount, deposit_amount);
                  assert_eq!(investment.weight, expected_assets);
                  assert_eq!(total_deposited, deposit_amount);
                  assert_eq!(total_sold_tokens, expected_assets);
              }


              #[test]
              fn test_deposit_fixed_price_with_discount_decimals_d18_s24_p6() {
           // price = 0.5 USD
           let price = 5 * 10u128.pow(5);

                  let mut config = fixed_price_config(price, 18, 24, 6);
                  config.discounts.push(Discount {
                      start_date: NOW,
                      end_date: NOW + TEN_DAYS,
                      percentage: 150,
                  });
                  let mut investment = InvestmentAmount::default();
                  let mut total_deposited = 0;
                  let mut total_sold_tokens = 0;
                  let deposit_amount = 10u128.pow(23); // 100k tokens

                  let result = deposit(
                      &mut investment,
                      deposit_amount,
                      &mut total_deposited,
                      &mut total_sold_tokens,
                      &config,
                      NOW + 1,
                  );

                  let discounted_amount = deposit_amount * 150 / 100;
                  let factor = 10u128.pow(12);
                  let expected_assets =
                      (U256::from(discounted_amount) * U256::from(factor) / U256::from(2_000_000));
                  assert_eq!(expected_assets, 750 * 10u128.pow(24));

                  assert_eq!(result, Ok(0));
                  assert_eq!(investment.amount, deposit_amount);
                  assert_eq!(investment.weight, expected_assets);
                  assert_eq!(total_deposited, deposit_amount);
                  assert_eq!(total_sold_tokens, expected_assets);
              }

              #[test]
              fn test_deposit_fixed_price_reached_sale_amount_no_discount_d18_s24_p6() {
                  let total_sale_amount = 500 * 10u128.pow(24);
                  let mut config = fixed_price_config(2_000_000, 18, 24, 6);
                  config.total_sale_amount = U128(total_sale_amount);

                  let mut investment = InvestmentAmount::default();
                  let mut total_deposited = 0;
                  let mut total_sold_tokens = 450 * 10u128.pow(24);

                  let deposit_amount = 100 * 10u128.pow(18);

                  let result = deposit(
                      &mut investment,
                      deposit_amount,
                      &mut total_deposited,
                      &mut total_sold_tokens,
                      &config,
                      NOW + 1,
                  );

                  let available_to_buy = total_sale_amount - total_sold_tokens;
                  let cost =
                      calculate_assets_revert(available_to_buy, config.mechanics.price.0, 24, 18, 6).unwrap();
                  let refund = deposit_amount - cost;

                  assert_eq!(result, Ok(refund));
                  assert_eq!(investment.amount, cost);
                  assert_eq!(investment.weight, available_to_buy);
                  assert_eq!(total_deposited, cost);
                  assert_eq!(total_sold_tokens, total_sale_amount);
              }

              #[test]
              fn test_deposit_fixed_price_reached_sale_amount_with_discount_d24_s18_p6() {
                  let total_sale_amount = 600 * 10u128.pow(18);
                  let mut config = fixed_price_config(2_000_000, 24, 18, 6);
                  config.total_sale_amount = U128(total_sale_amount);
                  config.discounts.push(Discount {
                      start_date: NOW,
                      end_date: NOW + TEN_DAYS,
                      percentage: 120,
                  }); // 20% бонус

                  let mut investment = InvestmentAmount::default();
                  let mut total_deposited = 0;
                  let mut total_sold_tokens = 500 * 10u128.pow(18);

                  let deposit_amount = 100 * 10u128.pow(24);

                  let result = deposit(
                      &mut investment,
                      deposit_amount,
                      &mut total_deposited,
                      &mut total_sold_tokens,
                      &config,
                      NOW + 1,
                  );

                  let available_to_buy = total_sale_amount - total_sold_tokens;
                  let cost_in_discounted_tokens =
                      calculate_assets_revert(available_to_buy, config.mechanics.price.0, 18, 24, 6).unwrap();
                  let actual_cost = cost_in_discounted_tokens * 100 / 120;
                  let refund = deposit_amount - actual_cost;

                  assert_eq!(result, Ok(refund));
                  assert_eq!(investment.amount, actual_cost);
                  assert_eq!(investment.weight, available_to_buy);
                  assert_eq!(total_deposited, actual_cost);
                  assert_eq!(total_sold_tokens, total_sale_amount);
              }

              #[test]
              fn test_deposit_zero_amount() {
                  let config = price_discovery_config();
                  let mut investment = InvestmentAmount::default();
                  let mut total_deposited = 100;
                  let mut total_sold_tokens = 100;

                  let result = deposit(
                      &mut investment,
                      0,
                      &mut total_deposited,
                      &mut total_sold_tokens,
                      &config,
                      NOW + 1,
                  );

                  assert_eq!(result, Ok(0));
                  assert_eq!(investment.amount, 0);
                  assert_eq!(investment.weight, 0);
                  assert_eq!(total_deposited, 100); // Не изменилось
                  assert_eq!(total_sold_tokens, 100); // Не изменилось
              }

              #[test]
              fn test_deposit_fixed_price_zero_price() {
                  let config = fixed_price_config(0, 18, 18, 0);
                  let mut investment = InvestmentAmount::default();
                  let mut total_deposited = 0;
                  let mut total_sold_tokens = 0;

                  let result = deposit(
                      &mut investment,
                      1000,
                      &mut total_deposited,
                      &mut total_sold_tokens,
                      &config,
                      NOW + 1,
                  );

                  assert_eq!(result, Err("A price must be greater than zero"));
              }

              #[test]
              fn test_deposit_fixed_price_sale_exactly_at_cap() {
                  let total_sale_amount = 500 * 10u128.pow(18);
                  let mut config = fixed_price_config(2_000_000_000, 18, 18, 6);
                  config.total_sale_amount = U128(total_sale_amount);

                  let mut investment = InvestmentAmount::default();
                  let mut total_deposited = 0;
                  let mut total_sold_tokens = 400 * 10u128.pow(18);

                  let deposit_amount =
                      calculate_assets_revert(100 * 10u128.pow(18), config.mechanics.price.0, 18, 18, 6)
                          .unwrap();

                  let result = deposit(
                      &mut investment,
                      deposit_amount,
                      &mut total_deposited,
                      &mut total_sold_tokens,
                      &config,
                      NOW + 1,
                  );

                  assert_eq!(result, Ok(0));
                  assert_eq!(investment.amount, deposit_amount);
                  assert_eq!(investment.weight, 100 * 10u128.pow(18));
                  assert_eq!(total_deposited, deposit_amount);
                  assert_eq!(total_sold_tokens, total_sale_amount);
              }
    */
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
        let price = 2 * TOKEN_SCALE;

        let result = calculate_assets(amount, price, 24, 24, 24).unwrap();
        assert_eq!(result, 5 * TOKEN_SCALE);
    }

    #[test]
    fn test_small_fraction_result() {
        let amount = 1;
        let price = 2 * TOKEN_SCALE;
        let result = calculate_assets(amount, price, 24, 24, 24).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_price_is_one_token_scale() {
        let amount = 42;
        let price = TOKEN_SCALE;
        let result = calculate_assets(amount, price, 24, 24, 24).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_multiplication_overflow() {
        // Max safe value before overflow: U128::MAX / TOKEN_SCALE
        let overflow_amount = (u128::MAX / TOKEN_SCALE) + 1;
        let price = 1;
        let result = calculate_assets(overflow_amount, price, 24, 24, 24);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_when_price_is_less_then_amount() {
        let amount = 10 * TOKEN_SCALE;
        let price: u128 = 31 * 10u128.pow(20);
        let result = calculate_assets(amount, price, 24, 24, 24).unwrap();
        let expected = U256::from(amount) * U256::from(TOKEN_SCALE) / U256::from(price);
        assert_eq!(result, 3_225_806_451_612_903_225_806_451_612);
        assert_eq!(result, to_u128(expected).unwrap());
    }

    #[test]
    fn test_when_decimals_24_18_6() {
        let amount = 10 * 10u128.pow(24);
        let price = 3 * 10u128.pow(6);
        let result = calculate_assets(amount, price, 24, 18, 6).unwrap();
        let expected = 10u128.pow(19) / 3;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18_6_low_amount() {
        let amount = 10 * 10u128.pow(6);
        let price = 3 * 10u128.pow(6);
        let result = calculate_assets(amount, price, 24, 18, 6).unwrap();
        let expected = 10u128.pow(1) / 3;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_24_18_6_too_small_amount() {
        let amount = 10 * 10u128.pow(5);
        let price = 3 * 10u128.pow(6);
        let result = calculate_assets(amount, price, 24, 18, 6).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_when_decimals_18_24_6() {
        let amount = 10 * 10u128.pow(18);
        let price = 3 * 10u128.pow(6);
        let result = calculate_assets(amount, price, 18, 24, 6).unwrap();
        let expected = 10u128.pow(25) / 3;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_when_decimals_18_24_6_low_amount() {
        let amount = 10;
        let price = 3 * 10u128.pow(6);
        let result = calculate_assets(amount, price, 18, 24, 6).unwrap();
        let expected = 10u128.pow(7) / 3;
        assert_eq!(result, expected);
    }
}

#[cfg(test)]
mod tests_calculate_assets_revert {
    use super::*;

    const DECIMALS: u32 = 24;
    const TOKEN_SCALE: u128 = 10u128.pow(DECIMALS);

    #[test]
    fn test_normal_case() {
        // amount = 5, price = 2 * TOKEN_SCALE
        // result = 5 * 2 * 10^24 / 10^24 = 10
        let amount = 5;
        let price = 2 * TOKEN_SCALE;
        let result = calculate_assets_revert(amount, price, 24, 24, 24).unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_price_less_than_token_scale() {
        // price = 0.5 token scale
        // result = 5 * 0.5 = 2.5 (truncated to 2)
        let amount = 5;
        let price = TOKEN_SCALE / 2;
        let result = calculate_assets_revert(amount, price, 24, 24, 24).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_zero_amount() {
        let amount = 0;
        let price = 1_000_000;
        let result = calculate_assets_revert(amount, price, 24, 24, 24).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_zero_price() {
        let amount = 10;
        let price = 0;
        let result = calculate_assets_revert(amount, price, 24, 24, 24).unwrap();
        assert_eq!(result, 0); // 0 * 10 = 0 / TOKEN_SCALE = 0
    }

    #[test]
    fn test_division_truncates_fraction() {
        // (5 * TOKEN_SCALE + TOKEN_SCALE / 2) / TOKEN_SCALE = 5.5 -> 5
        let amount = 1;
        let price = TOKEN_SCALE + TOKEN_SCALE / 2; // 1.5
        let result = calculate_assets_revert(amount, price, 24, 24, 24).unwrap();
        assert_eq!(result, 1); // floor(1.5)
    }

    #[test]
    fn test_multiplication_overflow_for_max_should_fail() {
        let amount = u128::MAX;
        let price = u128::MAX;
        let result = calculate_assets_revert(amount, price, 24, 24, 24);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_large_valid_multiplication_no_overflow() {
        // Should be OK just under an overflow threshold
        let max_safe = u128::MAX / 2;
        let price = 2;
        let result = calculate_assets_revert(max_safe, price, 24, 24, 24);
        assert!(result.is_ok());
    }

    #[test]
    fn test_division_result_exceeds_u128_should_fail() {
        // This produces value > u128::MAX
        // (u128::MAX / 2) * TOKEN_SCALE * 3 / TOKEN_SCALE = 3 * (u128::MAX / 2) = > u128::MAX
        let amount = u128::MAX / 2;
        let price = 3 * TOKEN_SCALE;

        let result = calculate_assets_revert(amount, price, 24, 24, 24);
        assert!(result.is_err()); // Because to_u128 fails
    }

    #[test]
    fn test_when_price_is_less_then_amount() {
        let amount = 10 * TOKEN_SCALE;
        let price: u128 = 31 * 10u128.pow(20);
        let deposit_amount = calculate_assets_revert(amount, price, 24, 24, 24).unwrap();
        let expected = U256::from(amount) * U256::from(price) / U256::from(TOKEN_SCALE);
        assert_eq!(deposit_amount, 10 * price);
        assert_eq!(deposit_amount, to_u128(expected).unwrap());
    }

    #[test]
    fn test_when_decimals_24_18_6() {
        let sale_amount = 10 * 10u128.pow(24);
        let price = 3 * 10u128.pow(6);
        let deposit_amount = calculate_assets_revert(sale_amount, price, 24, 18, 6).unwrap();
        let expected = 3 * 10u128.pow(19);
        assert_eq!(deposit_amount, expected);
    }

    #[test]
    fn test_when_decimals_18_24_6() {
        let sale_amount = 10 * 10u128.pow(18);
        let price = 3 * 10u128.pow(6);
        let deposit_amount = calculate_assets_revert(sale_amount, price, 18, 24, 6).unwrap();
        let expected = 3 * 10u128.pow(25);
        assert_eq!(deposit_amount, expected);
    }

    #[test]
    fn test_when_decimals_18_24_6_revert() {
        let amount = 10u128.pow(19) / 3;
        let price = 3 * 10u128.pow(6);
        let deposit_amount = calculate_assets_revert(amount, price, 18, 24, 6).unwrap();
        let expected = (10u128.pow(19) - 1) * 10u128.pow(6);
        assert_eq!(deposit_amount, expected);
    }

    #[test]
    fn test_when_decimals_18_24_6_low_amount_revert() {
        let amount = 10u128.pow(1) / 3;
        let price = 3 * 10u128.pow(6);
        let deposit_amount = calculate_assets_revert(amount, price, 18, 24, 6).unwrap();
        let expected = 9 * 10u128.pow(6);
        assert_eq!(deposit_amount, expected);
    }

    #[test]
    fn test_when_decimals_24_18_6_revert() {
        let sale_amount = 10u128.pow(25) / 3;
        let price = 3 * 10u128.pow(6);
        let deposit_amount = calculate_assets_revert(sale_amount, price, 24, 18, 6).unwrap();
        let expected = 10 * 10u128.pow(18) - 1;
        assert_eq!(deposit_amount, expected);
    }

    #[test]
    fn test_when_decimals_24_18_6_low_amount_revert() {
        let amount = 10u128.pow(7) / 3;
        let price = 3 * 10u128.pow(6);
        let deposit_amount = calculate_assets_revert(amount, price, 24, 18, 6).unwrap();
        let expected = 10 - 1;
        assert_eq!(deposit_amount, expected);
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
