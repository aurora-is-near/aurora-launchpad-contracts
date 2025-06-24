use alloy_primitives::ruint::aliases::U256;
use aurora_launchpad_types::InvestmentAmount;
use aurora_launchpad_types::config::{LaunchpadConfig, Mechanics};

/// Decimal precision for token amounts, used to represent fractional tokens.
pub const DECIMALS: u32 = 24;
/// The scale factor for token amounts, used to handle decimals in calculations.
pub const TOKEN_SCALE: u128 = 10u128.pow(DECIMALS);

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
    if let Mechanics::FixedPrice { price } = config.mechanics {
        if price.0 == 0 {
            return Err("A price must be greater than zero");
        }
        // Calculate the assets based on the weight and price
        // We use U256 to handle large numbers and avoid overflow
        let assets = calculate_assets(weight, price.0)?;
        investment.weight += assets;
        *total_sold_tokens += assets;

        // Check if the total sold tokens exceed the total sale amount
        if *total_sold_tokens > config.total_sale_amount.0 {
            // Recalculate the excess assets based on token price
            let assets_excess =
                calculate_assets_revert(*total_sold_tokens - config.total_sale_amount.0, price.0)?;
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
fn calculate_assets(amount: u128, price: u128) -> Result<u128, &'static str> {
    U256::from(amount)
        .checked_mul(U256::from(TOKEN_SCALE))
        .ok_or("Multiplication overflow")
        .map(|result| result / U256::from(price))
        .and_then(to_u128)
}

/// Reverts the asset calculation to get the amount based on the price.
fn calculate_assets_revert(amount: u128, price: u128) -> Result<u128, &'static str> {
    U256::from(amount)
        .checked_mul(U256::from(price))
        .ok_or("Multiplication overflow")
        .map(|result| result / U256::from(TOKEN_SCALE))
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

#[cfg(test)]
mod tests_calculate_assets {
    use super::*;
    use alloy_primitives::ruint::aliases::U256;

    #[test]
    fn test_normal_case() {
        let amount = 10 * TOKEN_SCALE;
        let price = 2 * TOKEN_SCALE;
        let result = calculate_assets(amount, price).unwrap();
        assert_eq!(result, 5 * TOKEN_SCALE);
    }

    #[test]
    fn test_small_fraction_result() {
        let amount = 1;
        let price = 2 * TOKEN_SCALE;
        let result = calculate_assets(amount, price).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_price_is_one_token_scale() {
        let amount = 42;
        let price = TOKEN_SCALE;
        let result = calculate_assets(amount, price).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_multiplication_overflow() {
        // Max safe value before overflow: U128::MAX / TOKEN_SCALE
        let overflow_amount = (u128::MAX / TOKEN_SCALE) + 1;
        let price = 1;
        let result = calculate_assets(overflow_amount, price);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_when_price_is_less_then_amount() {
        let amount = 10 * TOKEN_SCALE;
        let price: u128 = 31 * 10u128.pow(20);
        let result = calculate_assets(amount, price).unwrap();
        let expected = U256::from(amount) * U256::from(TOKEN_SCALE) / U256::from(price);
        assert_eq!(result, 3_225_806_451_612_903_225_806_451_612);
        assert_eq!(result, to_u128(expected).unwrap());
    }
}

#[cfg(test)]
mod tests_calculate_assets_revert {
    use super::*;

    #[test]
    fn test_normal_case() {
        // amount = 5, price = 2 * TOKEN_SCALE
        // result = 5 * 2 * 10^24 / 10^24 = 10
        let amount = 5;
        let price = 2 * TOKEN_SCALE;
        let result = calculate_assets_revert(amount, price).unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_price_less_than_token_scale() {
        // price = 0.5 token scale
        // result = 5 * 0.5 = 2.5 (truncated to 2)
        let amount = 5;
        let price = TOKEN_SCALE / 2;
        let result = calculate_assets_revert(amount, price).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn test_zero_amount() {
        let amount = 0;
        let price = 1_000_000;
        let result = calculate_assets_revert(amount, price).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_zero_price() {
        let amount = 10;
        let price = 0;
        let result = calculate_assets_revert(amount, price).unwrap();
        assert_eq!(result, 0); // 0 * 10 = 0 / TOKEN_SCALE = 0
    }

    #[test]
    fn test_division_truncates_fraction() {
        // (5 * TOKEN_SCALE + TOKEN_SCALE / 2) / TOKEN_SCALE = 5.5 -> 5
        let amount = 1;
        let price = TOKEN_SCALE + TOKEN_SCALE / 2; // 1.5
        let result = calculate_assets_revert(amount, price).unwrap();
        assert_eq!(result, 1); // floor(1.5)
    }

    #[test]
    fn test_multiplication_overflow_for_max_should_fail() {
        let amount = u128::MAX;
        let price = u128::MAX;
        let result = calculate_assets_revert(amount, price);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Value is too large to fit in u128");
    }

    #[test]
    fn test_large_valid_multiplication_no_overflow() {
        // Should be OK just under an overflow threshold
        let max_safe = u128::MAX / 2;
        let price = 2;
        let result = calculate_assets_revert(max_safe, price);
        assert!(result.is_ok());
    }

    #[test]
    fn test_division_result_exceeds_u128_should_fail() {
        // This produces value > u128::MAX
        // (u128::MAX / 2) * TOKEN_SCALE * 3 / TOKEN_SCALE = 3 * (u128::MAX / 2) = > u128::MAX
        let amount = u128::MAX / 2;
        let price = 3 * TOKEN_SCALE;

        let result = calculate_assets_revert(amount, price);
        assert!(result.is_err()); // Because to_u128 fails
    }

    #[test]
    fn test_when_price_is_less_then_amount() {
        let amount = 10 * TOKEN_SCALE;
        let price: u128 = 31 * 10u128.pow(20);
        let result = calculate_assets_revert(amount, price).unwrap();
        let expected = U256::from(amount) * U256::from(price) / U256::from(TOKEN_SCALE);
        assert_eq!(result, 10 * price);
        assert_eq!(result, to_u128(expected).unwrap());
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
