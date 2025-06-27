use alloy_primitives::ruint::aliases::U256;
use aurora_launchpad_types::InvestmentAmount;
use aurora_launchpad_types::config::{LaunchpadConfig, Mechanics};

use crate::mechanics::to_u128;

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

#[cfg(test)]
mod tests {
    use aurora_launchpad_types::InvestmentAmount;
    use near_sdk::json_types::U128;

    use crate::mechanics::claim::available_for_claim;
    use crate::mechanics::test_utils::{NOW, price_discovery_config};

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
