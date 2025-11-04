use aurora_launchpad_types::config::Mechanics;
use aurora_launchpad_types::discount::{DepositDistribution, DiscountParams, DiscountPhase};
use near_sdk::test_utils::test_env::alice;

use crate::tests::discount::{TestContext, price_discovery};
use crate::tests::utils::base_config;

const MECHANICS: Mechanics = price_discovery();

#[test]
fn deposit_distribution_without_discount() {
    let config = base_config(MECHANICS);
    let ctx = TestContext::new(config);
    let contract = ctx.contract();
    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, 11);

    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithoutDiscount(deposit)
    );
}

#[test]
fn use_discount_three_phases() {
    let mut config = base_config(MECHANICS);

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: 10,
                end_time: 12,
                percentage: 3000,
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 13,
                end_time: 15,
                percentage: 2000,
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                start_time: 16,
                end_time: 18,
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();

    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 1300)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, 14);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(1, 1200)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, 17);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(2, 1100)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, 19);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithoutDiscount(deposit)
    );
}

#[test]
fn use_whitelist_for_discount_two_phases() {
    let mut config = base_config(MECHANICS);

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: 10,
                end_time: 15,
                percentage: 2000,
                whitelist: Some(std::iter::once(alice().into()).collect()),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 12,
                end_time: 15,
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(13),
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();

    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 1200)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::Refund(deposit) // bob is not in the whitelist and no public sale is available
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, 12);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(1, 1100)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, 15);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithoutDiscount(deposit) // there are no discount phases left, public sale.
    );
}
