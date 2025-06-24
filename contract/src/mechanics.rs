use alloy_primitives::ruint::aliases::U256;
use aurora_launchpad_types::InvestmentAmount;
use aurora_launchpad_types::config::{LaunchpadConfig, Mechanics};

/// Decimal precision for token amounts, used to represent fractional tokens.
pub const DECIMALS: u32 = 24;
/// The scale factor for token amounts, used to handle decimals in calculations.
pub const TOKEN_SCALE: u128 = 10u128.pow(DECIMALS);

/// Deposits an amount into the investment, applying the current discount if available.
/// 1. For Fixed Price , the weight is calculated based on the price & current discount.
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
    // and validate  total sale amount
    if let Mechanics::FixedPrice { price } = config.mechanics {
        if price.0 == 0 {
            return Err("price cannot be 0");
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
    let value = U256::from(amount)
        .checked_mul(U256::from(TOKEN_SCALE))
        .ok_or("Multiplication overflow")?
        / U256::from(price);
    to_u128(value)
}

/// Reverts the assets calculation to get the amount based on the price.
fn calculate_assets_revert(amount: u128, price: u128) -> Result<u128, &'static str> {
    let value = U256::from(amount)
        .checked_mul(U256::from(price))
        .ok_or("Multiplication overflow")?
        / U256::from(TOKEN_SCALE);
    to_u128(value)
}

/// Converts a U256 value to u128, ensuring it fits within the range of u128.
fn to_u128(value: U256) -> Result<u128, &'static str> {
    let limbs = value.as_limbs();
    if limbs[2] != 0 || limbs[3] != 0 {
        return Err("Value is too large to fit in u128");
    }
    Ok(u128::from(limbs[0]) | (u128::from(limbs[1]) << 64))
}
