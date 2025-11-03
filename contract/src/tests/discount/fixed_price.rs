use aurora_launchpad_types::InvestmentAmount;
use aurora_launchpad_types::discount::{DepositDistribution, DiscountParams, DiscountPhase};
use near_sdk::test_utils::test_env::alice;

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
