use aurora_launchpad_types::InvestmentAmount;
use aurora_launchpad_types::config::LaunchpadConfig;

// TODO: reconsider the softCap and totalDeposits calculation logic
pub fn deposit(
    investment: &mut InvestmentAmount,
    amount: u128,
    total_deposited: &mut u128,
    _total_saled_tokens: &mut u128,
    config: &LaunchpadConfig,
    timestamp: u64,
) -> Result<u128, &'static str> {
    let discount = config.get_current_discount(timestamp);
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
    investment.weight += weight;
    *total_deposited += weight;
    if config.soft_cap.0 < config.total_sale_amount.0 {
        let assets_excess = config.total_sale_amount.0 - config.soft_cap.0;
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
        *total_deposited -= assets_excess;
        remain
    } else {
        0
    }
}

// TODO: reconsider the totalDeposits calculation logic
pub fn withdraw(
    investment: &mut InvestmentAmount,
    amount: u128,
    total_deposited: &mut u128,
    config: &LaunchpadConfig,
    timestamp: u64,
) -> Result<(), &'static str> {
    if amount > investment.amount {
        return Err("Insufficient funds to withdraw");
    }
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
    investment.amount -= amount;
    *total_deposited -= weight - investment.weight;

    Ok(())
}
