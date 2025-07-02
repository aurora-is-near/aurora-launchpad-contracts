use aurora_launchpad_types::config::Mechanics;
use aurora_launchpad_types::discount::Discount;

use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::mt_token::MultiToken;
use crate::env::sale_contract::{Deposit, SaleContract};
use crate::tests::NANOSECONDS_PER_SECOND;

#[tokio::test]
async fn deposit_without_init() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config_nep245().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.deposit_token
        .storage_deposit(env.defuse.id())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 200_000.into(), alice.id().as_str())
        .await
        .unwrap();

    let result = alice
        .deposit_nep245(
            lp.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            100_000.into(),
        )
        .await;
    assert!(result.is_err()); // Because the Launchpad has the wrong status.

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();

    // The balance must be the same since the sale contract was not initialized.
    assert_eq!(balance, 200_000.into());
}

#[tokio::test]
async fn successful_deposits() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config_nep245().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposit(env.defuse.id())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 100_000.into(), alice.id().as_str())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 200_000.into(), bob.id().as_str())
        .await
        .unwrap();

    alice
        .deposit_nep245(
            lp.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            100_000.into(),
        )
        .await
        .unwrap();
    bob.deposit_nep245(
        lp.id(),
        env.defuse.id(),
        env.deposit_token.id().as_str(),
        100_000.into(),
    )
    .await
    .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );
}

#[tokio::test]
async fn successful_deposits_with_refund() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config_nep245().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposit(env.defuse.id())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 300_000.into(), alice.id().as_str())
        .await
        .unwrap();

    alice
        .deposit_nep245(
            lp.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            300_000.into(),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into()); // 100_000 was refunded because the total sale amount is 200_000

    assert_eq!(lp.get_participants_count().await.unwrap(), 1);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(200_000.into())
    );
}

#[tokio::test]
async fn successful_deposits_fixed_price_with_discount_and_refund() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config_nep245().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;

    // Add a discount to the configuration
    config.discounts.push(Discount {
        start_date: config.start_date,
        end_date: config.end_date,
        percentage: 2000, // 20% discount
    });

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposit(env.defuse.id())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 200_000.into(), alice.id().as_str())
        .await
        .unwrap();

    alice
        .deposit_nep245(
            lp.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            190_000.into(),
        )
        .await
        .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 33_333.into()); // 23_333 was refunded because the total sale amount is 200_000

    assert_eq!(lp.get_participants_count().await.unwrap(), 1);
    assert_eq!(lp.get_total_deposited().await.unwrap().0, 190_000 - 23_333);
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some((190_000 - 23_333).into())
    );

    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        200_000.into()
    );
}

#[tokio::test]
async fn successful_deposits_price_discovery() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config_nep245().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;
    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposit(env.defuse.id())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 100_000.into(), alice.id().as_str())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 200_000.into(), bob.id().as_str())
        .await
        .unwrap();

    alice
        .deposit_nep245(
            lp.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            70_000.into(),
        )
        .await
        .unwrap();
    bob.deposit_nep245(
        lp.id(),
        env.defuse.id(),
        env.deposit_token.id().as_str(),
        170_000.into(),
    )
    .await
    .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 30_000.into());
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 30_000.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 240_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(70_000.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(170_000.into())
    );

    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        58_333.into()
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
        (200_000 - 58333 - 1).into()
    );
}

#[tokio::test]
async fn successful_deposits_price_discovery_with_discount_and_without_discount() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config_nep245().await;

    config.end_date = config.start_date + 40 * NANOSECONDS_PER_SECOND;
    config.mechanics = Mechanics::PriceDiscovery;

    // Add a discount to the configuration
    let discount_end = config.start_date + 20 * NANOSECONDS_PER_SECOND;
    config.discounts.push(Discount {
        start_date: config.start_date,
        end_date: discount_end,
        percentage: 2000, // 20% discount
    });

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposit(env.defuse.id())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 100_000.into(), alice.id().as_str())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 200_000.into(), bob.id().as_str())
        .await
        .unwrap();

    // Alice deposits with a 20% discount
    alice
        .deposit_nep245(
            lp.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            70_000.into(),
        )
        .await
        .unwrap();

    // Wait for the discount period to end
    env.wait_for_timestamp(discount_end + 10 * NANOSECONDS_PER_SECOND)
        .await;
    // Bob deposits 170_000 without a discount
    bob.deposit_nep245(
        lp.id(),
        env.defuse.id(),
        env.deposit_token.id().as_str(),
        170_000.into(),
    )
    .await
    .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 30_000.into());
    let balance = env
        .defuse
        .mt_balance_of(bob.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 30_000.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 240_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(70_000.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(170_000.into())
    );

    assert_eq!(
        lp.get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        66_141.into()
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id().as_str()).await.unwrap(),
        (200_000 - 66_141 - 1).into()
    );
}

#[tokio::test]
async fn deposits_for_status_not_ongoing() {
    let env = create_env().await.unwrap();
    let config = env.create_config_nep245().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposit(env.defuse.id())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 100_000.into(), alice.id().as_str())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 200_000.into(), bob.id().as_str())
        .await
        .unwrap();

    alice
        .deposit_nep245(
            lp.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            100_000.into(),
        )
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap().as_str(), "Failed");

    let res = bob
        .deposit_nep245(
            lp.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            100_000.into(),
        )
        .await;
    assert!(format!("{res:?}").contains("Smart contract panicked: Launchpad is not ongoing"));

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 200_000.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 1);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 100_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );
    assert_eq!(lp.get_investments(bob.id().as_str()).await.unwrap(), None);
}

#[tokio::test]
async fn deposits_check_status_success() {
    let env = create_env().await.unwrap();
    let config = env.create_config_nep245().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposit(env.defuse.id())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 100_000.into(), alice.id().as_str())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer_call(env.defuse.id(), 200_000.into(), bob.id().as_str())
        .await
        .unwrap();

    alice
        .deposit_nep245(
            lp.id(),
            env.defuse.id(),
            env.deposit_token.id().as_str(),
            100_000.into(),
        )
        .await
        .unwrap();
    bob.deposit_nep245(
        lp.id(),
        env.defuse.id(),
        env.deposit_token.id().as_str(),
        100_000.into(),
    )
    .await
    .unwrap();

    let balance = env
        .defuse
        .mt_balance_of(alice.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 0.into());

    let balance = env
        .defuse
        .mt_balance_of(bob.id(), env.deposit_token.id().as_str())
        .await
        .unwrap();
    assert_eq!(balance, 100_000.into());

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000.into());
    assert_eq!(
        lp.get_investments(alice.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );
    assert_eq!(
        lp.get_investments(bob.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap().as_str(), "Success");
}
