use aurora_launchpad_types::config::{DepositToken, Mechanics};
use aurora_launchpad_types::discount::Discount;

use crate::env::Env;
use crate::env::fungible_token::FungibleToken;
use crate::env::sale_contract::{Deposit, SaleContract};
use crate::tests::NANOSECONDS_PER_SECOND;

#[tokio::test]
async fn deposit_without_init() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();

    let status = lp.get_status().await.unwrap();
    assert_eq!(status, "NotInitialized");

    let result = alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap_err();
    assert!(result.to_string().contains("Launchpad is not ongoing"));

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    // The balance must be the same since the sale contract was not initialized.
    assert_eq!(balance, 100_000);
}

#[tokio::test]
async fn successful_deposits() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(100_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(100_000));
}

#[tokio::test]
async fn successful_deposits_with_refund() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 300_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 300_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 100_000); // 100_000 was refunded because the total sale amount is 200_000

    assert_eq!(lp.get_participants_count().await.unwrap(), 1);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(200_000));
}

#[tokio::test]
async fn deposit_wrong_token() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;
    // Set a wrong deposit token to test the error handling.
    config.deposit_token = DepositToken::Nep141("wrong_token.near".parse().unwrap());

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 300_000)
        .await
        .unwrap();

    let result = alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 300_000)
        .await
        .unwrap_err();
    assert!(result.to_string().contains("Unsupported NEP-141 token"));

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 300_000); // All tokens should be refunded since the deposit token is wrong.

    assert_eq!(lp.get_participants_count().await.unwrap(), 0);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 0);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), None);
}

#[tokio::test]
async fn successful_deposits_fixed_price_with_discount_and_refund() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;

    // Add a discount to the configuration
    config.discounts.push(Discount {
        start_date: config.start_date,
        end_date: config.end_date,
        percentage: 2000, // 20% discount
    });

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 200_000)
        .await
        .unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 190_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 33_333); // 23_333 was refunded because the total sale amount is 200_000

    assert_eq!(lp.get_participants_count().await.unwrap(), 1);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 190_000 - 23_333);
    assert_eq!(
        lp.get_investments(alice.id()).await.unwrap(),
        Some(190_000 - 23_333)
    );

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        200_000
    );
}

#[tokio::test]
async fn successful_deposits_price_discovery() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

    config.end_date = config.start_date + 200 * NANOSECONDS_PER_SECOND;
    config.mechanics = Mechanics::PriceDiscovery;

    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 70_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 170_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 30_000);
    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 30_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 240_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(70_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(170_000));

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        58_333
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id()).await.unwrap(),
        200_000 - 58333 - 1
    );
}

#[tokio::test]
async fn successful_deposits_price_discovery_with_discount_and_without_discount() {
    let env = Env::new().await.unwrap();
    let mut config = env.create_config().await;

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
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    // Alice deposits with a 20% discount
    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 70_000)
        .await
        .unwrap();

    // Wait for the discount period to end
    env.wait_for_timestamp(discount_end + 10 * NANOSECONDS_PER_SECOND)
        .await;
    // Bob deposits 170_000 without a discount
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 170_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 30_000);
    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 30_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 240_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(70_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(170_000));

    assert_eq!(
        lp.get_available_for_claim(alice.id()).await.unwrap(),
        66_141
    );
    assert_eq!(
        lp.get_available_for_claim(bob.id()).await.unwrap(),
        200_000 - 66_141 - 1
    );
}

#[tokio::test]
async fn deposits_for_status_not_ongoing() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    env.wait_for_sale_finish(&config).await;

    assert_eq!(lp.get_status().await.unwrap(), "Failed");
    // NOTE: it's special to check `is_failed` method. Do not remove it.
    let is_failed: bool = lp.is_failed().await.unwrap();
    assert!(is_failed);

    let res = bob
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await;
    assert!(format!("{res:?}").contains("Smart contract panicked: Launchpad is not ongoing"));

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 200_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 1);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 100_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(100_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), None);
}

#[tokio::test]
async fn deposits_check_status_success() {
    let env = Env::new().await.unwrap();
    let config = env.create_config().await;
    let lp = env.create_launchpad(&config).await.unwrap();
    let alice = env.alice();
    let bob = env.bob();

    env.sale_token.storage_deposit(lp.id()).await.unwrap();
    env.sale_token
        .ft_transfer_call(lp.id(), config.total_sale_amount, "")
        .await
        .unwrap();

    env.deposit_ft
        .storage_deposits(&[lp.id(), alice.id(), bob.id()])
        .await
        .unwrap();
    env.deposit_ft
        .ft_transfer(alice.id(), 100_000)
        .await
        .unwrap();
    env.deposit_ft.ft_transfer(bob.id(), 200_000).await.unwrap();

    // NOTE: it's special to check `is_ongoing` method. Do not remove it.
    let is_ongoing: bool = lp.is_ongoing().await.unwrap();
    assert!(is_ongoing);

    alice
        .deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();
    bob.deposit_nep141(lp.id(), env.deposit_ft.id(), 100_000)
        .await
        .unwrap();

    let balance = env.deposit_ft.ft_balance_of(alice.id()).await.unwrap();
    assert_eq!(balance, 0);

    let balance = env.deposit_ft.ft_balance_of(bob.id()).await.unwrap();
    assert_eq!(balance, 100_000);

    assert_eq!(lp.get_participants_count().await.unwrap(), 2);
    assert_eq!(lp.get_total_deposited().await.unwrap(), 200_000);
    assert_eq!(lp.get_investments(alice.id()).await.unwrap(), Some(100_000));
    assert_eq!(lp.get_investments(bob.id()).await.unwrap(), Some(100_000));

    env.wait_for_sale_finish(&config).await;
    assert_eq!(lp.get_status().await.unwrap(), "Success");
    // NOTE: it's special to check `is_success` method. Do not remove it.
    let is_success: bool = lp.view("is_success").await.unwrap().json().unwrap();
    assert!(is_success);
}
