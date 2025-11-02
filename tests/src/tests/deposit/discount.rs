use aurora_launchpad_types::config::Mechanics;
use aurora_launchpad_types::discount::{DiscountParams, DiscountPhase};

use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
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
        lp.get_available_for_claim(bob.id()).await.unwrap(),
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
