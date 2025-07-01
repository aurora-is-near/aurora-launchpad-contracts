use crate::env::create_env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::{Deposit, SaleContract};
use aurora_launchpad_types::config::{DepositToken, Mechanics};
use aurora_launchpad_types::discount::Discount;

#[tokio::test]
async fn deposit_without_init() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.deposit_token
        .storage_deposits(&[launchpad.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();

    let status = launchpad.get_status().await.unwrap();
    assert_eq!(status, "NotStarted");

    alice
        .deposit_nep141(launchpad.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    // The balance must be the same since the sale contract was not initialized.
    assert_eq!(balance, 100_000.into());
}

#[tokio::test]
async fn successful_deposits() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[launchpad.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(launchpad.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(launchpad.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000.into());

    assert_eq!(launchpad.get_participants_count().await.unwrap(), 2);
    assert_eq!(
        launchpad.get_total_deposited().await.unwrap(),
        200_000.into()
    );
    assert_eq!(
        launchpad
            .get_investments(alice.id().as_str())
            .await
            .unwrap(),
        Some(100_000.into())
    );
    assert_eq!(
        launchpad.get_investments(bob.id().as_str()).await.unwrap(),
        Some(100_000.into())
    );
}

#[tokio::test]
async fn successful_deposits_with_refund() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[launchpad.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 300_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(launchpad.id(), env.deposit_token.id(), 300_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000.into()); // 100_000 was refunded because the total sale amount is 200_000

    assert_eq!(launchpad.get_participants_count().await.unwrap(), 1);
    assert_eq!(
        launchpad.get_total_deposited().await.unwrap(),
        200_000.into()
    );
    assert_eq!(
        launchpad
            .get_investments(alice.id().as_str())
            .await
            .unwrap(),
        Some(200_000.into())
    );
}

#[tokio::test]
async fn deposit_wrong_token() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);
    // Set a wrong deposit token to test the error handling.
    config.deposit_token = DepositToken::Nep141("wrong_token.near".parse().unwrap());

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[launchpad.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 300_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(launchpad.id(), env.deposit_token.id(), 300_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 300_000.into()); // All tokens should be refunded since the deposit token is wrong.

    assert_eq!(launchpad.get_participants_count().await.unwrap(), 0);
    assert_eq!(launchpad.get_total_deposited().await.unwrap(), 0.into());
    assert_eq!(
        launchpad
            .get_investments(alice.id().as_str())
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn successful_deposits_fixed_price_with_discount_and_refund() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);

    // Add a discount to the configuration
    config.discounts.push(Discount {
        start_date: config.start_date,
        end_date: config.end_date,
        percentage: 2000, // 20% discount
    });

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[launchpad.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(launchpad.id(), env.deposit_token.id(), 190_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 33_333.into()); // 23_333 was refunded because the total sale amount is 200_000

    assert_eq!(launchpad.get_participants_count().await.unwrap(), 1);
    assert_eq!(
        launchpad.get_total_deposited().await.unwrap().0,
        190_000 - 23_333
    );
    assert_eq!(
        launchpad
            .get_investments(alice.id().as_str())
            .await
            .unwrap(),
        Some((190_000 - 23_333).into())
    );

    assert_eq!(
        launchpad
            .get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        200_000.into()
    );
}

#[tokio::test]
async fn successful_deposits_price_discovery() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 200 * 10u64.pow(9);
    config.mechanics = Mechanics::PriceDiscovery;

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[launchpad.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(launchpad.id(), env.deposit_token.id(), 70_000.into())
        .await
        .unwrap();
    bob.deposit_nep141(launchpad.id(), env.deposit_token.id(), 170_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 30_000.into());
    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 30_000.into());

    assert_eq!(launchpad.get_participants_count().await.unwrap(), 2);
    assert_eq!(
        launchpad.get_total_deposited().await.unwrap(),
        240_000.into()
    );
    assert_eq!(
        launchpad
            .get_investments(alice.id().as_str())
            .await
            .unwrap(),
        Some(70_000.into())
    );
    assert_eq!(
        launchpad.get_investments(bob.id().as_str()).await.unwrap(),
        Some(170_000.into())
    );

    assert_eq!(
        launchpad
            .get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        58_333.into()
    );
    assert_eq!(
        launchpad
            .get_available_for_claim(bob.id().as_str())
            .await
            .unwrap(),
        (200_000 - 58333 - 1).into()
    );
}

#[tokio::test]
async fn successful_deposits_price_discovery_with_discount_and_without_discount() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 15 * 10u64.pow(9);
    config.mechanics = Mechanics::PriceDiscovery;

    // Add a discount to the configuration
    let discount_end = now + 10u64.pow(9);
    config.discounts.push(Discount {
        start_date: config.start_date,
        end_date: discount_end,
        percentage: 2000, // 20% discount
    });

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[launchpad.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    // Alice deposits with a 20% discount
    alice
        .deposit_nep141(launchpad.id(), env.deposit_token.id(), 70_000.into())
        .await
        .unwrap();

    // Wait for the discount period to end
    env.wait_for_timestamp(discount_end + 1000).await;
    // Bob deposits 170_000 without a discount
    bob.deposit_nep141(launchpad.id(), env.deposit_token.id(), 170_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 30_000.into());
    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 30_000.into());

    assert_eq!(launchpad.get_participants_count().await.unwrap(), 2);
    assert_eq!(
        launchpad.get_total_deposited().await.unwrap(),
        240_000.into()
    );
    assert_eq!(
        launchpad
            .get_investments(alice.id().as_str())
            .await
            .unwrap(),
        Some(70_000.into())
    );
    assert_eq!(
        launchpad.get_investments(bob.id().as_str()).await.unwrap(),
        Some(170_000.into())
    );

    assert_eq!(
        launchpad
            .get_available_for_claim(alice.id().as_str())
            .await
            .unwrap(),
        58_333.into()
    );
    assert_eq!(
        launchpad
            .get_available_for_claim(bob.id().as_str())
            .await
            .unwrap(),
        (200_000 - 58333 - 1).into()
    );
}

#[tokio::test]
async fn deposits_for_status_not_ongoing() {
    let env = create_env().await.unwrap();
    let mut config = env.create_config();
    let now = env.worker.view_block().await.unwrap().timestamp();

    config.start_date = now;
    config.end_date = now + 15 * 10u64.pow(9);

    let launchpad = env.create_launchpad(&config).await.unwrap();
    let alice = env.create_participant("alice").await.unwrap();
    let bob = env.create_participant("bob").await.unwrap();

    env.sale_token
        .storage_deposit(launchpad.id())
        .await
        .unwrap();
    env.sale_token
        .ft_transfer_call(launchpad.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_token
        .storage_deposits(&[launchpad.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(alice.id(), 100_000.into())
        .await
        .unwrap();
    env.deposit_token
        .ft_transfer(bob.id(), 200_000.into())
        .await
        .unwrap();

    alice
        .deposit_nep141(launchpad.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    assert_eq!(launchpad.get_status().await.unwrap().as_str(), "Failed");

    bob.deposit_nep141(launchpad.id(), env.deposit_token.id(), 100_000.into())
        .await
        .unwrap();

    let balance = env.deposit_token.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0.into());

    let balance = env.deposit_token.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 200_000.into());

    assert_eq!(launchpad.get_participants_count().await.unwrap(), 1);
    assert_eq!(
        launchpad.get_total_deposited().await.unwrap(),
        100_000.into()
    );
    assert_eq!(
        launchpad
            .get_investments(alice.id().as_str())
            .await
            .unwrap(),
        Some(100_000.into())
    );
    assert_eq!(
        launchpad.get_investments(bob.id().as_str()).await.unwrap(),
        None
    );
}
