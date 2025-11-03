use aurora_launchpad_types::config::Mechanics;
use aurora_launchpad_types::discount::{DiscountParams, DiscountPhase};

use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Deposit, SaleContract, WhiteListManage};
use crate::tests::NANOSECONDS_PER_SECOND;

#[tokio::test]
async fn deposits_for_different_discount_phases() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let now = env.current_timestamp().await;
    config.start_date = now;
    let duration = 20 * NANOSECONDS_PER_SECOND;
    let midpoint = config.start_date + duration / 2;
    config.end_date = now + duration;
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: config.start_date,
                end_time: midpoint,
                percentage: 2000, // 20% discount
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: midpoint,
                end_time: config.end_date,
                percentage: 1000, // 10% discount
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    });
    config.soft_cap = 50_000.into();
    config.sale_amount = 1_000_000.into();
    config.total_sale_amount = config.sale_amount;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 30_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 30_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 10_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 10_000)
        .await
        .unwrap();

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        24_000
    );
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 24_000);

    env.wait_for_timestamp(midpoint).await;

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 20_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 20_000)
        .await
        .unwrap();

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        24_000 + 44_000 // 10_000 + 20% * 2 + 20_000 + 10% * 2
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id()).await.unwrap(),
        24_000 + 44_000
    );
}

#[tokio::test]
async fn deposits_for_different_discount_phases_with_whitelist() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let admin = env.john();
    let now = env.current_timestamp().await;
    config.start_date = now;
    let duration = 20 * NANOSECONDS_PER_SECOND;
    config.end_date = now + duration;
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 2000, // 20% discount
                whitelist: Some(std::iter::once(alice.id().into()).collect()),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 1000, // 10% discount
                ..Default::default()
            },
        ],
        public_sale_start_time: None,
    });
    config.soft_cap = 50_000.into();
    config.sale_amount = 1_000_000.into();
    config.total_sale_amount = config.sale_amount;

    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();

    let whitelist = lp.get_whitelist_for_discount_phase(0).await.unwrap();
    assert_eq!(whitelist, Some(vec![alice.id().into()]));

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 30_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 30_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 10_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 10_000)
        .await
        .unwrap();

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        24_000 // 10_000 + 20% * 2
    );
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 22_000); // 10_000 + 10% * 2, bob skips the first phase

    // add bob to the whitelist of the first phase, now he can buy tokens with 20% discount
    admin
        .extend_whitelist_for_discount_phase(lp.id(), vec![bob.id().into()], 0)
        .await
        .unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 10_000)
        .await
        .unwrap();
    assert_eq!(
        lp.get_available_for_claim(bob.id()).await.unwrap(),
        22_000 + 24_000
    );

    // remove alice from the whitelist of the first phase, now she can buy tokens with 10% discount only
    admin
        .remove_from_whitelist_for_discount_phase(lp.id(), vec![alice.id().into()], 0)
        .await
        .unwrap();
    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 10_000)
        .await
        .unwrap();
    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        24_000 + 22_000
    );

    // activate a whitelist for the second phase, alice can buy tokens from the public sale now
    admin
        .extend_whitelist_for_discount_phase(lp.id(), vec![bob.id().into()], 1)
        .await
        .unwrap();
    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 10_000)
        .await
        .unwrap();

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        24_000 + 22_000 + 20_000
    );
}

#[tokio::test]
async fn deposits_with_moving_left_tokens() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let admin = env.john();
    let phase_duration = 10 * NANOSECONDS_PER_SECOND;

    config.start_date = env.current_timestamp().await;
    config.end_date = config.start_date + phase_duration * 5;
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    let (phase_periods, _) = (0..5).fold(
        (
            vec![],
            (config.start_date, config.start_date + phase_duration),
        ),
        |(mut periods, (mut start, mut end)), _| {
            periods.push((start, end));
            start += phase_duration;
            end += phase_duration;
            (periods, (start, end))
        },
    );
    let phases = vec![
        DiscountPhase {
            id: 0,
            start_time: phase_periods[0].0,
            end_time: phase_periods[0].1,
            percentage: 2000,                    // 20% discount
            phase_sale_limit: Some(4800.into()), // 2000 deposit tokens = (2000 + 20%) * 2 sale tokens
            remaining_go_to_phase_id: Some(1),
            ..Default::default()
        },
        DiscountPhase {
            id: 1, // moved 2400 from the first phase
            start_time: phase_periods[1].0,
            end_time: phase_periods[1].1,
            percentage: 2000,                    // 20% discount
            phase_sale_limit: Some(2400.into()), // 1000 deposit tokens = (1000 + 20%) * 2 sale tokens
            remaining_go_to_phase_id: Some(4),
            ..Default::default()
        },
        DiscountPhase {
            id: 2,
            start_time: phase_periods[2].0,
            end_time: phase_periods[2].1,
            percentage: 1000,                    // 10% discount
            phase_sale_limit: Some(2200.into()), // 1000 deposit tokens = (1000 + 10%) * 2 sale tokens
            ..Default::default()
        },
        DiscountPhase {
            id: 3, // 1100 tokens should move from the third phase
            start_time: phase_periods[3].0,
            end_time: phase_periods[3].1,
            percentage: 1000,                    // 10% discount
            phase_sale_limit: Some(1100.into()), // 500 deposit tokens = (500 + 10%) * 2 sale tokens
            ..Default::default()
        },
        DiscountPhase {
            id: 4, // the limit should be 4400 including moving from the above phases (1100 + 2400)
            start_time: phase_periods[4].0,
            end_time: phase_periods[4].1,
            percentage: 1000,                   // 10% discount
            phase_sale_limit: Some(900.into()), // 2000 deposit tokens = (2000 + 10%) * 2 = 4400
            ..Default::default()
        },
    ];
    config.discounts = Some(DiscountParams {
        phases: phases.clone(),
        public_sale_start_time: None,
    });
    config.soft_cap = 10_000.into();
    config.sale_amount = 20_000.into();
    config.total_sale_amount = config.sale_amount;

    let lp = env
        .create_launchpad_with_admin(&config, Some(admin.id()))
        .await
        .unwrap();

    env.sale_token
        .storage_deposits(&[lp.id(), env.defuse.id()])
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 30_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 30_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 600)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 400)
        .await
        .unwrap();

    assert_eq!(
        tokio::try_join!(
            lp.get_available_for_claim(alice.id()),
            lp.get_available_for_claim(bob.id())
        )
        .unwrap(),
        (1440, 960) // (600 + 20% * 2, 400 + 20% * 2) = 2400
    );

    env.wait_for_timestamp(phase_periods[0].1).await; // wait for the first phase to end

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 600)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 400)
        .await
        .unwrap();

    assert_eq!(
        tokio::try_join!(
            lp.get_available_for_claim(alice.id()),
            lp.get_available_for_claim(bob.id())
        )
        .unwrap(),
        (1440 * 2, 960 * 2) // (600 + 20% * 2, 400 + 20% * 2) * 2 = 4800
    );

    env.wait_for_timestamp(phase_periods[1].1).await; // wait for the second phase to end

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 300)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 200)
        .await
        .unwrap();

    assert_eq!(
        tokio::try_join!(
            lp.get_available_for_claim(alice.id()),
            lp.get_available_for_claim(bob.id())
        )
        .unwrap(),
        (1440 * 2 + 660, 960 * 2 + 440)
    );

    env.wait_for_timestamp(phase_periods[2].1).await; // wait for the third phase to end

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 300)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 200)
        .await
        .unwrap();

    assert_eq!(
        tokio::try_join!(
            lp.get_available_for_claim(alice.id()),
            lp.get_available_for_claim(bob.id())
        )
        .unwrap(),
        (1440 * 2 + 660 * 2, 960 * 2 + 440 * 2)
    );

    env.wait_for_timestamp(phase_periods[3].1).await; // wait for the fourth phase to end

    // 4400 tokens left in the fifth phase, 900 - limit + 2400 from the second phase and 1100 from the fourth phase
    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 1000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 1000)
        .await
        .unwrap();

    assert_eq!(
        tokio::try_join!(
            lp.get_available_for_claim(alice.id()),
            lp.get_available_for_claim(bob.id())
        )
        .unwrap(),
        (1440 * 2 + 660 * 2 + 2200, 960 * 2 + 440 * 2 + 2200)
    );

    // Check that alice and bob bought all available tokens in the discount phases.
    assert_eq!(
        phases
            .iter()
            .map(|p| p.phase_sale_limit.map_or(0, |v| v.0))
            .sum::<u128>(),
        1440 * 2 + 660 * 2 + 2200 + 960 * 2 + 440 * 2 + 2200
    );

    // There are no limits left, the following deposits should be moved to the public sale without a discount.
    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 1000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 1000)
        .await
        .unwrap();

    assert_eq!(
        tokio::try_join!(
            lp.get_available_for_claim(alice.id()),
            lp.get_available_for_claim(bob.id())
        )
        .unwrap(),
        (
            1440 * 2 + 660 * 2 + 2200 + 2000,
            960 * 2 + 440 * 2 + 2200 + 2000
        )
    );

    // 15_400 tokens have been bought, 20000-15400 = 4600 tokens left for the public sale
    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 2000) // alice buys 4000
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 2000) // but for bob 600 sale tokens left
        .await
        .unwrap();

    assert_eq!(
        tokio::try_join!(
            lp.get_available_for_claim(alice.id()),
            lp.get_available_for_claim(bob.id())
        )
        .unwrap(),
        (
            1440 * 2 + 660 * 2 + 2200 + 2000 + 4000,
            960 * 2 + 440 * 2 + 2200 + 2000 + 600
        )
    );

    // Check that 1700 deposit tokens have been refunded to the bob's intent account
    assert_eq!(
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
            .await
            .unwrap(),
        1700
    );
}

// not whitelisted user deposits before public sale: refund all to the intent account
#[tokio::test]
async fn unwhitelisted_user_wants_to_buy_before_public_sale_starts() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let now = env.current_timestamp().await;
    config.start_date = now;
    let duration = 20 * NANOSECONDS_PER_SECOND;
    config.end_date = now + duration;
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: config.start_date,
            end_time: config.end_date,
            percentage: 2000, // 20% discount
            whitelist: Some(std::iter::once(alice.id().into()).collect()),
            ..Default::default()
        }],
        public_sale_start_time: Some(now + duration / 2),
    });
    config.soft_cap = 50_000.into();
    config.sale_amount = 1_000_000.into();
    config.total_sale_amount = config.sale_amount;

    let lp = env.create_launchpad(&config).await.unwrap();

    let whitelist = lp.get_whitelist_for_discount_phase(0).await.unwrap();
    assert_eq!(whitelist, Some(vec![alice.id().into()]));

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 10_000).await.unwrap();

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 10_000)
        .await
        .unwrap();

    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);

    // Check that deposit tokens have been refunded to the bob's intent account
    assert_eq!(
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
            .await
            .unwrap(),
        10_000
    );
}

// two whitelisted users deposit before the public sale starts, and bob reaches the phase limit:
// refund unused bob's tokens to his intent account
#[tokio::test]
async fn user_reaches_phase_total_limit() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let now = env.current_timestamp().await;
    config.start_date = now;
    let duration = 20 * NANOSECONDS_PER_SECOND;
    config.end_date = now + duration;
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: config.start_date,
            end_time: config.end_date,
            percentage: 2000, // 20% discount
            phase_sale_limit: Some(1200.into()),
            whitelist: Some([alice.id().into(), bob.id().into()].into_iter().collect()),
            ..Default::default()
        }],
        public_sale_start_time: Some(now + duration / 2),
    });
    config.soft_cap = 50_000.into();
    config.sale_amount = 1_000_000.into();
    config.total_sale_amount = config.sale_amount;

    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 1000).await.unwrap();
    env.deposit_ft.ft_transfer(alice.id(), 1000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 375)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 375)
        .await
        .unwrap();

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 900);
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 300); // phase limit 1200, spent 125 tokens

    // Check that deposit tokens have been refunded to the bob's intent account
    assert_eq!(
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
            .await
            .unwrap(),
        250
    );
}

#[tokio::test]
async fn user_does_not_reach_min_account_limit_and_refund_over_max_limit() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let now = env.current_timestamp().await;
    config.start_date = now;
    let duration = 30 * NANOSECONDS_PER_SECOND;
    config.end_date = now + duration;
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    config.discounts = Some(DiscountParams {
        phases: vec![DiscountPhase {
            id: 0,
            start_time: config.start_date,
            end_time: config.end_date,
            percentage: 2000, // 20% discount
            min_limit_per_account: Some(250.into()),
            max_limit_per_account: Some(600.into()), // (250 + 20%) * 2
            ..Default::default()
        }],
        public_sale_start_time: Some(config.end_date),
    });
    config.soft_cap = 50_000.into();
    config.sale_amount = 1_000_000.into();
    config.total_sale_amount = config.sale_amount;

    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 1000).await.unwrap();
    env.deposit_ft.ft_transfer(alice.id(), 1000).await.unwrap();

    // alice buys 720 tokens
    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 300) // 300 = 720 sale tokens with 20% discount
        .await
        .unwrap();

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 600);
    // Check that deposit tokens have been refunded to the bob's intent account
    assert_eq!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
            .await
            .unwrap(),
        50
    );

    // bob tries to deposit less than min_limit_per_account
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100) // 100 = 240 sale tokens with 20% discount
        .await
        .unwrap();

    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 0);
    // Check that deposit tokens have been refunded to the bob's intent account
    assert_eq!(
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
            .await
            .unwrap(),
        100
    );

    // bob tries to deposit a bit more than min_limit_per_account costs
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 105) // 105 = 252 sale tokens with 20% discount
        .await
        .unwrap();
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 252);

    // check that the second deposit could be less than min_limit_per_account
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100) // 100 = 240 sale tokens with 20% discount
        .await
        .unwrap();
    assert_eq!(
        lp.get_available_for_claim(bob.id()).await.unwrap(),
        252 + 240
    );

    // bob buys 600 tokens
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100) // but 108 sale tokens left for 45 deposit tokens
        .await
        .unwrap();

    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 600);
    assert_eq!(
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
            .await
            .unwrap(),
        100 + 55
    );
}

#[tokio::test]
async fn overlapped_phases_with_phase_limit_without_public_sale() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let now = env.current_timestamp().await;
    config.start_date = now;
    let duration = 30 * NANOSECONDS_PER_SECOND;
    config.end_date = now + duration;
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 2000, // 20% discount
                max_limit_per_account: Some(2400.into()),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 1000, // 10% discount
                max_limit_per_account: Some(2200.into()),
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(config.end_date),
    });
    config.soft_cap = 50_000.into();
    config.sale_amount = 1_000_000.into();
    config.total_sale_amount = config.sale_amount;

    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 3000).await.unwrap();
    env.deposit_ft.ft_transfer(alice.id(), 3000).await.unwrap();

    // alice attempts to buy tokens for 3000 deposit tokens, but it's possible to spend by 1000 per phase:
    // 1000 + 20% * 2 = 2400 (max_limit_per_account phase1)
    // 1000 + 10% * 2 = 2200 (max_limit_per_account phase2)
    // 1000 should be refunded to the alice's intent account
    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 3000) // 300 = 720 sale tokens with 20% discount
        .await
        .unwrap();

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 4600);
    // Check that deposit tokens have been refunded to the bob's intent account
    assert_eq!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
            .await
            .unwrap(),
        1000
    );

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 3000) // 100 = 240 sale tokens with 20% discount
        .await
        .unwrap();

    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 4600);
    // Check that deposit tokens have been refunded to the bob's intent account
    assert_eq!(
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
            .await
            .unwrap(),
        1000
    );
}

#[tokio::test]
async fn overlapped_phases_with_phase_limit_with_public_sale() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let now = env.current_timestamp().await;
    config.start_date = now;
    let duration = 30 * NANOSECONDS_PER_SECOND;
    let midpoint = now + duration / 2;
    config.end_date = now + duration;
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 2000, // 20% discount
                max_limit_per_account: Some(2400.into()),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 1000, // 10% discount
                max_limit_per_account: Some(2200.into()),
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(midpoint),
    });
    config.soft_cap = 50_000.into();
    config.sale_amount = 1_000_000.into();
    config.total_sale_amount = config.sale_amount;

    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 3000).await.unwrap();
    env.deposit_ft.ft_transfer(alice.id(), 3000).await.unwrap();

    // alice attempts to buy tokens for 3000 deposit tokens, but it's possible to spend by 1000 per phase:
    // 1000 + 20% * 2 = 2400 (max_limit_per_account phase1)
    // 1000 + 10% * 2 = 2200 (max_limit_per_account phase2)
    // 1000 should be refunded to the alice's intent account
    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 3000) // 300 = 720 sale tokens with 20% discount
        .await
        .unwrap();

    assert_eq!(lp.get_available_for_claim(alice.id()).await.unwrap(), 4600);
    // Check that deposit tokens have been refunded to the bob's intent account
    assert_eq!(
        env.defuse
            .mt_balance_of(alice.id(), format!("nep141:{}", env.deposit_ft.id()))
            .await
            .unwrap(),
        1000
    );

    // bob waits for the public sale to start
    env.wait_for_timestamp(midpoint).await;

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 3000) // 100 = 240 sale tokens with 20% discount
        .await
        .unwrap();

    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 6600);
    // Check that there is no refund for bob
    assert_eq!(
        env.defuse
            .mt_balance_of(bob.id(), format!("nep141:{}", env.deposit_ft.id()))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn overlapped_phases_with_whitelists_users_in_different_phases_and_public_sale() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let now = env.current_timestamp().await;
    config.start_date = now;
    let duration = 30 * NANOSECONDS_PER_SECOND;
    let midpoint = now + duration / 2;
    config.end_date = now + duration;
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 2000,                    // 20% discount
                phase_sale_limit: Some(2400.into()), // 1000 deposit tokens = 2400 sale tokens
                whitelist: Some([alice.id().into()].into_iter().collect()),
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 1000,                    // 10% discount
                phase_sale_limit: Some(1100.into()), // 500 deposit tokens = 1100 sale tokens
                whitelist: Some([bob.id().into()].into_iter().collect()),
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(midpoint),
    });
    config.sale_amount = 1_000_000.into();
    config.total_sale_amount = config.sale_amount;

    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(alice.id(), 2000).await.unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 2000).await.unwrap();

    env.wait_for_timestamp(midpoint).await;

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 1200)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 700)
        .await
        .unwrap();

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        2400 + 400
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id()).await.unwrap(),
        1100 + 400
    );
}

#[tokio::test]
async fn overlapped_phases_no_whitelist_with_public_sale_pass_all_phases_with_deposit() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let now = env.current_timestamp().await;
    config.start_date = now;
    let duration = 30 * NANOSECONDS_PER_SECOND;
    let midpoint = now + duration / 2;
    config.end_date = now + duration;
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 2000,                    // 20% discount
                phase_sale_limit: Some(2400.into()), // 1000 deposit tokens
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 1000,                    // 10% discount
                phase_sale_limit: Some(1100.into()), // 500 deposit tokens
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(midpoint),
    });
    config.sale_amount = 1_000_000.into();
    config.total_sale_amount = config.sale_amount;

    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(alice.id(), 3000).await.unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 3000).await.unwrap();

    env.wait_for_timestamp(midpoint).await;

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 2000)
        .await
        .unwrap();
    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        2400 + 1100 + 1000 // Phase1 limit + Phase2 limit + public sale
    );

    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 500)
        .await
        .unwrap();
    assert_eq!(lp.get_available_for_claim(bob.id()).await.unwrap(), 1000);
}

#[tokio::test]
async fn overlapped_phases_with_max_limit_per_account_pass_all_phases_for_deposit() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    let alice = env.alice();
    let bob = env.bob();
    let now = env.current_timestamp().await;
    config.start_date = now;
    let duration = 30 * NANOSECONDS_PER_SECOND;
    let midpoint = now + duration / 2;
    config.end_date = now + duration;
    config.mechanics = Mechanics::FixedPrice {
        deposit_token: 1.into(),
        sale_token: 2.into(),
    };
    config.discounts = Some(DiscountParams {
        phases: vec![
            DiscountPhase {
                id: 0,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 2000,                        // 20% discount
                max_limit_per_account: Some(240.into()), // requires 100 deposit
                ..Default::default()
            },
            DiscountPhase {
                id: 1,
                start_time: config.start_date,
                end_time: config.end_date,
                percentage: 1000,                        // 10% discount
                max_limit_per_account: Some(220.into()), // requires 100 deposit
                ..Default::default()
            },
        ],
        public_sale_start_time: Some(midpoint),
    });
    config.sale_amount = 1_000_000.into();
    config.total_sale_amount = config.sale_amount;

    let lp = env.create_launchpad(&config).await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id(), env.defuse.id()])
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(alice.id(), 2000).await.unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 2000).await.unwrap();

    env.wait_for_timestamp(midpoint).await;

    let (alice_claim, bob_claim) = tokio::try_join!(
        async {
            alice
                .deposit_nep141(lp.id(), env.deposit_ft.id(), 1000)
                .await?;
            lp.get_available_for_claim(alice.id()).await
        },
        async {
            bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 500)
                .await?;
            lp.get_available_for_claim(bob.id()).await
        }
    )
    .unwrap();

    assert_eq!(alice_claim, 240 + 220 + 1600);
    assert_eq!(bob_claim, 240 + 220 + 600);
}
