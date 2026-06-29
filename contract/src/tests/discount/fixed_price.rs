use aurora_launchpad_types::InvestmentAmount;
use aurora_launchpad_types::config::{LaunchpadConfig, Mechanics};
use aurora_launchpad_types::discount::{DepositDistribution, DiscountParams, DiscountPhase};
use near_sdk::test_utils::test_env::alice;

use crate::mechanics::deposit::deposit;
use crate::tests::discount::{TestContext, fixed_price};
use crate::tests::utils::base_config;

#[test]
fn deposit_distribution_without_discount() {
    let config = base_config(fixed_price(1, 1));
    let ctx = TestContext::new(config);
    let contract = ctx.contract();
    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, 11);

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
        public_sale_start_time: Some(12),
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();
    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, 11);

    assert_eq!(deposit_distribution, DepositDistribution::Refund(deposit));
}

#[test]
fn public_sale_because_too_early_for_discount() {
    let mut config = base_config(fixed_price(1, 1));

    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: 12,
            end_time: 15,
            percentage: 1000,
            ..Default::default()
        }],
        public_sale_start_time: Some(10),
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();
    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, 11);
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
            start_time: 10,
            end_time: 15,
            percentage: 1000,
            min_limit_per_account: Some(2201.into()), // We can buy 2200 tokens with a 10% discount for 1000 deposit tokens
            ..Default::default()
        }],
        public_sale_start_time: Some(10),
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();
    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, 11);
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
            start_time: 10,
            end_time: 15,
            percentage: 1000,
            min_limit_per_account: Some(2200.into()), // We can buy 2200 tokens with a 10% discount for 1000 deposit tokens
            ..Default::default()
        }],
        public_sale_start_time: Some(10),
    });

    let ctx = TestContext::new(config);
    let deposit = 1000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);

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
            .get_deposit_distribution(ctx.alice(), deposit / 2, 12);
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
            start_time: 10,
            end_time: 15,
            percentage: 1000,
            whitelist: Some(std::iter::once(alice().into()).collect()),
            ..Default::default()
        }],
        public_sale_start_time: Some(10),
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();

    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 1100)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, 12);
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
                start_time: 10,
                end_time: 15,
                percentage: 1000,
                whitelist: Some(std::iter::once(alice().into()).collect()),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 12,
                end_time: 15,
                percentage: 500,
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(10),
    });

    let ctx = TestContext::new(config);
    let contract = ctx.contract();

    let deposit = 1000;
    let deposit_distribution = contract.get_deposit_distribution(ctx.alice(), deposit, 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 1100)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, 12);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(1, 1050)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    let deposit_distribution = contract.get_deposit_distribution(ctx.bob(), deposit, 15);
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
                start_time: 10,
                end_time: 15,
                percentage: 2000,
                max_limit_per_account: Some(2400.into()), // (1000 + 20%) * 2
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 10,
                end_time: 15,
                max_limit_per_account: Some(2200.into()), // (1000 + 10%) * 2
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(10),
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);
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
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 12);
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
                start_time: 10,
                end_time: 15,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()), // (1000 + 20%) * 2
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 10,
                end_time: 15,
                phase_sale_limit: Some(2200.into()), // (1000 + 10%) * 2
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(10),
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);
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
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 12);
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
                start_time: 10,
                end_time: 15,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()), // (1000 + 20%) * 2
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 10,
                end_time: 15,
                phase_sale_limit: Some(2200.into()), // (1000 + 10%) * 2
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(10),
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);
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
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 12);
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
                start_time: 10,
                end_time: 15,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()), // (1000 + 20%) * 2
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 10,
                end_time: 15,
                phase_sale_limit: Some(2200.into()), // (1000 + 10%) * 2
                percentage: 1000,
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                start_time: 10,
                end_time: 15,
                phase_sale_limit: Some(2100.into()), // (1000 + 10%) * 2
                percentage: 500,
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(15),
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);
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

    let deposit = 3000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 12);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600), (1, 1100), (2, 1050)],
            public_sale_weight: 0,
            refund: 500,
        }
    );
}

#[test]
fn move_unsold_limit_to_another_phase() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: 10,
                end_time: 12,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 13,
                end_time: 15,
                percentage: 1500,
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                start_time: 16,
                end_time: 18,
                phase_sale_limit: Some(1000.into()), // Should be moved 1200 from phase 0.
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(10),
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600)], // 2400 - (500 + 20%) * 2 = 1200 sale tokens left.
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200; // (500 + 20%) * 2

    let deposit = 2000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 14);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(1, 2300)], // There is no limit. 2000 + 15%
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 4600; // (500 + 20%) * 2 + (2000 + 15%) * 2

    let deposit = 2000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 17);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(2, 1100)], // limit 1000 + 1200 from phase 0.
            public_sale_weight: 1000,
            refund: 0,
        }
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn move_unsold_limit_to_public_sale() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.sale_amount = 10_000.into();
    config.total_sale_amount = 10_000.into();
    config.distribution_proportions.solver_allocation = 0.into();
    config.distribution_proportions.stakeholder_proportions = vec![];

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: 10,
                end_time: 12,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 13,
                end_time: 15,
                percentage: 1500,
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                start_time: 16,
                end_time: 18,
                phase_sale_limit: Some(1000.into()), // Should be moved 1200 from phase 0.
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    });

    let ctx = TestContext::new(config.clone());
    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600)], // 2400 - (500 + 20%) * 2 = 1200 sale tokens left.
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200; // (500 + 20%) * 2

    let deposit = 2000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 14);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(1, 2300)], // There is no limit. 2000 + 15%
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 4600; // (500 + 20%) * 2 + (2000 + 15%) * 2

    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 17);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(2, 550)], // limit 1000 + 1200 from phase 0.
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 4600 + 1100; // (500 + 20%) * 2 + (2000 + 15%) * 2 + (500 + 10%) * 2

    let deposit = 2000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 19);

    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithoutDiscount(2000)
    );

    let contract = ctx.contract();
    let mut investments = InvestmentAmount::default();
    let mut total_deposited = contract.total_deposited;
    let mut total_sold_tokens = contract.total_sold_tokens;
    let refund = crate::mechanics::deposit::deposit(
        &mut investments,
        deposit,
        &mut total_deposited,
        &mut total_sold_tokens,
        &config,
        &deposit_distribution,
    )
    .unwrap();
    // Left tokens: 10_000 - 1200 - 4600 - 1100 = 3100 => can sell for 1550 deposit tokens
    assert_eq!(refund, deposit - 1550);
}

#[test]
#[allow(clippy::too_many_lines)]
fn specify_where_to_move_unsold_tokens() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.sale_amount = 100_000.into();
    config.total_sale_amount = 100_000.into();
    config.distribution_proportions.solver_allocation = 0.into();
    config.distribution_proportions.stakeholder_proportions = vec![];

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: 10,
                end_time: 12,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()),
                remaining_go_to_phase_id: Some(2),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 13,
                end_time: 15,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()),
                remaining_go_to_phase_id: Some(2),
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                start_time: 16,
                end_time: 18,
                phase_sale_limit: Some(1000.into()), // Should be moved 1200 from phase 0.
                percentage: 1000,
                ..Default::default()
            },
            DiscountPhase {
                id: 3,
                start_time: 19,
                end_time: 21,
                phase_sale_limit: Some(1000.into()), // Should be moved 1200 from phase 1.
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600)], // 2400 - (500 + 20%) * 2 = 1200 sale tokens left.
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200; // (500 + 20%) * 2

    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 14);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(1, 600)], // Left tokens from phase 0 go to the phase with id: 2
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 1200; // (500 + 20%) * 2 + (500 + 20%) * 2

    let deposit = 2000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 17);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(2, 1700)], // limit 1000 + 1200 from phase 0.
            public_sale_weight: 455,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 1200 + 3400; // (500 + 20%) * 2 + (500 + 20%) * 2 + (1000 + 10%) * 2 + 1000 * 2

    let deposit = 2000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 20);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(3, 500)], // limit 1000 + 1200 from phase 1.
            public_sale_weight: 1546,
            refund: 0,
        }
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn specify_where_to_move_unsold_tokens_two() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.sale_amount = 100_000.into();
    config.total_sale_amount = 100_000.into();
    config.distribution_proportions.solver_allocation = 0.into();
    config.distribution_proportions.stakeholder_proportions = vec![];

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: 10,
                end_time: 12,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()),
                remaining_go_to_phase_id: Some(2),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 13,
                end_time: 15,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()),
                remaining_go_to_phase_id: Some(3),
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                start_time: 16,
                end_time: 18,
                phase_sale_limit: Some(1000.into()), // Should be moved 1200 from phase 0.
                percentage: 1000,
                ..Default::default()
            },
            DiscountPhase {
                id: 3,
                start_time: 19,
                end_time: 21,
                phase_sale_limit: Some(1000.into()), // Should be moved 1200 from phase 1.
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600)], // 2400 - (500 + 20%) * 2 = 1200 sale tokens left.
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200; // (500 + 20%) * 2

    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 14);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(1, 600)], // Left tokens from phase 0 go to the phase with id: 2
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 1200; // (500 + 20%) * 2 + (500 + 20%) * 2

    let deposit = 2000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 17);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(2, 1100)], // limit 1000 + 1200 from phase 0.
            public_sale_weight: 1000,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 1200 + 3200; // (500 + 20%) * 2 + (500 + 20%) * 2 + (1000 + 10%) * 2 + 1000 * 2

    let deposit = 2000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 20);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(3, 1100)], // limit 1000 + 1200 from phase 1.
            public_sale_weight: 1000,
            refund: 0,
        }
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn specify_where_to_move_unsold_tokens_more_complicated() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.sale_amount = 100_000.into();
    config.total_sale_amount = 100_000.into();
    config.distribution_proportions.solver_allocation = 0.into();
    config.distribution_proportions.stakeholder_proportions = vec![];

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: 10,
                end_time: 12,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()),
                remaining_go_to_phase_id: Some(2),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 13,
                end_time: 15,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()),
                remaining_go_to_phase_id: Some(2),
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                start_time: 16,
                end_time: 18,
                phase_sale_limit: Some(1000.into()), // Should be moved 1200 from phase 0 and 1.
                percentage: 1000,
                remaining_go_to_phase_id: Some(3),
                ..Default::default()
            },
            DiscountPhase {
                id: 3,
                start_time: 19,
                end_time: 21,
                phase_sale_limit: Some(1000.into()), // Should be moved 1200 from phase 2.
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(0, 600)], // 2400 - (500 + 20%) * 2 = 1200 sale tokens left.
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200; // (500 + 20%) * 2

    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 14);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(1, 600)], // Left tokens from phase 0 go to the phase with id: 2
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 1200; // (500 + 20%) * 2 + (500 + 20%) * 2

    let deposit = 1000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 17);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(2, 1100)], // limit 1000 + 1200 from phase 0.
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 1200 + 2200; // (500 + 20%) * 2 + (500 + 20%) * 2 + (1000 + 10%) * 2 + 1000 * 2

    let deposit = 2000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 20);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(3, 1100)], // limit 1000 + 1200 from phase 1.
            public_sale_weight: 1000,
            refund: 0,
        }
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn specify_where_to_move_unsold_tokens_more_complicated_two() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.sale_amount = 100_000.into();
    config.total_sale_amount = 100_000.into();
    config.distribution_proportions.solver_allocation = 0.into();
    config.distribution_proportions.stakeholder_proportions = vec![];

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 34,
                start_time: 10,
                end_time: 12,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()),
                remaining_go_to_phase_id: Some(18),
                ..Default::default()
            },
            DiscountPhase {
                id: 23,
                start_time: 13,
                end_time: 15,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()),
                remaining_go_to_phase_id: Some(18),
                ..Default::default()
            },
            DiscountPhase {
                id: 14,
                start_time: 16,
                end_time: 18,
                phase_sale_limit: Some(1100.into()),
                percentage: 1000,
                ..Default::default()
            },
            DiscountPhase {
                id: 17,
                start_time: 19,
                end_time: 21,
                phase_sale_limit: Some(1100.into()),
                percentage: 1000,
                ..Default::default()
            },
            DiscountPhase {
                id: 18,
                start_time: 22,
                end_time: 24,
                phase_sale_limit: Some(1000.into()), // Should be moved 1200 from phase 0 and 1.
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(34, 600)], // 2400 - (500 + 20%) * 2 = 1200 sale tokens left.
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200; // (500 + 20%) * 2

    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 14);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(23, 600)], // Left tokens from phase 23 go to the phase with id: 18
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 1200;

    let deposit = 200;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 17);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(14, 220)],
            public_sale_weight: 0,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 1200 + 440;

    let deposit = 1000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 20);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(17, 880)], // limit 1000 + 1200 from phase 14.
            public_sale_weight: 200,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 1200 + 440 + 1760;

    let deposit = 2000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 23);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(18, 1700)], // limit 1000 + 1200 +1200 from phase 34 and 23.
            public_sale_weight: 455,
            refund: 0,
        }
    );
}

#[test]
fn specify_where_to_move_unsold_tokens_without_limit() {
    let price = fixed_price(1, 2);
    let mut config = base_config(price);

    config.sale_amount = 100_000.into();
    config.total_sale_amount = 100_000.into();
    config.distribution_proportions.solver_allocation = 0.into();
    config.distribution_proportions.stakeholder_proportions = vec![];

    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: 10,
                end_time: 12,
                percentage: 2000,
                remaining_go_to_phase_id: Some(1),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: 13,
                end_time: 15,
                percentage: 2000,
                phase_sale_limit: Some(2400.into()),
                remaining_go_to_phase_id: Some(2),
                ..Default::default()
            },
            DiscountPhase {
                id: 2,
                start_time: 16,
                end_time: 18,
                phase_sale_limit: Some(1100.into()), // Phase 2 has its own limit of 1100.
                percentage: 1000,
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    });

    let ctx = TestContext::new(config);
    let deposit = 500;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);
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

    let deposit = 2000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 14);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(1, 1200)], // There are no tokens from phase 0 with discount since phase 0 has no limit.
            public_sale_weight: 1000,
            refund: 0,
        }
    );

    ctx.contract_mut()
        .update_discount_state(ctx.alice(), &deposit_distribution, price);
    ctx.contract_mut().total_sold_tokens = 1200 + 2400 + 2000; // (500 + 20%) * 2 + (1000 + 20%) * 2 + 1000 * 2 (public sale)

    let deposit = 1000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 17);
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![(2, 550)], // limit 1100 (500 + 10% * 2)
            public_sale_weight: 500,
            refund: 0,
        }
    );
}

/// Builds a config with a single active discount phase whose per-account minimum is set so high
/// that a tiny deposit can never satisfy it. This keeps a discount phase present (so the
/// public-sale branch of `deposit_distribution_fixed_price` is exercised) while routing the whole
/// deposit to the public sale. The remaining public-sale cap is `sale_amount - sold_before`.
fn public_sale_cap_config(price: Mechanics, sale_amount: u128) -> LaunchpadConfig {
    let mut config = base_config(price);
    config.sale_amount = sale_amount.into();
    config.total_sale_amount = sale_amount.into();
    config.distribution_proportions.solver_allocation = 0.into();
    config.distribution_proportions.stakeholder_proportions = vec![];
    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: 10,
            end_time: 15,
            percentage: 1000,
            // Larger than any sale-token amount a public-sale-sized deposit could ever buy, so the
            // discount phase is always skipped and the deposit falls through to the public sale.
            min_limit_per_account: Some((u128::from(u64::MAX)).into()),
            ..Default::default()
        }],
        public_sale_start_time: Some(10),
    });
    config
}

/// Applies `distribution` for `deposit` to a fresh investment, starting from `sold_before` already
/// sold tokens. Returns `(refund, investment_weight, total_sold_after)`, mirroring the real deposit
/// flow used by `handle_deposit`.
fn apply_deposit(
    config: &LaunchpadConfig,
    distribution: &DepositDistribution,
    deposit_amount: u128,
    sold_before: u128,
) -> (u128, u128, u128) {
    let mut investment = InvestmentAmount::default();
    let mut total_deposited = 0;
    let mut total_sold_tokens = sold_before;
    let refund = deposit(
        &mut investment,
        deposit_amount,
        &mut total_deposited,
        &mut total_sold_tokens,
        config,
        distribution,
    )
    .expect("deposit must not fail");

    (refund, investment.weight, total_sold_tokens)
}

/// Regression test for the public-sale cap rounding overflow.
///
/// With a `1 : 2` `FixedPrice` ratio and a single sale token still available, one accepted deposit
/// unit maps to two sale tokens. The buggy refund path computed the excess in sale-token units and
/// rounded its deposit equivalent *down* to zero, so the full deposit was accepted and
/// `total_sold_tokens` jumped to 102 against a `sale_amount` of 101. The fix caps the accepted
/// weight to the remaining capacity (rounding down), so the deposit is refunded and the cap holds.
#[test]
fn public_sale_cap_rounding_can_oversell_by_one_sale_token() {
    // 1 deposit token buys 2 sale tokens.
    let price = fixed_price(1, 2);
    let config = public_sale_cap_config(price, 101);

    let ctx = TestContext::new(config.clone());
    // 100 of 101 sale tokens already sold: only a single sale token remains.
    ctx.contract_mut().total_sold_tokens = 100;

    let deposit_amount = 1;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit_amount, 11);

    // The single remaining sale token cannot be bought with one deposit unit (which maps to two
    // sale tokens), so nothing is accepted for the public sale and the deposit is fully refunded.
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![],
            public_sale_weight: 0,
            refund: deposit_amount,
        }
    );

    let (refund, weight, total_sold_after) =
        apply_deposit(&config, &deposit_distribution, deposit_amount, 100);

    assert_eq!(refund, deposit_amount);
    assert_eq!(weight, 0);
    assert_eq!(total_sold_after, 100);
    assert!(total_sold_after <= config.sale_amount.0);
}

/// Regression test proving the overflow is not dust-bounded: with a `1 : 1e18` `FixedPrice` ratio
/// and `sale_amount = 1`, the buggy path accepted a single deposit unit and recorded
/// `investment.weight = total_sold_tokens = 1e18`. After the fix, the deposit is refunded and no
/// sale-token weight is recorded.
#[test]
fn public_sale_cap_rounding_oversell_scales_to_price_granularity_after_refund() {
    // 1 deposit token buys 1e18 sale tokens.
    let price = fixed_price(1, 10u128.pow(18));
    let config = public_sale_cap_config(price, 1);

    let ctx = TestContext::new(config.clone());
    // No tokens sold yet: the whole `sale_amount` of 1 token remains.
    ctx.contract_mut().total_sold_tokens = 0;

    let deposit_amount = 1;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit_amount, 11);

    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![],
            public_sale_weight: 0,
            refund: deposit_amount,
        }
    );

    let (refund, weight, total_sold_after) =
        apply_deposit(&config, &deposit_distribution, deposit_amount, 0);

    assert_eq!(refund, deposit_amount);
    assert_eq!(weight, 0);
    assert_eq!(total_sold_after, 0);
    assert!(total_sold_after <= config.sale_amount.0);
}

/// Control case: when the remaining cap aligns with the sale-token granularity (`1 : 1` ratio),
/// the public-sale path consumes exactly the remaining capacity and ends precisely at
/// `sale_amount`. This guards against overcorrecting the refund path.
#[test]
fn public_sale_cap_rounding_control_when_cap_matches_sale_token_granularity() {
    // 1 deposit token buys exactly 1 sale token.
    let price = fixed_price(1, 1);
    let config = public_sale_cap_config(price, 101);

    let ctx = TestContext::new(config.clone());
    // 100 of 101 sale tokens already sold: a single sale token remains.
    ctx.contract_mut().total_sold_tokens = 100;

    let deposit_amount = 1;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit_amount, 11);

    // One deposit unit buys exactly the one remaining sale token, with no refund.
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![],
            public_sale_weight: 1,
            refund: 0,
        }
    );

    let (refund, weight, total_sold_after) =
        apply_deposit(&config, &deposit_distribution, deposit_amount, 100);

    assert_eq!(refund, 0);
    assert_eq!(weight, 1);
    assert_eq!(total_sold_after, 101);
    assert_eq!(total_sold_after, config.sale_amount.0);
}

/// Config with an *active* discount phase (entered, not skipped) and no public sale, so a deposit
/// overshooting the remaining global cap is handled inside the discount-phase branch of
/// `deposit_distribution_fixed_price`.
fn entered_discount_phase_cap_config(price: Mechanics, sale_amount: u128) -> LaunchpadConfig {
    let mut config = base_config(price);
    config.sale_amount = sale_amount.into();
    config.total_sale_amount = sale_amount.into();
    config.distribution_proportions.solver_allocation = 0.into();
    config.distribution_proportions.stakeholder_proportions = vec![];
    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: 10,
            end_time: 15,
            percentage: 1000,
            ..Default::default()
        }],
        // Public sale starts in the future, so at t=11 the deposit can only enter the discount phase.
        public_sale_start_time: Some(100),
    });
    config
}

/// Cap-rounding is not limited to the `required_deposit == 0` case the MEDIUM-1 guard covers: a
/// discount phase capped to a single remaining sale token at a `7 : 3` price computes a phase
/// weight of 2 (`required_deposit > 0`, so the phase is recorded), yet that weight buys
/// `floor(2 * 3 / 7) = 0` sale tokens. The buyer must be refunded in full, never charged for zero
/// sale tokens, and the global cap must hold.
#[test]
fn discount_phase_cap_dust_does_not_charge_for_zero_sale_tokens() {
    let price = fixed_price(7, 3);
    let config = entered_discount_phase_cap_config(price, 100);

    let ctx = TestContext::new(config.clone());
    // 99 of 100 sale tokens already sold: a single sale token remains.
    ctx.contract_mut().total_sold_tokens = 99;

    let deposit_amount = 7;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit_amount, 11);

    let (refund, weight, total_sold_after) =
        apply_deposit(&config, &deposit_distribution, deposit_amount, 99);

    assert_eq!(weight, 0);
    assert_eq!(refund, deposit_amount);
    assert_eq!(total_sold_after, 99);
    assert!(total_sold_after <= config.sale_amount.0);
}

/// The same sub-grain dust on the public-sale path: at a `7 : 3` price the cap-limited public-sale
/// weight is 2, which also buys `floor(2 * 3 / 7) = 0` sale tokens. The deposit must be refunded in
/// full rather than charged for zero sale tokens.
#[test]
fn public_sale_cap_dust_does_not_charge_for_zero_sale_tokens() {
    let price = fixed_price(7, 3);
    let config = public_sale_cap_config(price, 100);

    let ctx = TestContext::new(config.clone());
    // 99 of 100 sale tokens already sold: a single sale token remains.
    ctx.contract_mut().total_sold_tokens = 99;

    let deposit_amount = 7;
    let deposit_distribution =
        ctx.contract()
            .get_deposit_distribution(ctx.alice(), deposit_amount, 11);

    let (refund, weight, total_sold_after) =
        apply_deposit(&config, &deposit_distribution, deposit_amount, 99);

    assert_eq!(weight, 0);
    assert_eq!(refund, deposit_amount);
    assert_eq!(total_sold_after, 99);
    assert!(total_sold_after <= config.sale_amount.0);
}

/// Regression for the per-account minimum must be enforced against the amount
/// actually credited *after* caps, not the uncapped discounted amount. Here the discounted 2200
/// clears the 2000 minimum, but only 1500 sale tokens remain, so the deposit is capped to 1500
/// (< 2000). The phase must therefore be skipped and the deposit routed to the public sale, never
/// recorded in the phase below the minimum.
#[test]
fn min_limit_enforced_against_capped_amount() {
    let mut config = base_config(fixed_price(1, 2));
    config.sale_amount = 10_000.into();
    config.total_sale_amount = 10_000.into();
    config.distribution_proportions.solver_allocation = 0.into();
    config.distribution_proportions.stakeholder_proportions = vec![];
    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: 10,
            end_time: 15,
            percentage: 1000,
            min_limit_per_account: Some(2000.into()),
            ..Default::default()
        }],
        public_sale_start_time: Some(10),
    });

    let ctx = TestContext::new(config);
    // Only 1500 of 10_000 sale tokens remain, so the discounted 2200 is capped to 1500 < 2000.
    ctx.contract_mut().total_sold_tokens = 8_500;

    let deposit = 1000;
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), deposit, 11);

    // Capped below the minimum → skipped, routed to the public sale, which then caps to the
    // remaining 1500 sale tokens (= 750 weight) and refunds the rest.
    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![],
            public_sale_weight: 750,
            refund: 250,
        }
    );
}

/// Companion to `min_limit_enforced_against_capped_amount` for a lossy price ratio: the per-account
/// minimum must be checked against the round-tripped accepted amount, not the pre-conversion cap.
/// At `7:3`, capping to 2000 sale tokens converts to a weight of `floor(2000 * 7 / 3) = 4666` that
/// credits only `floor(4666 * 3 / 7) = 1999` sale tokens — below a 2000 minimum — so the phase must
/// be skipped rather than recording a sub-minimum position.
#[test]
fn min_limit_enforced_against_round_tripped_amount() {
    let mut config = base_config(fixed_price(7, 3));
    config.sale_amount = 2000.into();
    config.total_sale_amount = 2000.into();
    config.distribution_proportions.solver_allocation = 0.into();
    config.distribution_proportions.stakeholder_proportions = vec![];
    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: 10,
            end_time: 20,
            percentage: 0,
            min_limit_per_account: Some(2000.into()),
            ..Default::default()
        }],
        public_sale_start_time: Some(20),
    });

    let ctx = TestContext::new(config);
    // 7000 deposit buys 3000 sale tokens uncapped, capped to the 2000 remaining supply; the cap
    // round-trips to only 1999 credited sale tokens (< 2000 min), so the phase is skipped and — with
    // the public sale not yet open — the whole deposit is refunded.
    let deposit_distribution = ctx
        .contract()
        .get_deposit_distribution(ctx.alice(), 7000, 11);

    assert_eq!(
        deposit_distribution,
        DepositDistribution::WithDiscount {
            phase_weights: vec![],
            public_sale_weight: 0,
            refund: 7000,
        }
    );
}
