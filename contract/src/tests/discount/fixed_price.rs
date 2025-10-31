use aurora_launchpad_types::discount::{DepositDistribution, DiscountParams, DiscountPhase};
use near_sdk::test_utils::test_env::alice;

use crate::tests::discount::{TestContext, fixed_price};
use crate::tests::utils::{NOW, base_config};

#[test]
fn deposit_distribution_without_discount() {
    let config = base_config(fixed_price(1, 1));
    let ctx = TestContext::new(config);
    let contract = ctx.contract();
    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, NOW + 11);

    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithoutDiscount(deposit)
    );
}

#[test]
fn refund_since_no_public_sale_available() {
    let mut config = base_config(fixed_price(1, 1));

    config.discounts = Some(DiscountParams {
        phases: vec![],
        public_sale_start_time: NOW + 12,
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();
    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, NOW + 11);

    assert_eq!(deposit_distribution, DepositDistribution::Refund(deposit));
}

#[test]
fn public_sale_because_too_early_for_discount() {
    let mut config = base_config(fixed_price(1, 1));

    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: NOW + 12,
            end_time: NOW + 15,
            percentage: 1000,
            whitelist: None,
            phase_sale_limit: None,
            min_limit_per_account: None,
            max_limit_per_account: None,
            remaining_go_to_phase_id: None,
        }],
        public_sale_start_time: NOW + 10,
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();
    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, NOW + 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithoutDiscount(deposit)
    );
}

#[test]
fn min_limit_per_account_not_reached() {
    let mut config = base_config(fixed_price(1, 2));

    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: NOW + 10,
            end_time: NOW + 15,
            percentage: 1000,
            whitelist: None,
            phase_sale_limit: None,
            min_limit_per_account: Some(2201.into()), // We can buy 2200 tokens with a 10% discount for 1000 deposit tokens
            max_limit_per_account: None,
            remaining_go_to_phase_id: None,
        }],
        public_sale_start_time: NOW + 10,
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();
    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, NOW + 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![],
            public_sale_weight: deposit,
            refund: 0,
        }
    );
}

#[test]
fn skip_min_limit_per_account_for_next_deposits() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: NOW + 10,
            end_time: NOW + 15,
            percentage: 1000,
            min_limit_per_account: Some(2200.into()), // We can buy 2200 tokens with a 10% discount for 1000 deposit tokens
            ..Default::default()
        }],
        public_sale_start_time: NOW + 10,
    });

    let ctx = TestContext::new(config);
    let deposit = 1000;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit, NOW + 11);

    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 1100)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    {
        ctx.contract_mut()
            .update_discount_state(ctx.alice(), &deposit_distribution, price);
    }

    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit / 2, NOW + 12);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 1100 / 2)],
            public_sale_weight: 0,
            refund: 0,
        }
    );
}

#[test]
fn use_whitelist_for_discount() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: NOW + 10,
            end_time: NOW + 15,
            percentage: 1000,
            whitelist: Some(std::iter::once(alice().into()).collect()),
            ..Default::default()
        }],
        public_sale_start_time: NOW + 10,
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();

    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, NOW + 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 1100)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, NOW + 12);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithoutDiscount(deposit)
    );
}

#[test]
fn use_whitelist_for_discount_two_phases() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: NOW + 10,
                end_time: NOW + 15,
                percentage: 1000,
                whitelist: Some(std::iter::once(alice().into()).collect()),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: NOW + 12,
                end_time: NOW + 15,
                percentage: 500,
                ..Default::default()
            },
        ],
        public_sale_start_time: NOW + 10,
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();

    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, NOW + 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 1100)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, NOW + 12);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(1, 1050)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, NOW + 15);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithoutDiscount(deposit)
    );
}

#[test]
fn use_max_account_limits_to_walk_through_phases() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: NOW + 10,
                end_time: NOW + 15,
                percentage: 2000,
                max_limit_per_account: Some(2400.into()), // (1000 + 20%) * 2
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: NOW + 10,
                end_time: NOW + 15,
                max_limit_per_account: Some(2200.into()), // (1000 + 10%) * 2
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: NOW + 10,
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit, NOW + 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);

    let deposit = 2500;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit, NOW + 12);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600), (1, 1100)],
            public_sale_weight: 1000,
            refund: 0,
        }
    );
}

#[test]
fn use_max_phase_limits_to_walk_through_phases() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: NOW + 10,
                end_time: NOW + 15,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()), // (1000 + 20%) * 2
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: NOW + 10,
                end_time: NOW + 15,
                phase_sale_limit: Some(2200.into()), // (1000 + 10%) * 2
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: NOW + 10,
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit, NOW + 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);

    let deposit = 2500;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit, NOW + 12);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600), (1, 1100)],
            public_sale_weight: 1000,
            refund: 0,
        }
    );
}

#[test]
fn refund_reach_global_sale_amount() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.sale_amount = 5000.into();
    config.total_sale_amount = 5000.into();
    config.distribution_proportions.solver_allocation = 0.into();
    config.distribution_proportions.stakeholder_proportions = vec![];

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: NOW + 10,
                end_time: NOW + 15,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()), // (1000 + 20%) * 2
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: NOW + 10,
                end_time: NOW + 15,
                phase_sale_limit: Some(2200.into()), // (1000 + 10%) * 2
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: NOW + 10,
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit, NOW + 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200; // (500 + 20%) * 2

    let deposit = 2500;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit, NOW + 12);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600), (1, 1100)],
            public_sale_weight: 200,
            refund: 800,
        }
    );
}

#[test]
fn refund_reach_phase_limits_no_public_sale() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: NOW + 10,
                end_time: NOW + 15,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()), // (1000 + 20%) * 2
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: NOW + 10,
                end_time: NOW + 15,
                phase_sale_limit: Some(2200.into()), // (1000 + 10%) * 2
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: NOW + 15,
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit, NOW + 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200; // (500 + 20%) * 2

    let deposit = 2500;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit, NOW + 12);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600), (1, 1100)],
            public_sale_weight: 0,
            refund: 1000,
        }
    );
}
